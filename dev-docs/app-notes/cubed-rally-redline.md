# Cubed Rally Redline compatibility work note

- Branch and last pushed commit: `compat/cubed-rally-redline`, see below for the
  tested revision.
- Artifact: maintainer's local copy, filename
  `Cubed Rally Redline (v1.32) [Decrypted].ipa`,
  SHA-256 `837ED1C107602EC6738CBD29DFE53BCD15F6DD57A429D7F9F88A6BDAD3F7DE87`.
  Not Archive-backed; no public provenance recorded.
- Identity, read from `tapHLE --info` (not from the filename): display name
  `Redline`, bundle identifier `com.nocanwin.redline`, bundle version `1.32`,
  minimum OS version `5.0`, armv7, iPhone + iPad, landscape only.
- Highest clean committed milestone: **reaches a race and plays it**. Menus,
  car select and the controls screen all work; the race starts, the HUD counts
  distance, the car responds to physics and the player dies. It crashes shortly
  after death (see the frontier below). Honest rating: **two stars** — stable
  screens, a loop that starts but does not persist.

## What this app is

A Unity 4.0.1f2 game, built against the iOS 6.0 SDK. That makes it much newer
than tapHLE's usual targets and the first Unity title to run, so most of what
was fixed for it is engine-shaped rather than game-shaped: the Mono runtime, its
allocator, and Core Foundation string and data bridging. `--info` warns that the
app wants iOS 5.0 and tapHLE supports 4.0 and earlier; that warning is not fatal
and did not need addressing.

`cryptid` is 0, so the binary is genuinely decrypted and load failures would be
real results rather than an artefact of a bad dump.

## Click map

Launch options: none (no entry in `tapHLE_default_options.txt`). Window client
area 480x320, landscape, matching the app's own resolution 1:1.

Wait about 55 s after launch before the first tap — the Mono runtime, scene load
and asset unload take that long.

**Press duration matters: use ~300 ms.** A 90 ms press is not seen at all, and
that produced a convincing false negative — the title screen animates on its own
for a few seconds after load, so a frame comparison right after a missed tap
still shows a change. Confirm the screen is static first, then tap.

| From | Client tap | To |
| --- | --- | --- |
| Title / mode select | (242, 141) | Car select |
| Car select | (247, 264) | Controls screen |
| Controls screen | (239, 287) | Race |

## Proven

- Unity engine initialises, Mono loads every assembly including
  `Assembly-CSharp.dll`, and scenes load (4711 objects for the race scene).
- `applicationDidBecomeActive()` fires and the Core Animation compositor
  presents.
- Gameplay is real, not a still frame: `stdout` records `NoMetric:Play`, then
  `Player: Dead`, then `NoMetric:EndTimed:Game`, and OS-level screenshots hash
  differently between samples during the race.
- These helpers use the base AAPCS convention (operands and results in core
  registers, not VFP), read off `___fixdfdi` and `___floatdidf` call sites in
  this binary rather than assumed.

## Rejected

- "The soft-float helpers return in `d0` because iOS is AAPCS-VFP." Disproved by
  disassembly: the app does `vmov r0, r1, d16` immediately before calling
  `___fixdfdi` and reads a `___floatdidf` result from `r0`/`r1`.
- "The app freezes because it is waiting for input." It was spinning on an
  `mmap` that could not be satisfied; frames were byte-identical for 30 s.
- "The first tap did not land because touch routing is broken." The press was
  simply too short.

## Known gaps that are visible but not fatal

- **The track does not render.** During a race the sky, car, HUD and buttons
  draw, but the terrain does not, so the car falls with nothing under it. This
  is the most valuable thing to look at next for anyone wanting the game
  playable rather than merely running; it is a rendering gap, not a crash, and
  is untouched by any of the work so far.
- Several menu buttons are white rectangles, i.e. missing textures.
- No audio. Unity decodes compressed clips through `AudioQueueOfflineRender`,
  which tapHLE does not implement; the call now fails cleanly and the clip is
  dropped.
- No tilt control: Unity reads acceleration through Core Motion's block-based
  `startAccelerometerUpdatesToQueue:withHandler:`, which is a no-op.

## 2026-07-31 graphics retest

On Windows, the exact recorded IPA still reaches the animated title and the
Redline car-selection scene in a visible 480x320 window. EAGL captures show
that the fault is not limited to the race: the selection scene has broad green
areas and striped/corrupt image panels, while the title has missing art. The
earlier race capture's flat green terrain therefore belongs to the same broad
rendering frontier.

