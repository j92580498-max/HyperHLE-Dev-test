# Efficient Windows app troubleshooting

This is the operational playbook for advancing a selected game without
repeating settled work. Read it with `AGENTS.md`, `agent-workflow.md`, and the
app's sanitized note under `dev-docs/app-notes/` when one exists.

## Resume from evidence, not from launch

Before building or running anything:

1. Check the current branch, worktree, recent commits, and remote tracking
   state.
2. Read the app's compatibility record and continuation note.
3. Identify the highest milestone already reproduced on a committed build.
4. Write down the one next observable boundary. Examples are “menu tap reaches
   level select,” “the queue starts,” or “the first frame presents.”
5. Choose the cheapest observation that distinguishes the live hypotheses.

Do not replay the whole investigation merely to become familiar with it. Start
from the last trustworthy milestone and test only the next frontier unless a
code change could have regressed an earlier boundary.

Keep three small lists while working:

- **Proven:** directly supported by a hash-verified run, static inspection, or
  a deterministic test.
- **Rejected:** hypotheses contradicted by evidence, including why.
- **Next discriminator:** one bounded trace, test, or input that decides what
  to implement next.

If work crosses agents or sessions, update the app note before stopping. The
note is a continuation aid, not a compatibility claim.

## Keep app debugging visible

Run compatibility targets in a normal, visible tapHLE window by default. The
maintainer should be able to watch startup, rendering, orientation changes,
automated input, crashes, and newly reached screens while an agent works. Do
not hide an app run with `--headless`, a background-only desktop, or an
off-screen window unless the named experiment is genuinely independent of
UIKit, input, and graphics.

Agents may capture frames and click tested client coordinates to make repeated
runs objective and efficient. Keep the emulator window visible while doing so,
show one boundary at a time, and record the coordinate/timing recipe. Automated
screenshots and clicks support what a person can observe; they do not replace a
human playtest for menu, input, gameplay, orientation, or audio claims.

## Freeze the artifact identity first

For an Archive-backed target, follow `compatibility/README.md` exactly. Use the
item URL supplied by the maintainer; never search for a substitute.

At intake and again before a report-worthy clean run, use the full
`compatibility.py verify-archive` protocol. Before every intervening runtime
experiment, at minimum recalculate the local SHA-256:

```powershell
Get-FileHash -Algorithm SHA256 -LiteralPath '<exact local IPA>'
```

The local SHA-256 must match the value calculated during full verification and
recorded in the compatibility database for that exact filename. Archive
metadata supplies the MD5/SHA-1 checks used by the full protocol; do not imply
that it publishes the repository's SHA-256. If any hash does not match, stop
using that local copy. Confirm the embedded bundle identifier/version and
`tapHLE --info` output once per artifact and record the result in the app note.
A copied filename is not identity.

Do not duplicate IPAs between run directories. Keep them outside Git and pass
their absolute path to tapHLE.

## Follow an evidence ladder

Use the lowest-cost layer that can answer the current question:

1. Existing log or compatibility observation.
2. Source search for the exact symbol, status code, selector, or format.
3. Static inspection of the authorized app to recover a branch condition,
   call target, UI hit box, or faulting symbol.
4. A focused unit or TestApp regression.
5. A bounded diagnostic build and one controlled Windows run.
6. A clean committed release build and exact-artifact milestone run.

Static inspection and runtime tracing complement each other. Static work is
often faster for questions such as “what input advances this screen?” Runtime
work is required for questions such as “does that input reach the next level?”
Do not guess at hidden buttons, timers, formats, or ABI behavior through dozens
of blind runs when one branch condition or structure dump can answer it.

Keep static work equally bounded. Extract only the embedded `Info.plist` and
executable needed to answer the named question into a unique `%TEMP%`
directory; do not unpack an entire library of apps. Record the summarized
condition or symbol, then remove the exact temporary directory when it is no
longer needed.

For an archived UIKit screen, diagnose drawing and hit testing separately.
`UISubviews` is stored back-to-front, so a decorative full-screen view near the
end can intercept every control even when the buttons render correctly. Check
the exact archived interaction flag and any content wrapper objects (for
example, image-only button content) before changing event routing or drawing.
When a missing keyed value and an explicit false value have different
semantics, use `containsValueForKey:` before decoding; otherwise a decoder can
silently replace a subclass-specific default.

