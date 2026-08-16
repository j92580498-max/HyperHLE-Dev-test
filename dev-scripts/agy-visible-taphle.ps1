[CmdletBinding()]
param(
    [ValidateSet('Launch', 'Status', 'Focus', 'Click', 'Capture', 'Close',
        'Uninstall', 'BrokerLaunch', 'BrokerFocus', 'BrokerClick', 'BrokerClose')]
    [string]$Action = 'Status',
    [string]$AppPath,
    [int]$X,
    [int]$Y,
    [ValidateRange(1, 60)][int]$TimeoutSeconds = 15
)

$ErrorActionPreference = 'Stop'
$launchTask = 'tapHLE-AGY-Visible-Launch'
$inputTask = 'tapHLE-AGY-Visible-Input'
$repo = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot '..')).Path
$script = $MyInvocation.MyCommand.Path
$exe = Join-Path $repo 'target\release\tapHLE.exe'
$state = Join-Path $env:LOCALAPPDATA 'tapHLE\agy-visible'
$pidFile = Join-Path $state 'taphle.pid'
$statusFile = Join-Path $state 'taphle.status.json'
$stdoutFile = Join-Path $state 'taphle.stdout.log'
$stderrFile = Join-Path $state 'taphle.stderr.log'
$requestFile = Join-Path $state 'frame.request'
$frameFile = Join-Path $state 'frame.ppm'
$pngFile = Join-Path $state 'frame.png'

function Initialize-State {
    if (-not (Test-Path -LiteralPath $state -PathType Container)) {
        [void](New-Item -ItemType Directory -Path $state)
    }
}

function Quote-Argument([string]$Value) {
    return '"' + $Value.Replace('"', '\"') + '"'
}

function Register-InteractiveTask(
    [string]$Name,
    [Microsoft.Management.Infrastructure.CimInstance]$TaskAction
) {
    $user = [System.Security.Principal.WindowsIdentity]::GetCurrent().Name
    $principal = New-ScheduledTaskPrincipal `
        -UserId $user -LogonType Interactive -RunLevel Limited
    $settings = New-ScheduledTaskSettingsSet `
        -ExecutionTimeLimit ([TimeSpan]::Zero) `
        -AllowStartIfOnBatteries -DontStopIfGoingOnBatteries
    $trigger = New-ScheduledTaskTrigger -Once -At ([DateTime]::Now.AddYears(10))
    Register-ScheduledTask -TaskName $Name -Action $TaskAction `
        -Trigger $trigger -Principal $principal -Settings $settings `
        -Description 'tapHLE interactive desktop bridge for Google Antigravity CLI.' `
        -Force | Out-Null
}

function Get-BrokerProcess {
    if (-not (Test-Path -LiteralPath $pidFile -PathType Leaf)) {
        throw 'No broker PID is recorded. Run -Action Launch first.'
    }
    $id = [int](Get-Content -Raw -LiteralPath $pidFile)
    $process = Get-Process -Id $id -ErrorAction SilentlyContinue
    if ($null -eq $process -or $process.ProcessName -ne 'tapHLE') {
        throw "Recorded tapHLE process $id is not running."
    }
    return $process
}