Unity queries `GL_MAX_RENDERBUFFER_SIZE_OES` (`0x84e8`) immediately after
creating its GLES 1.1 context. The GLES1-on-GL2 layer previously treated it as
unknown and left the guest result untouched. The local follow-up answers that
supported `OES_framebuffer_object` query through the host extension and has a
unit regression check. It removes the warning but does **not** change the
captured title or selection rendering, so do not treat it as the graphics fix.

Unity also logged `NPOT RTs are not supported: ignoring SetResolution` before
the menu scene. Windows uses the GLES1-on-GL2 backend, whose OpenGL 2.1 texture
implementation supports unrestricted non-power-of-two textures, but the guest
extension string did not advertise `GL_OES_texture_npot`. Adding that capability
removes the Unity warning and improves the captured title: the Cubed Rally logo
and HUD chrome render instead of the earlier broadly blank/corrupt layout. Its
central menu artwork is still missing, so this is a verified incremental
rendering improvement, not a graphics-complete or three-star claim.

A bounded title-scene texture-upload trace sees ordinary `glTexImage2D`
uploads, including 512x1024 RGBA and 256x256 RGBA atlas textures with complete
mip chains. It sees no compressed-texture upload. The remaining blank artwork
is therefore not explained by a missing PVRTC/paletted decoder; inspect texture
state or draw state next.

A matching client-array trace finds Unity's colour arrays use
`GL_UNSIGNED_BYTE` (stride 24), not `GL_FIXED`. Fixed-point colour translation
is not the source of the missing artwork.

A real foreground `SendInput` rerun confirms the three-tap route: title
`(242, 141)`, car select `(247, 264)`, then controls `(239, 287)`. It reaches
`NoMetric:Game` and produces a race frame. The car, HUD, clouds, and red guide
line render, but the dirt track and its green terrain are entirely absent.
The striped car-selection and controls frames show the same failure outside
the race. Unity populates many static `ARRAY_BUFFER` VBOs while loading the
game but does not call `glMapBufferOES`, so mapped-buffer forwarding is not the
terrain failure.

Temporary global probes that suppress depth testing, face culling, or fog all
leave the terrain absent. They change layering of sprites/clouds but do not
make road geometry appear, so do not treat depth range, winding, or fog
conversion as the current root cause.
Suppressing blending likewise leaves the terrain absent while visibly breaking
the sprites and HUD, so normal source/destination compositing is not the
missing road path either.

A bounded title-scene trace that consumed host GL errors after every guest GLES
entry point found none. Continue with a draw/state trace or a captured working
reference, rather than adding more guessed extension constants. The documented
synthetic-message route is not evidence; use real foreground input for every
gameplay claim.

## 2026-08-01 draw and review-prompt trace

The repeated title/cars corruption is produced in the guest renderbuffer,
before the host window presentation. The affected GUI draws have texturing
enabled and `GL_COMBINE` selected, but alpha test, lighting and scissor test
are all disabled; their viewport and scissor box cover the full 480x320
surface. Those states do not explain the blank/striped artwork.

The 512x1024 RGBA Unity atlas used by the affected UI draws was read back from
the host after `glTexImage2D`. Both its colour content and alpha mask match the
uploaded atlas rather than being blank or zeroed. The remaining graphics defect
is therefore after texture decode/upload: distinguish texture-coordinate,
texture-matrix and texture-combine input state for the bad draw batches.

A temporary `GL_REPLACE` probe for every draw changes the prompt's colours but
does not restore its collapsed road/background or the race terrain. Texture
combine is not the missing-geometry cause; inspect the texture-coordinate and
geometry input state next.

For the affected 512x1024-atlas batches, Unity supplies both position and UV
arrays directly from guest memory as `GL_FLOAT` with a 24-byte stride; neither
is VBO-backed. Both active texture selectors are `GL_TEXTURE0`, the texture
matrix is identity, and the sampled first position/UV values are finite and in
the expected normalized ranges. The pointer translation and primary texture
matrix are therefore not immediate explanations either; inspect the remaining
fixed-function raster state or compare the same draw batches to a working
renderer.

A complete indexed-array scan narrows this further. The 84-index batch spans
raw X/Y `-0.8..0.8` / `-0.275..0.281` and transformed NDC X/Y
`-2.125..2.125` / `-1.096..1.120`; its UVs span nearly the full atlas height.
The 402-index batch spans NDC X/Y `-0.994..0.987` / `-0.990..0.995`, with
finite UVs covering `0.045..0.182` horizontally and effectively `0..1`
vertically. Both use the same finite model-view and perspective matrices. The
corrupt frame therefore reaches desktop GL with viewport-covering indexed
geometry, not collapsed guest vertices, indices, transforms, or UV pointers.
Continue at texture sampling and the remaining fixed-function pixel/raster
state, or compare these exact batches with a known-good renderer.

