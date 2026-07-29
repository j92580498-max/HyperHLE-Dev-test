# Launch every app in the local collection, wait, and check it is still alive
# and still drawing.
#
# This exists because the check kept being rewritten from memory each session
# and kept coming out smaller than the one before. dev-docs/app-debugging-
# playbook.md, "A liveness check is not a regression check", records what went
# wrong when it did: SPY mouse HD aborted at about forty seconds and a
# twenty-second check called it healthy, and JellyCar 2 was not in the list at
# all and had been dead for a dozen commits before anyone launched it.
#
# So the two rules that shaped this script are:
#
#   Every app, not a convenient subset. There is no list here to fall out of;
#   whatever is in the collection directory is swept. An app absent from the
#   sweep is an app whose rating is a claim about the past.
#
#   Frames compared by hash, sampled until they differ. A single frame cannot
#   tell a running game from one frozen on a plausible screenshot, and that
#   distinction is the one that has caught the most. Sampling rather than
#   comparing one fixed pair matters too: a slowly changing title screen reads
#   as still if the two frames are close together.
#
# Per-app launch options are not configured here. tapHLE reads them from
# tapHLE_default_options.txt by bundle identifier, so an app that needs
# --landscape-native or --force-composition gets it without this script
# knowing anything about that app.
#
#   .\dev-scripts\regression-sweep.ps1
#   .\dev-scripts\regression-sweep.ps1 -Only warlords -Settle 60
#
# Exit code is 0 when every app started and stayed up, 1 otherwise.
#
# What this does NOT do: drive each app to the milestone it is rated for.
# Three stars means a gameplay loop, and nothing here leaves the title screen,
# so a break that only shows in play is still invisible to it. The click maps
# in dev-docs/app-notes/ are what a check that goes that far would use.

[CmdletBinding()]
param(
    [string]$AppsDir,
    [int]$Settle = 45,
    [int]$Gap = 8,
    [int]$Frames = 3,
    [int]$CaptureTimeout = 20,
    [string]$Only
)

$ErrorActionPreference = 'Stop'
$repo = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot '..')).Path
$exe = Join-Path $repo 'target\release\tapHLE.exe'
if (-not (Test-Path -LiteralPath $exe -PathType Leaf)) {
    throw "Release executable not found: $exe. Run: cargo build --release"
}
if (-not $AppsDir) { $AppsDir = Join-Path $repo 'tapHLE_apps' }
if (-not (Test-Path -LiteralPath $AppsDir -PathType Container)) {
    throw "App collection directory not found: $AppsDir"
}

$work = Join-Path $env:TEMP 'taphle-regression-sweep'
New-Item -ItemType Directory -Force -Path $work | Out-Null

# Apps already known to fail, so that a real regression is not lost among them.
# A missing entry only means the app is reported normally, which is why this is
# a denylist and not the list of what to run.
$knownBad = @{}
$knownBadFile = Join-Path $PSScriptRoot 'regression-sweep-known-bad.txt'
if (Test-Path -LiteralPath $knownBadFile) {
    foreach ($line in Get-Content -LiteralPath $knownBadFile -Encoding UTF8) {
        $trimmed = $line.Trim()
        if (-not $trimmed -or $trimmed.StartsWith('#')) { continue }
        $split = $trimmed.IndexOf(':')
        if ($split -lt 1) { continue }
        $knownBad[$trimmed.Substring(0, $split).Trim()] =
            $trimmed.Substring($split + 1).Trim()
    }
}

function Get-KnownBadReason([string]$Name) {
    foreach ($key in $knownBad.Keys) {
        if ($Name -like "*$key*") { return $knownBad[$key] }
    }
    return $null
}

# A listed failure is still printed, with its verdict, so the run says what
# happened rather than hiding it. Only the exit code treats it as expected.
function Get-Label([string]$Verdict, [string]$Reason) {
    if ($Reason) { return "expected:$Verdict".PadRight(11) }
    return $Verdict.PadRight(11)
}

