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
- Highest clean committed milestone: **the gameplay loop starts and
  persists, but the game is not playable**. A race starts, the HUD counts
  distance and score, the car responds to physics, the player crashes, the
  RESULT screen appears, and its play button starts another race that also
  runs to a result. **Do not read that as three stars.** The ground is a flat
  opaque grey sheet with a hard diagonal horizon, and the voxel terrain is a
  scatter of a few pixels, so a person cannot see where the road is. Rating
  stays at **two stars** until the rendering is fixed; the loop persisting is a
  necessary part of three stars, not a sufficient one. See "The rendering
  frontier, measured (2026-08-04)" for exactly which draw does what.

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


## Underlying systems this app uses that tapHLE does not implement

Measured, not guessed. Two sources, and they answer different questions:

- **Imported but unprovided** — `tapHLE --dump=linking-info` on the exact
  artifact: 102 lazy symbols with no `linked_to`, and 27 Objective-C classes
  tapHLE reports as `unimplemented`. An entry here is a gap in tapHLE's API
  surface; it does not prove the game calls it.
- **Actually reached** — the `TODO:`/`Warning:` lines a real windowed run
  emits while playing to a race result. These are the gaps the game hits.

Do not read the 930 selectors the dump lists as unimplemented as gaps. Almost
all are the app's own (Chartboost and Prime31 ad-SDK) selectors, which have no
host implementation because they are guest code.

### Reached during play, in rough order of how much they cost the game

1. **Offline audio rendering.** `AudioQueueOfflineRender` is unimplemented, so
   `AudioQueueSetOfflineRenderFormat` fails. This is how Unity decodes
   compressed clips, so those sounds are dropped. Fails cleanly; the game runs.
2. **Core Motion.** `CMMotionManager` is reached twice per launch. Unity reads
   acceleration through the block-based
   `startAccelerometerUpdatesToQueue:withHandler:`, which is a no-op, so tilt
   steering does not work. The game is still playable with the on-screen
   left/right controls.
3. **Audio session properties.** Three `AudioSessionSetProperty` and three
   `AudioSessionAddPropertyListener` calls are TODOs, so the app cannot observe
   route changes or interruptions.
4. **Analytics and ads.** `Flurry` is the only faked class actually messaged
   (`logEvent:`, `logEvent:timed:`, `logEvent:withParameters:`,
   `setSessionReportsOnPauseEnabled:`). Chartboost and Prime31 RestKit are
   present in the binary but their network paths produce nothing, since
   `NSURLConnection` is a stub and `NSJSONSerialization` is unimplemented — the
   app falls back to its bundled JSONKit, which is what exposed the
   guest-allocated-collection defect.
5. **`__cxa_atexit` (476 calls) and `atexit`.** Registered destructors are
   never run. Harmless while the app is alive.
6. **`readlink` (256 calls), `stat`/`fstat` (mostly unimplemented), `ferror`,
   `sigaction` (7 signals), `sigprocmask`, `signal`, `mprotect`,
   `pthread_attr_setschedpolicy`/`setschedparam`, `pthread_detach`.** Mono
   probes all of these; none is fatal.
7. **UIKit trimmings** actually touched: `UIViewController` properties,
   `UITextField` border/clear-button styles, `setExclusiveTouch:`,
   `isGeneratingDeviceOrientationNotifications`, and the
   "existing view hidden by another view" appearance case.
8. **`AVCaptureSessionPreset*`** constants are unresolved non-lazy symbols, so
   the camera path would fault if it were ever entered. It is not entered in
   normal play.

### Imported but unprovided, grouped by system

Not observed to be called in a normal play session; listed so the next agent
knows what is missing rather than re-deriving it.

- **Game Center (15 classes):** `GKAchievement`,
  `GKAchievementDescription`, `GKAchievementViewController`,
  `GKFriendRequestComposeViewController`, `GKLeaderboardViewController`,
  `GKMatchRequest`, `GKMatchmaker`, `GKMatchmakerViewController`, `GKPlayer`,
  `GKTurnBasedEventHandler`, `GKTurnBasedMatch`,
  `GKTurnBasedMatchmakerViewController`, `GKTurnBasedParticipant`,
  `GKVoiceChat`, `GKVoiceChatService`.