function Add-WindowInterop {
    if ('TapHLEAgyInterop' -as [type]) { return }
    Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;
public static class TapHLEAgyInterop {
    [StructLayout(LayoutKind.Sequential)]
    public struct POINT { public int X; public int Y; }
    [DllImport("user32.dll")] public static extern bool ClientToScreen(IntPtr h, ref POINT p);
    [DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y);
    [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
    [DllImport("user32.dll")] public static extern bool ShowWindow(IntPtr h, int command);
    [DllImport("user32.dll")] public static extern void mouse_event(
        uint flags, uint dx, uint dy, uint data, UIntPtr extra);
    [DllImport("user32.dll")] public static extern bool PostMessage(
        IntPtr h, uint message, IntPtr wParam, IntPtr lParam);
}
'@
}

function Get-Window {
    $process = Get-BrokerProcess
    $deadline = [DateTime]::UtcNow.AddSeconds($TimeoutSeconds)
    do {
        $process.Refresh()
        if ($process.MainWindowHandle -ne [IntPtr]::Zero) {
            return $process.MainWindowHandle
        }
        Start-Sleep -Milliseconds 100
    } while ([DateTime]::UtcNow -lt $deadline)
    throw "tapHLE process $($process.Id) has no top-level window."
}

function Focus-Window {
    Add-WindowInterop
    $window = Get-Window
    [void][TapHLEAgyInterop]::ShowWindow($window, 9)
    [void][TapHLEAgyInterop]::SetForegroundWindow($window)
    Start-Sleep -Milliseconds 250
}

function Invoke-InputTask([string]$BrokerAction, [string[]]$Extra) {
    $arguments = @('-NoProfile', '-ExecutionPolicy', 'Bypass', '-File',
        (Quote-Argument $script), '-Action', $BrokerAction) + $Extra
    $taskAction = New-ScheduledTaskAction -Execute 'powershell.exe' `
        -Argument ($arguments -join ' ') -WorkingDirectory $repo
    Register-InteractiveTask -Name $inputTask -TaskAction $taskAction
    Start-ScheduledTask -TaskName $inputTask
    $deadline = [DateTime]::UtcNow.AddSeconds($TimeoutSeconds)
    do {
        $task = Get-ScheduledTask -TaskName $inputTask
        if ($task.State -ne 'Running') {
            $result = (Get-ScheduledTaskInfo -TaskName $inputTask).LastTaskResult
            if ($result -ne 0) { throw "Input task failed with result $result." }
            return
        }
        Start-Sleep -Milliseconds 100
    } while ([DateTime]::UtcNow -lt $deadline)
    throw "Timed out waiting for $inputTask."
}

function Test-CompletePpm([string]$Path) {
    if (-not (Test-Path -LiteralPath $Path -PathType Leaf)) { return $false }
    $bytes = [IO.File]::ReadAllBytes($Path)
    if ($bytes.Length -lt 16) { return $false }
    $text = [Text.Encoding]::ASCII.GetString(
        $bytes, 0, [Math]::Min($bytes.Length, 128))
    $match = [regex]::Match($text, '^P6\s+(\d+)\s+(\d+)\s+255\s')
    if (-not $match.Success) { return $false }
    $expected = $match.Length +
        3 * [int64]$match.Groups[1].Value * [int64]$match.Groups[2].Value
    return $bytes.Length -eq $expected
}

switch ($Action) {
    'BrokerLaunch' {
        Initialize-State
        $env:TAPHLE_FRAME_CAPTURE_REQUEST = $requestFile
        $env:TAPHLE_FRAME_CAPTURE_OUTPUT = $frameFile
        $startArguments = @{
            FilePath = $exe
            WorkingDirectory = $repo
            RedirectStandardOutput = $stdoutFile
            RedirectStandardError = $stderrFile
            PassThru = $true
        }
        if ($AppPath) {
            $startArguments.ArgumentList = @((Quote-Argument $AppPath))
        }
        $process = Start-Process @startArguments
        Set-Content -LiteralPath $pidFile -Value $process.Id -NoNewline
        $deadline = [DateTime]::UtcNow.AddSeconds($TimeoutSeconds)
        do {
            $process.Refresh()
            if ($process.MainWindowHandle -ne [IntPtr]::Zero) {
                [pscustomobject]@{
                    ProcessId = $process.Id
                    SessionId = $process.SessionId
                    WindowHandle = $process.MainWindowHandle.ToInt64()
                    WindowTitle = $process.MainWindowTitle
                    RecordedAt = [DateTime]::UtcNow.ToString('o')
                } | ConvertTo-Json | Set-Content -LiteralPath $statusFile
                break
            }
            Start-Sleep -Milliseconds 100
        } while ([DateTime]::UtcNow -lt $deadline)
        if (-not (Test-Path -LiteralPath $statusFile -PathType Leaf)) {
            Stop-Process -Id $process.Id -ErrorAction SilentlyContinue
            throw 'tapHLE did not create a window on the interactive desktop in time.'
        }
        $process.WaitForExit()
        exit $process.ExitCode
    }
    'BrokerFocus' {
        Focus-Window
        exit 0
    }
    'BrokerClick' {
        Add-WindowInterop
        $window = Get-Window
        Focus-Window
        $point = [TapHLEAgyInterop+POINT]::new()
        $point.X = $X
        $point.Y = $Y
        if (-not [TapHLEAgyInterop]::ClientToScreen($window, [ref]$point)) {
            throw 'ClientToScreen failed.'
        }
        if (-not [TapHLEAgyInterop]::SetCursorPos($point.X, $point.Y)) {
            throw 'SetCursorPos failed.'
        }
        Start-Sleep -Milliseconds 150
        [TapHLEAgyInterop]::mouse_event(0x0002, 0, 0, 0, [UIntPtr]::Zero)
        Start-Sleep -Milliseconds 75
        [TapHLEAgyInterop]::mouse_event(0x0004, 0, 0, 0, [UIntPtr]::Zero)
        exit 0
    }
    'BrokerClose' {
        Add-WindowInterop
        if (-not [TapHLEAgyInterop]::PostMessage(
            (Get-Window), 0x0010, [IntPtr]::Zero, [IntPtr]::Zero)) {
            throw 'Posting WM_CLOSE failed.'
        }
        exit 0
    }
    'Launch' {
        Initialize-State
        if (-not (Test-Path -LiteralPath $exe -PathType Leaf)) {
            throw "Release executable not found: $exe"
        }
        if (Get-Process -Name tapHLE -ErrorAction SilentlyContinue) {
            throw 'A tapHLE process is already running. Close it first.'
        }
        foreach ($path in @($pidFile, $statusFile, $stdoutFile, $stderrFile,
                $requestFile, $frameFile, $pngFile)) {
            if (Test-Path -LiteralPath $path) {
                Remove-Item -LiteralPath $path -Force
            }
        }
        $arguments = @('-NoProfile', '-ExecutionPolicy', 'Bypass', '-File',
            (Quote-Argument $script), '-Action', 'BrokerLaunch')
        if ($AppPath) {
            $resolved = (Resolve-Path -LiteralPath $AppPath).Path
            $arguments += @('-AppPath', (Quote-Argument $resolved))
        }
        $taskAction = New-ScheduledTaskAction -Execute 'powershell.exe' `
            -Argument ($arguments -join ' ') -WorkingDirectory $repo
        Register-InteractiveTask -Name $launchTask -TaskAction $taskAction
        Start-ScheduledTask -TaskName $launchTask
        $deadline = [DateTime]::UtcNow.AddSeconds($TimeoutSeconds)
        do {
            if (Test-Path -LiteralPath $statusFile -PathType Leaf) {
                $status = Get-Content -Raw -LiteralPath $statusFile |
                    ConvertFrom-Json
                [void](Get-BrokerProcess)
                $status | Add-Member -NotePropertyName StateDirectory `
                    -NotePropertyValue $state -PassThru
                return
            }
            Start-Sleep -Milliseconds 100
        } while ([DateTime]::UtcNow -lt $deadline)
        throw 'tapHLE did not create a visible window in time.'
    }
    'Status' {
        [void](Get-BrokerProcess)
        if (-not (Test-Path -LiteralPath $statusFile -PathType Leaf)) {
            throw 'The interactive broker did not record window status.'
        }
        $status = Get-Content -Raw -LiteralPath $statusFile | ConvertFrom-Json
        if ([int64]$status.WindowHandle -eq 0) {
            throw 'The interactive broker recorded a zero window handle.'
        }
        $status | Add-Member -NotePropertyName StateDirectory `
            -NotePropertyValue $state -PassThru
    }
    'Focus' {
        Invoke-InputTask -BrokerAction 'BrokerFocus' -Extra @()
        'Focused the broker-launched tapHLE window.'
    }
    'Click' {
        Invoke-InputTask -BrokerAction 'BrokerClick' `
            -Extra @('-X', $X, '-Y', $Y)
        "Clicked tapHLE client coordinate ($X, $Y)."
    }
    'Capture' {
        [void](Get-BrokerProcess)
        foreach ($path in @($frameFile, $pngFile, $requestFile)) {
            if (Test-Path -LiteralPath $path) {
                Remove-Item -LiteralPath $path -Force
            }
        }
        $marker = [IO.File]::Open($requestFile, [IO.FileMode]::CreateNew,
            [IO.FileAccess]::Write, [IO.FileShare]::None)
        $marker.Dispose()
        $deadline = [DateTime]::UtcNow.AddSeconds($TimeoutSeconds)
        do {
            if (Test-CompletePpm -Path $frameFile) {
                $ffmpeg = Get-Command ffmpeg -ErrorAction SilentlyContinue
                if ($null -eq $ffmpeg) {
                    throw 'ffmpeg is required to convert the captured PPM to an inspectable PNG.'
                }
                & $ffmpeg.Source -v error -y -i $frameFile -vf vflip $pngFile
                if ($LASTEXITCODE -ne 0 -or
                    -not (Test-Path -LiteralPath $pngFile -PathType Leaf)) {
                    throw 'ffmpeg failed to convert the captured frame.'
                }
                $pngFile
                return
            }
            Start-Sleep -Milliseconds 100
        } while ([DateTime]::UtcNow -lt $deadline)
        throw "Timed out waiting for frame capture: $frameFile"
    }
    'Close' {
        Invoke-InputTask -BrokerAction 'BrokerClose' -Extra @()
        'Posted WM_CLOSE to the broker-launched tapHLE window.'
    }
    'Uninstall' {
        foreach ($name in @($launchTask, $inputTask,
                'tapHLE-AGY-Visible-Launcher')) {
            if (Get-ScheduledTask -TaskName $name -ErrorAction SilentlyContinue) {
                Unregister-ScheduledTask -TaskName $name -Confirm:$false
            }
        }
        'Removed the tapHLE AGY interactive tasks.'
    }
}