Forcing `GL_LINEAR` minification immediately before every indexed and
non-indexed draw, so all textures sample level zero instead of their uploaded
mipmap chains, leaves the corrupt prompt and background unchanged. Bad mip
selection is not the cause either. Do not repeat level-zero or mip-filter
probes; identify the draw subset that fails or inspect the remaining pixel and
raster state.

A deduplicated draw inventory shows that the prompt/UI batches are all
`GL_TRIANGLES` into framebuffer 1 with a 480x320 viewport. Across textures 31,
33, 35 and 37 they share the same state: texture, blending, depth testing and
dithering enabled; vertex, UV and colour client arrays enabled; alpha test,
culling, fog, lighting, scissor, stencil, logic op and polygon offset disabled;
`GL_NEAREST_MIPMAP_NEAREST` / `GL_NEAREST` filtering and repeat wrap. Because
correct and corrupt UI elements pass through that same state, do not pursue a
state difference between those batches without new evidence. Isolate their
individual geometry/UV topology or compare the guest-produced arrays instead.

`--landscape-native` changes Unity's first framebuffer-1 viewport from 320x480
to 480x320, but the settled prompt frame retains the same road-strip cutoff and
green background. The early portrait viewport is not the cause of the lasting
corruption, and this app should continue to launch without that option.

On a real 300 ms foreground press, the first title action opens the game's
rate prompt. Both visible actions (`NOT NOW` and `RATE`) log
`NoMetric:ReviewRequest` and then raise JSONKit's null-input assertion before
the same `0x3cda10` parser fault. `RATE` is therefore not a bypass to the car
selection scene. A temporary trace confirmed this path does not call the
existing `CFReadStreamOpen` stub, and the existing synchronous
`NSURLConnection` log does not appear. Do not change either network API on this
evidence alone; trace the actual data-producing Foundation or third-party
boundary first.

A full Objective-C selector trace identified that boundary in JSONKit's public
NSString adapter: it calls `CFDataCreateMutable` with the UTF-8 capacity, then
immediately calls `CFDataGetMutableBytePtr` and converts into the reserved
storage. tapHLE discarded `NSMutableData` capacity and returned a null pointer.
Preserving reserved capacity removes the JSONKit null-input assertion and the
`0x3cda10` fault.

The next run reached `0x3cf4c4`, where the binary dereferenced non-lazy slot
`0xb50204`. `nl-symbol-at.py` identifies it as
`_OBJC_IVAR_$_NSObject.isa`; linking that ivar-offset global to a word
containing offset zero removes this second fault. JSONKit then advances to its
private guest `JKDictionary` subclass, where an inherited tapHLE
`NSDictionary` method rejects the object because it has no host
`DictionaryHostObject`.

## Previous discriminator

The crash after death (and now the review-prompt dismissal) is `Error during CPU execution: MemoryError`, faulting at
guest PC `0x3cda10` (ARM, not Thumb — pass `--arm` to
`dev-scripts/disasm-guest-fault.py`) with `R0 = 0`, `R1 = 0xbe`. The instruction
is `ldrb r3, [r8]` where `r8 = r0 + r2` and `r0` is the null base pointer. The
surrounding code compares bytes against `'/'`, `'\t'` and `' '`, so it is a text
parser walking a buffer it was handed as null.

It is reached after a `CFStringGetBytes` measuring pass, and this build links
P31RestKit (Prime31), ChartBoost and Flurry, so the likely route is score
submission, review or analytics parsing a response that tapHLE never produced.
Start by finding the concrete data-producing call that returns the null buffer
rather than by changing the parser or guessing at a network API.

The post-death occurrence blocks the loop persisting into a second race; the
review occurrence blocks ordinary menu progression when its prompt is shown.

## Next discriminator

The prompt parser now fails because JSONKit directly allocates its
`JKDictionary` guest subclass, while tapHLE's inherited `NSDictionary`
implementation assumes every receiver owns a host `DictionaryHostObject`.
Trace the first inherited collection selector and make that Foundation
primitive operate on the guest subclass's real Objective-C storage, or provide
the smallest correct class-cluster bridge. Do not fake a JSON result or
special-case JSONKit.

The post-death path may share the same JSONKit sequence but has not been rerun
with these two fixes. The prompt still blocks ordinary menu progression, and
the graphics defect remains independently unresolved.