- **Camera capture:** `AVCaptureSession`, `AVCaptureDevice`,
  `AVCaptureDeviceInput`, `AVCaptureVideoDataOutput`, plus
  `CMSampleBufferGetImageBuffer`, `CMTimeMake` and the six `CVPixelBuffer*`
  entry points.
- **Networking and serialization:** `NSHTTPURLResponse`, `NSURLCredential`,
  `NSJSONSerialization`, `NSPredicate`,
  `CFURLCreateStringByReplacingPercentEscapesUsingEncoding`.
- **Objective-C exceptions and runtime introspection:**
  `objc_begin_catch`, `objc_end_catch`, `objc_exception_rethrow`,
  `objc_enumerationMutation`, `class_getInstanceVariable`,
  `class_getIvarLayout`, `ivar_getName`, `ivar_getOffset`, `object_setIvar`,
  `protocol_getMethodDescription`.
- **Mach-O/dyld introspection:** `_dyld_image_count`,
  `_dyld_get_image_header`, `_dyld_get_image_name`,
  `_dyld_register_func_for_add_image`, `getsectbynamefromheader`.
- **Drawing:** `UIBezierPath`, `UIPinchGestureRecognizer`,
  `UIImagePNGRepresentation`, `UIRectFill`, `CGGradientCreateWithColors`,
  `CGContextDrawLinearGradient`, `CGContextDrawRadialGradient`,
  `CAKeyframeAnimation`, `CATransform3DRotate/Scale/Translate`.
- **Store:** `SKMutablePayment`.
- **POSIX process and IPC:** `fork`, `execv`, `execve`, `pipe`, `poll`,
  `kill`, `raise`, `dup2`, `getuid`, `geteuid`, `getpwnam_r`, `getpwuid_r`,
  `getgrnam_r`, `getgrgid_r`, `getrusage`, `getpriority`, `setpriority`,
  `setitimer`, `sched_get_priority_min/max`, `pthread_cancel`, `pthread_kill`,
  `chmod`, `rmdir`, `utime`, `tcgetattr`, `tcsetattr`, `tcflush`.
- **Sockets:** `getnameinfo`, `getpeername`, `getprotobyname`, `inet_ntoa`,
  `recvmsg`, `sendmsg`, `freeifaddrs`, `gai_strerror`.
- **Other libc:** `__assert_rtn`, `__snprintf_chk`, `__vsnprintf_chk`,
  `strtoll`, `bcopy`, `nl_langinfo`, `iconv`/`iconv_open`/`iconv_close`,
  `sys_icache_invalidate`, `dlerror`, `_exit`.
- **Apple System Log:** `asl_new`, `asl_get`, `asl_key`, `asl_search`,
  `aslresponse_next`, `aslresponse_free`.
- **CommonCrypto:** `CC_SHA1_Init`, `CC_SHA1_Update`, `CC_SHA1_Final`.
- **Core Foundation:** `CFArrayCreate`, `CFArrayGetValues`,
  `CFDictionaryCreate`, `CFUUIDGetUUIDBytes`, and the four `CFBitVector*`
  entry points.
- **Audio:** `AudioUnitRender`.
- **GLES 2 introspection:** `glGetActiveAttrib`, `glGetActiveUniform`,
  `glGetShaderSource`. The game runs its GLES 1.1 path, so these are unused.

**None of this explains the rendering defect.** Every gap above is a missing
API; the render failure is in code tapHLE does implement. Keep the two apart.

## The rendering frontier, measured (2026-08-04)

Everything written before 2026-08-03 about textures, combiners and mip
selection was observed through the double-rotation presentation bug and is
worthless. The 2026-08-03 conclusion — "seven world batches rasterise zero
pixels, the geometry is placed where the camera cannot see it" — is **also
wrong**, and is corrected below. It came from computing NDC by hand for one
vertex read out of a shared scratch buffer at the wrong offset.