# A frame is only written once the compositor sees the marker file, and the
# emulator re-arms itself afterwards, so each capture creates its own marker
# and takes the output away before asking for the next one.
function Wait-ForFrame([string]$Output, [int]$Seconds) {
    $deadline = [DateTime]::UtcNow.AddSeconds($Seconds)
    do {
        if (Test-Path -LiteralPath $Output -PathType Leaf) {
            $bytes = [IO.File]::ReadAllBytes($Output)
            if ($bytes.Length -ge 16) {
                $head = [Text.Encoding]::ASCII.GetString(
                    $bytes, 0, [Math]::Min($bytes.Length, 128))
                $m = [regex]::Match($head, '^P6\s+(\d+)\s+(\d+)\s+255\s')
                # A partial file would hash differently every time and read as
                # motion, so the frame is only accepted once it is all there.
                if ($m.Success) {
                    $expected = $m.Length +
                        3 * [int64]$m.Groups[1].Value * [int64]$m.Groups[2].Value
                    if ($bytes.Length -eq $expected) {
                        return (Get-FileHash -LiteralPath $Output `
                            -Algorithm SHA256).Hash.Substring(0, 12)
                    }
                }
            }
        }
        Start-Sleep -Milliseconds 200
    } while ([DateTime]::UtcNow -lt $deadline)
    return $null
}

# Why it stopped is the whole value of a failure line, and it is what a bisect
# starts from. A panic goes to stderr, so both streams are worth reading.
function Write-Reason($Process, [string]$Out, [string]$Err) {
    $Process.WaitForExit()
    "            exit=$($Process.ExitCode)"
    foreach ($pair in @(@('stderr', $Err), @('stdout', $Out))) {
        if (-not (Test-Path -LiteralPath $pair[1])) { continue }
        # tapHLE writes UTF-8; without saying so, Get-Content decodes as the
        # system codepage and mangles any non-ASCII in the message.
        $lines = @(Get-Content -LiteralPath $pair[1] -Tail 4 -Encoding UTF8 |
            Where-Object { $_.Trim() })
        foreach ($line in $lines) { "            $($pair[0]): $line" }
    }
}

function Get-Frame([string]$Marker, [string]$Output, [int]$Seconds) {
    foreach ($p in @($Marker, $Output)) {
        if (Test-Path -LiteralPath $p) { Remove-Item -LiteralPath $p -Force }
    }
    New-Item -ItemType File -Path $Marker | Out-Null
    $hash = Wait-ForFrame -Output $Output -Seconds $Seconds
    foreach ($p in @($Marker, $Output)) {
        if (Test-Path -LiteralPath $p) { Remove-Item -LiteralPath $p -Force }
    }
    return $hash
}

$candidates = @(Get-ChildItem -LiteralPath $AppsDir |
    Where-Object { $_.Extension -eq '.ipa' -or $_.Extension -eq '.app' } |
    Sort-Object Name)
if ($Only) {
    $candidates = @($candidates | Where-Object { $_.Name -like "*$Only*" })
}
if ($candidates.Count -eq 0) { throw "No apps to sweep in $AppsDir" }

"Sweeping $($candidates.Count) app(s) from $AppsDir"
""

$failures = 0
$static = 0
$expected = 0
$fixed = 0

foreach ($app in $candidates) {
    $reason = Get-KnownBadReason -Name $app.Name
    $failed = $false
    # The file name is the app's own; only a sanitised stem is used for paths.
    $slug = ($app.BaseName -replace '[^A-Za-z0-9]+', '-').Trim('-')
    if ($slug.Length -gt 40) { $slug = $slug.Substring(0, 40) }
    $marker = Join-Path $work "$slug.request"
    $frame = Join-Path $work "$slug.ppm"
    $out = Join-Path $work "$slug.stdout.log"
    $err = Join-Path $work "$slug.stderr.log"

    $env:TAPHLE_FRAME_CAPTURE_REQUEST = $marker
    $env:TAPHLE_FRAME_CAPTURE_OUTPUT = $frame
    $process = $null
    try {
        $process = Start-Process -FilePath $exe -WorkingDirectory $repo `
            -ArgumentList @("`"$($app.FullName)`"") `
            -RedirectStandardOutput $out -RedirectStandardError $err -PassThru
        # Touching the handle makes .NET cache it, which is what makes
        # ExitCode readable later. Without this it comes back empty.
        $null = $process.Handle

        Start-Sleep -Seconds $Settle
        if ($process.HasExited) {
            $failed = $true
            "$(Get-Label 'DIED-EARLY' $reason)  $($app.BaseName)"
            if (-not $reason) {
                Write-Reason -Process $process -Out $out -Err $err
            }
            continue
        }

        # Sample until two frames differ rather than comparing a fixed pair.
        # Warlords HD's title screen changes over about fifteen seconds, so a
        # single five-second gap reported it as still - a false STATIC is
        # exactly the kind of quietly wrong answer this script exists to stop.
        # A busy app settles it on the second frame and pays nothing extra.
        $first = Get-Frame -Marker $marker -Output $frame -Seconds $CaptureTimeout
        $second = $first
        for ($i = 2; $i -le $Frames -and $second -eq $first; $i++) {
            Start-Sleep -Seconds $Gap
            $second = Get-Frame -Marker $marker -Output $frame `
                -Seconds $CaptureTimeout
        }

        if ($process.HasExited) {
            $failed = $true
            "$(Get-Label 'DIED-LATE' $reason)  $($app.BaseName) (alive at ${Settle}s)"
            if (-not $reason) {
                Write-Reason -Process $process -Out $out -Err $err
            }
            continue
        }
        if (-not $first -or -not $second) {
            $failed = $true
            "$(Get-Label 'NO-FRAME' $reason)  $($app.BaseName) (alive, no frame)"
            continue
        }
        if ($first -eq $second) {
            # Not a failure on its own: a title screen is legitimately still.
            # It is a failure for anything rated for gameplay, which is why it
            # is counted and reported at the end.
            "STATIC      $($app.BaseName) ($first)"
            $static++
        } else {
            "MOVING      $($app.BaseName) ($first -> $second)"
        }
    } catch {
        $failed = $true
        "$(Get-Label 'ERROR' $reason)  $($app.BaseName): $($_.Exception.Message)"
    } finally {
        if ($failed) {
            if ($reason) { $expected++ } else { $failures++ }
        } elseif ($reason) {
            # A listed app that works is a line to delete, and saying so is the
            # only thing that stops this file rotting into unread history.
            "FIXED       $($app.BaseName) now passes. Remove it from"
            "            dev-scripts/regression-sweep-known-bad.txt ($reason)"
            $fixed++
        }
        if ($process -and -not $process.HasExited) {
            Stop-Process -Id $process.Id -Force -ErrorAction SilentlyContinue
        }
        Remove-Item Env:\TAPHLE_FRAME_CAPTURE_REQUEST -ErrorAction SilentlyContinue
        Remove-Item Env:\TAPHLE_FRAME_CAPTURE_OUTPUT -ErrorAction SilentlyContinue
        Start-Sleep -Seconds 2
    }
}

""
"$($candidates.Count) swept, $failures failed, $expected expected-fail, " +
    "$fixed newly passing, $static static."
"Logs and frames: $work"
if ($static -gt 0) {
    "A still frame is expected on a title screen and is a regression for"
    "anything rated for gameplay. This sweep cannot tell the two apart,"
    "because it does not drive any app past its title screen."
}
if ($failures -gt 0) { exit 1 }
exit 0