Treat layout as a separate runtime boundary too. A view created after launch
can be mounted, touchable, and still blank if its custom `layoutSubviews` never
runs. For an EAGL-backed view, distinguish these checkpoints in order:
display-link creation, layout-driven drawable allocation, a nonzero bound
renderbuffer, and `presentRenderbuffer:`. Do not paper over a missing layout
pass by calling every view's layout method every frame; that can make an app
destroy and recreate its framebuffer continuously. The current narrow
post-launch behavior lays out a newly mounted controller root once. Nested
runtime views and later geometry changes still need a future dirty-layout
implementation.

For a guest crash, resolve the program counter to its owning app image,
library, or HLE boundary before designing a fix. Inspect the register values
and object/allocation lifetime around that exact operation. A native library
fault, managed-code fault, and emulator panic have different recovery paths.

On Windows, first check the LLVM tools already installed with the pinned Rust
toolchain before installing another disassembler. After extracting only the
authorized app executable to a unique temporary directory, a narrow lookup is:

```powershell
$llvmBin = Join-Path (rustc --print sysroot) `
    'lib\rustlib\x86_64-pc-windows-msvc\bin'
$objdump = Join-Path $llvmBin 'llvm-objdump.exe'
$appBinary = '<temporary path to the extracted app executable>'
$faultPc = '1234' # clear the Thumb bit when an address includes it

& $objdump --macho --arch=armv6 --disassemble $appBinary |
    Select-String -Pattern "^\s*$faultPc`:" -Context 40,60