This section is measured with a probe that read back the framebuffer before
and after **every** draw of a race frame, alongside the matrices, the GL state,
and the vertex/texcoord data read through the very array pointers the driver
uses (`glGetPointerv`, not the probe's own bookkeeping).

### What the defect actually is

A race frame is 13–16 draws. Every one of them rasterises. Nothing is culled
away and nothing lands off-screen by mistake. The frame is:

| what | indices | texture | result |
| --- | --- | --- | --- |
| terrain chunks | 228–480 each | 103 | 0–6 pixels each |
| **ground plane** | **1050** | **103** | **~80 000 px, flat (128,128,128)** |
| car | 1020 | 99 | correct |
| clouds | 18–36 | 97 | correct |
| HUD, text, particles | 6–546 | 35/65/67 | correct |

So the world **is** drawn. The screen-covering grey sheet the maintainer sees
as "shredded diagonal streaks" is one 1050-index ground plane resolving to a
single flat colour, and the voxel terrain chunks are each a handful of pixels.

The projection is a correct 480x320 orthographic matrix, the modelview is a
constant proper rotation, and the camera position recovered from it
(`-Rᵀt`) tracks the car sensibly: y fixed at 19.71, z advancing about 0.3
units a frame while the car drives. **The camera and the geometry transform
are fine.** Do not look there again.

### Why the ground plane is flat grey

- Texture 103 is a 1024x1024 `GL_RGB`/`GL_UNSIGNED_BYTE` sprite atlas with a
  guest-supplied 11-level mip chain, `MIN_FILTER = NEAREST_MIPMAP_NEAREST`,
  `MAG_FILTER = NEAREST`, wrap `REPEAT` on both axes.
- The bytes the guest hands `glTexImage2D` are **correct**. They were dumped
  and viewed: a proper atlas, art packed into a strip at one end, the rest
  white. Levels 0–10 are all sane; the 1x1 level is (239,238,235).
- The ground plane's texture coordinates, read through GL's own array pointer,
  span **u ∈ [-10.4, +21.4], v ∈ [-1.0, +1.0]** — while every other draw in
  the frame is inside 0..1. The u range *slides* as the car drives (20.19,
  20.60, 21.44 on three consecutive frames) but keeps a roughly constant span
  of about 31. That is a scrolling, tiling strip.
- Tiling a mostly-white atlas 31 times across the screen makes the minification
  derivative enormous, so the sampler lands on the top mip levels, which are
  near-white. Near-white texel × a ~0.5 primary colour is exactly the observed
  (128,128,128).
- Proof: forcing every `*_MIPMAP_*` minification filter down to `GL_NEAREST`
  makes texture detail reappear on that plane (a tiled, multicoloured band
  instead of flat grey). The grey is texture-sampling collapse, not geometry.

The state at that draw: `TEXTURE_ENV_MODE = GL_COMBINE`, `GL_LIGHTING`
enabled, blending off, depth test and depth write on, culling on, texture
matrix identity, `ACTIVE_TEXTURE = CLIENT_ACTIVE_TEXTURE = GL_TEXTURE0`.

### Ruled out, with the evidence

- **The texture matrix.** The guest touches it five times in a whole session,
  always `glLoadIdentity` on an already-identity matrix. It never scales UVs
  that way, so `present_frame` clobbering it could not be the cause. (Its
  push/pop in `present_renderbuffer` is symmetric anyway.)
- **`fmod`/`fmodf`.** Rust's `%` on floats is C `fmod` semantics; the
  implementations in `src/libc/math.rs` are correct.
- **The soft-float helpers.** `src/libc/compiler_rt.rs` is correct for every
  finite value, and the marshalling is pinned by tests.
- **`mach_absolute_time`/`mach_timebase_info`.** Nanoseconds with a 1/1
  timebase; correct.
- **The buffer-mapping path.** `glMapBufferOES` is never called by this app.
- **`--landscape-native`**, which is on and fixed a *different*, earlier defect.

### Next discriminator

Two questions remain, and they are different questions:

1. **Is the ground plane meant to use texture 103 at all?** A strip that tiles
   31 times wants a small tileable road texture, not a sprite atlas. Trace
   every `glBindTexture` in the frames *before* the one being read back (the
   array and texture setup for these batches happens in an earlier frame, so a
   single-frame trace does not show it), and check whether the guest asked for
   a texture tapHLE never created or silently dropped.
2. **Why is each terrain chunk only a handful of pixels?** 228–480 indices
   should not paint 2 pixels. Read that batch's positions through
   `glGetPointerv` the way the ground plane's UVs were read here, and compare
   the predicted window bounding box with the measured one. If they agree, the
   guest is submitting collapsed chunk meshes and the fault is upstream of GL.

Do not resume the "geometry is off-screen" or the pre-2026-08-03
texture/combiner investigations. Both are closed.
