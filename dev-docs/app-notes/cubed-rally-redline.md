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

## Next discriminator

The crash after death is `Error during CPU execution: MemoryError`, faulting at
guest PC `0x3cda10` (ARM, not Thumb — pass `--arm` to
`dev-scripts/disasm-guest-fault.py`) with `R0 = 0`, `R1 = 0xbe`. The instruction
is `ldrb r3, [r8]` where `r8 = r0 + r2` and `r0` is the null base pointer. The
surrounding code compares bytes against `'/'`, `'\t'` and `' '`, so it is a text
parser walking a buffer it was handed as null.

It is reached on the post-death path, right after a `CFStringGetBytes` measuring
pass, and this build links `P31RestKit` (Prime31) and calls Flurry, so the
likely route is score submission or analytics parsing a response that tapHLE's
networking never produced. Start by finding which call returns the null buffer
rather than by looking at the parser.

Note this is *after* a complete play session, so it does not block the two-star
result; it blocks the loop persisting into a second race.