```

Use `llvm-nm.exe --arch=armv6 --defined-only` from the same directory to map
nearby methods and C++ functions. Keep only the small address context needed
to explain the fault; do not save or commit a full disassembly. Reconcile the
faulting operands with tapHLE's register dump. For an Objective-C++ object,
also check compiler-emitted `.cxx_construct` and `.cxx_destruct` methods before
treating a zeroed C++ container as valid initialized state.

When guest CPU execution fails without a debugger attached, current tapHLE
automatically prints all guest registers and the current stack trace before
panicking. Preserve that small diagnostic in normal builds: it can turn a null
memory error into a single disassembly lookup without a trace rebuild. If the
faulting instruction dereferences a zero register, inspect Mach-O non-lazy data
imports as well as lazy function imports. Compiler support such as
`___stack_chk_guard` is data, so a missing relocation may survive startup and
only crash later when generated code loads through the unresolved slot.

Rust and Cargo may retain absolute paths to an old worktree in `target` build
metadata. If a diagnostic build unexpectedly names a deleted sibling checkout,
do not recreate that sibling. Confirm the current repository and target path,
then invalidate only the affected cached build scope. Never treat a failure
caused by stale build metadata or a missing optional debug dependency as an app
compatibility result. Prefer fixing such a reference at its source over working
around it: a build script that locates files through `env!("CARGO_MANIFEST_DIR")`
bakes the compiling worktree's absolute path into the build-script binary, which
cargo can reuse from a since-deleted worktree; reading `CARGO_MANIFEST_DIR` at
run time (`std::env::var`) is stale-proof because cargo sets it fresh per
invocation. The native wrapper crates (`dynarmic`, `openal`) build their C/C++
via CMake and need `cmake` on `PATH`; a from-scratch debug build of those crates
will fail without it.

When the fault is an assertion at an HLE graphics boundary, recover the exact
guest API call and enum before relaxing it. Some old apps make invalid GL calls
that a real driver answers by recording `GL_INVALID_ENUM` and continuing. A
bounded emulator fix should preserve that guest-visible error behavior instead
of converting the app mistake into a host panic or ignoring all GL errors.

## Instrument one boundary at a time

Temporary diagnostics should answer one named question and produce little
output. Gate them by the exact format, API, queue, or bundle when practical.

Good diagnostic fields include:

- an API's input structure and status code;
- buffer byte size, count, pointer route, and a four-byte signature;
- the first few packet descriptors or offsets;
- one allocation size/address and its allocate/free/reuse events; or
- one UI phase, coordinate, and resulting level/screen transition.

Do not dump entire audio buffers, textures, guest memory, app binaries, or raw
logs. Never commit a diagnostic containing proprietary bytes or personal
paths. After the question is answered, remove trace-only code or convert the
small useful failure message into a normal diagnostic.

### Recognize compiler-created Objective-C blocks

Clang emits stack and global block literals directly in guest memory. They are
Objective-C-compatible objects, but they were not allocated through tapHLE's
normal object table. A high stack address receiving `copy`, combined with an
unhandled `__NSConcreteStackBlock` or `__NSConcreteGlobalBlock` relocation, is
evidence of a missing Blocks runtime rather than a corrupt ordinary object.

Trace only the final selector sequence and inspect the stable block prefix:
`isa`, flags, reserved word, invoke pointer and descriptor pointer. Implement
the documented Blocks ABI coherently: external block classes, stack-to-heap
copy by descriptor size, copy/dispose helpers, heap reference counts,
`_Block_object_assign`, `_Block_object_dispose`, and `__block` forwarding.
Returning the original stack pointer from `copy` may move one run forward but
leaves a use-after-return bug, so it is not a valid compatibility fix.

### Suspect the emulator's own iteration before the app's ownership

A message arriving at an object whose class differs from run to run is a
use-after-free. The tempting next step is to hunt for a retain the app is
missing, but check the other side first: does tapHLE deliver that message by
iterating a **copy** of a callback list it took before running guest code?

`NSNotificationCenter`, responder and delegate lists, timer and run-loop
queues all have to copy before dispatching, because delivery reenters the guest
and the guest may register or unregister during it. Copying is correct;
iterating the copy blindly is not. These lists deliberately hold unretained
references, so an entry that unregisters and is deallocated part-way through a
dispatch leaves the copy naming freed memory. Cocoa's guarantee that a removed
observer is not messaged is exactly what makes unregistering in a teardown
method safe, so a faithful implementation has to re-check each copied entry
against the live list before messaging it.

Two traps make this easy to misdiagnose:

- Instrumenting `retain`/`release` and attributing each call by whether a guest
  `LR` is available will label the emulator's own retain around a dispatch as
  guest code. A retain/release pair that brackets exactly one send, and nothing
  else, is more likely to be the host's dispatch guard than an app's.
- If the app routes its own event system through a Foundation class, the guest
  method names will not say so. Resolve the wrapper: a two-call method that
  fetches a singleton and then sends one selector to it is almost always a thin
  forwarder, and resolving its `__objc_selrefs` entries names the real API.

Read structures before reinterpreting data. For compressed audio, for example,
capture the `AudioStreamBasicDescription`, packet-description route/count, and
the first sync bytes before treating the buffer as PCM. For a stale-pointer
crash, prove allocation/free/reuse behavior before disabling an allocator rule
globally.

Use existing dependencies and subsystem abstractions before introducing a new
decoder, parser, or host library.

## Make Windows runs isolated and repeatable

Use a uniquely named directory under `%TEMP%` as tapHLE's working directory.
Link its `tapHLE_dylibs` and `tapHLE_fonts` to the checkout and copy the small
tracked default-options file rather than copying large support trees. Do not
add local options unless the experiment is specifically testing one.

Do not use `--headless` merely to hide an automated app run. UIKit and EAGL
paths can legitimately require `Environment.window`; headless mode may create
an earlier, unrelated unwrap or no-context failure. Use it only when the named
test is genuinely window-independent. A bounded normal-window run may redirect
logs and stop only its exact spawned PID.

For each meaningful run, record:

- exact tapHLE commit or that the build was dirty;
- verified IPA hash;
- tapHLE arguments and options source;
- client-area input coordinates and timing;
- last screen/event reached and whether the process stayed alive; and
- the narrow log lines supporting the conclusion.

Use real foreground mouse input when Windows message injection does not reach
SDL. Before every injected event, re-check that the foreground window belongs
to the exact spawned tapHLE process; abort input if focus or process identity
changed. Save and restore the previous cursor position and foreground window,
and stop only the exact process launched by the run. Visual inspection of a
screenshot can support a rendering observation, but the screenshot stays in
the ignored temporary directory.

If Windows foreground locking rejects `SetForegroundWindow`, temporarily
attach the harness thread's input queue to both the thread that currently owns
the foreground and the verified tapHLE window thread. Re-read both handles and
thread IDs on every bounded retry, detach in reverse order before sleeping, and
verify the exact target handle again after positioning the cursor and before
injecting input. Do not send a synthetic Alt key while an unrelated app owns
the foreground; it can alter that app's state. Abort safely if dual attachment
cannot establish ownership.

Allow the target event loop to consume the cursor move before sending the
button-down event (a short bounded pause such as 200 ms is sufficient in the
current harness). When touch routing is the boundary under test, confirm the
trace contains a `MouseMotion` at the intended client coordinate before the
matching `MouseButtonDown`. Sending the press immediately after
`SetCursorPos` can make SDL use the previous cursor coordinate and create a
false hit-testing diagnosis.

Also verify that `FocusGained` precedes the first intended client click.
Windows may consume the first click only to activate the emulator even when
`SetForegroundWindow` was requested. A reliable harness can click the spawned
window's title bar, wait for the focus event, and only then begin the recorded
client-coordinate recipe. Never use a focus click inside the app as if it were
part of the compatibility test.

Reuse a proven input recipe. Change one step at the frontier rather than
inventing a new route on every launch. A successful click path is evidence and
belongs in the app note.

### Capture the rendered frame when desktop capture is black

Windows desktop screenshot APIs may return a black OpenGL client area. tapHLE
can capture the next rendered frame after an explicit request marker appears.
EAGL apps capture the next valid renderbuffer submitted through
`presentRenderbuffer:`. UIKit-only screens capture the next frame presented by
the Core Animation compositor. This is a harness feature, not an app option:
set both paths in the child process environment before launch, then create the
request marker only after the target state is reached.

```powershell
$captureDir = Join-Path ([IO.Path]::GetTempPath()) (
    'taphle-frame-' + [Guid]::NewGuid().ToString('N')
)
[void](New-Item -ItemType Directory -Path $captureDir)
$tapHLEPath = '<absolute path to the exact committed tapHLE.exe>'
$ipaPath = '<absolute path to the exact hash-verified IPA>'
$requestPath = Join-Path $captureDir 'frame.request'
$outputPath = Join-Path $captureDir 'frame.ppm'

$env:TAPHLE_FRAME_CAPTURE_REQUEST = $requestPath
$env:TAPHLE_FRAME_CAPTURE_OUTPUT = $outputPath
try {
    # Start-Process returns immediately, which lets the harness create the
    # marker while this exact child is running. Set up the capture directory's
    # resource links/default-options file as described earlier in this section.
    $process = Start-Process `
        -FilePath $tapHLEPath `
        -ArgumentList ('"' + $ipaPath + '"') `
        -WorkingDirectory $captureDir `
        -PassThru
}
finally {
    Remove-Item Env:\TAPHLE_FRAME_CAPTURE_REQUEST -ErrorAction SilentlyContinue
    Remove-Item Env:\TAPHLE_FRAME_CAPTURE_OUTPUT -ErrorAction SilentlyContinue
}

# At the desired UI state, create the marker without replacing anything.
$marker = [IO.File]::Open(
    $requestPath,
    [IO.FileMode]::CreateNew,
    [IO.FileAccess]::Write,
    [IO.FileShare]::None
)
$marker.Dispose()

$deadline = [DateTime]::UtcNow.AddSeconds(10)
$captureComplete = $false
while (-not $captureComplete -and [DateTime]::UtcNow -lt $deadline) {
    if (Test-Path -LiteralPath $outputPath -PathType Leaf) {
        # The destination becomes visible before its payload is fully written,
        # so file existence alone is not completion.
        ffmpeg -v quiet -i $outputPath -f null -
        $captureComplete = $LASTEXITCODE -eq 0
    }
    Start-Sleep -Milliseconds 100
}
if (-not $captureComplete) {
    throw 'Timed out waiting for a complete, decodable frame capture'
}
```

The output path must not exist. tapHLE creates it with no-overwrite semantics,
logs the PPM dimensions on success, and makes only one attempt per marker. A
successful attempt removes the marker on a best-effort basis and re-arms the
same paths. To capture a later state in the same process, move the completed
output to a unique evidence filename, perform the next action, then create the
same marker again. A write failure, inaccessible path, invalid marker, or
pre-existing output is logged without crashing the emulator; that failure
disarms the request and may leave the marker in place, so restart tapHLE before
retrying.

Wait for the output with a short deadline rather than an unbounded loop. Check
that the file begins with a `P6` header and that a decoder reports the expected
dimensions and complete pixel payload. The capture has OpenGL's pixel origin
and may need a vertical flip and an orientation-specific rotation for visual
inspection. For example, Ricky's landscape-left diagnostic used:

```powershell
ffmpeg -i $outputPath -vf 'vflip,transpose=2' (
    Join-Path $captureDir 'frame-oriented.png'
)
```

Read the success log before interpreting the image because the rendering path
changes what was captured:

- `Captured submitted EAGL renderbuffer` is the guest renderbuffer before host
  rotation, layer composition, scaling, letterboxing, or virtual-cursor
  drawing. In a multi-layer app, it may not be the layer ultimately visible.
- `Captured presented Core Animation frame` is the presented viewport after
  host rotation and UIKit layer composition. This is the better check for a
  menu or other UIKit-only screen that stops submitting EAGL frames.

Both captures have OpenGL's bottom-left pixel origin, so start visual
conversion with `vflip`. Apply an additional rotation only to a pre-rotation
EAGL capture when its orientation requires one. The synchronous readback can
briefly stall one frame and temporarily holds both RGBA and RGB copies in
memory. Keep the PPM, converted image, marker, and raw logs in the unique
temporary run directory; never add them to Git or treat a dirty-build capture
as compatibility-database evidence.

## Treat shutdown as an observable boundary

Test process shutdown separately after reaching a meaningful milestone. Post
`WM_CLOSE` only to the verified window owned by the spawned tapHLE PID, allow a
short fixed grace period, and record both the process exit code and whether the
harness had to kill it. A lifecycle callback or the absence of a Rust panic is
not sufficient evidence of a clean exit.

On Windows, take an Application event-log baseline before posting the close and
then query new event ID 1000 entries for `tapHLE.exe`. Native structured
exception handling or coroutine forced unwinding can produce an access
violation after guest shutdown logs appear normal. A clean result requires a
natural zero exit, no forced termination, and no new matching crash event. Use
a fresh run directory when confirming a fix so persisted app state cannot mask
the path.

PowerShell scripts that launch the emulator through
`System.Diagnostics.Process` may leave `$LASTEXITCODE` unset even after a
successful run. Validate the script with `$?` and its explicit `EXIT_CODE` and
`FORCED_TERMINATION` fields instead of turning an unset value into a false
failure.

When shutdown reaches a missing thread-termination API, first establish the
guest thread ID and whether the call comes from the pthread's top-level start
routine or from a nested host-to-guest callback. These paths cannot be assumed
to unwind the same host coroutine. Prefer a normal return from the coroutine
for the proven path; forced unwinding suspended Windows coroutines can fault in
the host runtime.

Also account for dynamic-link stub shape. A four-byte lazy symbol stub branches
to the guest LR after its host implementation returns, while larger stubs may
continue at the PC set by that implementation. A non-returning HLE function
must redirect all control-flow state required by each supported stub shape.
State the boundary explicitly: supporting top-level secondary-thread exit does
not imply support for main-thread exit, nested callbacks, pthread cleanup
handlers, or thread-specific-data destructors.

## Verify streaming audio in layers

Separate four questions: did the decoder produce PCM, did buffers recycle, did
the host mixer emit signal, and did a person hear sane audio? One observation
does not automatically answer all four.

This cuts both ways, and the negative direction is the easier mistake. A log
full of failed loads is not evidence that the app is silent. An app may reach
audio by more than one route — a failing `AudioFileOpenURL()` loader and a
working OpenAL streaming path can coexist in the same title — so a repeated
warning bounds which loader failed and nothing more. Baby Monkey was recorded as
having "no audio" on exactly that reasoning while it was in fact playing music
throughout; the wave capture below settled it in one run, and the maintainer had
simply heard it. Never state what a user will hear, see, or be unable to do
without an observation at that layer. If measuring is not practical, write down
the API-level fact you actually have and say the user-facing consequence is
unknown.

The bundled OpenAL Soft Wave File Writer can capture tapHLE's process-local
mixed output without a microphone or system loopback. In a unique run
directory, create an OpenAL config with a new output path and launch tapHLE with
`ALSOFT_CONF` pointing to it and `ALSOFT_DRIVERS=wave`. Use stereo, signed
16-bit output at the app's sample rate when possible. Assert that the target
WAV does not exist first: the backend overwrites it without prompting. This
backend replaces speaker output for the run.

Use `ffprobe` to validate the resulting codec, rate, channel count, duration,
and file size. Use `ffmpeg` `silencedetect`, `volumedetect`, or `astats` over
bounded time windows rather than inspecting raw samples. For a streaming
queue, calculate how much audio its initial buffers contain. Meaningful signal
beyond that duration supports continuity, but pair it with a module-scoped
queue trace when the conclusion matters. A strong lifecycle trace shows each
guest buffer repeating this chain:

```text
processed by OpenAL -> guest callback -> re-enqueue -> decode
```

Count exact buffer references, decode/recycle events, decoder warnings, and
source-underrun restarts. Do not dump encoded packets or PCM.

tapHLE may open separate OpenAL contexts for guest OpenAL and internal Audio
Toolbox. OpenAL Soft's wave backend uses one global output filename, so two
devices can truncate or overwrite regions of the same file. Do not interpret
that file's timeline by itself when multiple devices appear in the OpenAL log;
use the queue lifecycle trace to attribute continuity. A wave capture proves
host-mixer signal, not default-device routing or subjective audio quality.
Keep the compatibility feature `partial` until listening or another trustworthy
end-to-end observation confirms sane audible output.

Close only the spawned process and allow a bounded grace period so the backend
can finalize RIFF lengths. If shutdown panics or needs forced termination,
verify the header and duration before using the capture and record that
limitation. Never silently repair or classify an ambiguous file as evidence.

## Conserve time and disk

- Check free space before a large build or extraction when the drive has been
  under pressure.
- Reuse Cargo's incremental/target cache; do not run `cargo clean` as routine
  troubleshooting.
- Build once for a batch of trace questions instead of rebuilding after every
  log line.
- Filter a large log for the exact symbol, status code, or subsystem plus a few
  context lines; do not repeatedly reread or paste the full file.
- Prefer junctions/hard links over copied runtime support trees.
- Remove only the exact temporary extraction/run directories created for the
  completed experiment. Never use a broad wildcard or an unresolved path. The
  practice that makes this automatic: the harness that creates a directory
  captures its path in a variable and is the only thing that deletes it, by that
  variable. Enumerating the temporary directory by a `taphle-*` pattern is the
  mistake this rule exists to prevent — that directory is shared by every
  session and every app, so a matching name is not evidence that the directory
  is yours, and a force delete there does not go to the Recycle Bin. If you are
  not certain a directory is yours, leave it: stale temporary data costs
  nothing, and it may be another investigation's only remaining evidence.
- Keep the small sanitized conclusion, not gigabytes of intermediate data.

## Choose and bound the fix

The fix may be general, intentionally partial, or game-specific. Prefer the
smallest complete behavior supported by evidence.

Treat a later crash as a new observation, not automatically as progress. A
useful step implements or corrects a coherent guest-visible capability and
proves that the target crosses that boundary. Suppressing several imports with
unrelated success stubs is not a checkpoint; preserve such experiments only as
leads and review each API contract separately.

- A general implementation should validate its inputs and fail safely for
  unsupported variants.
- A partial implementation should state the exact supported shape and leave a
  visible fallback/TODO for other routes.
- A game workaround should be gated as narrowly as possible and explain the
  observed behavior it preserves.

Do not broaden scope to adjacent formats, platforms, or APIs just because they
are nearby. An implementation for the route the target actually uses is a
valid checkpoint. Add a synthetic regression that contains no proprietary
fixture whenever deterministic logic can be isolated.

## Commit, then make compatibility claims

A dirty-worktree run is a useful experiment but never database evidence.

1. Commit the focused implementation on `compat/<app-slug>`.
2. Build and run the relevant tests from that exact commit.
3. Re-verify the IPA hash and replay the milestone on Windows.
4. If the milestone is reproducible, append a compatibility report referencing
   the tested implementation commit.
5. Submit the verified result to the live compatibility database when
   publication is authorized.
6. When publishing is authorized, push unfinished checkpoints to the
   compatibility branch so another agent can resume them. Merge a stable,
   documented milestone to `trunk` even when known limitations remain.

Keep implementation, agent-policy documentation, and compatibility reports in
separable commits. This makes incomplete app experiments easy to continue or
revert without losing durable process improvements.

## Continuation-note template

Create `dev-docs/app-notes/<app-slug>.md` on the app branch when work will span
more than one focused turn. Keep it sanitized and concise:

```markdown
# <App> compatibility work note

- Branch and last pushed commit:
- Canonical artifact URL, filename, SHA-256, bundle ID/version:
- Highest clean committed milestone:
- Proven input recipe:
- Proven facts:
- Rejected hypotheses:
- Current uncommitted diagnostics or code:
- Checks already run:
- Known risks/regressions to watch:
- Next discriminator or implementation step:
```

Do not put app code, decompiled listings, assets, screenshots, raw logs, keys,
or personal paths in the note. Exact public provenance, hashes, symbol names,
addresses, API shapes, and summarized behavior are sufficient for continuity.
