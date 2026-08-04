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
  runs to a result. **Do not read that as three stars.** The world geometry is
  shredded into diagonal streaks and menu artwork is covered by opaque white
  blocks, so a person cannot actually see where the road is. Rating stays at
  **two stars** until the rendering is fixed; the loop persisting is a
  necessary part of three stars, not a sufficient one.

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

## The rendering frontier, measured (2026-08-03)

Everything before this section about textures, combiners, mip selection and
"collapsed" geometry was observed through the double-rotation presentation bug
and is worthless. This is measured with the frame reaching the window intact.

### What the defect actually is

During a race the road and terrain are absent and the car flies over empty
sky. A per-draw framebuffer readback of one race frame (17 draws) shows that
**seven of them change zero pixels**: the large world-geometry batches, with
index counts in the hundreds to a thousand. The small objects — the car, a
coin, the HUD — draw normally.

So the world geometry is submitted and rasterises nothing. It is not a
texture, blend, combiner or filtering problem; those affect what a fragment
looks like, and there are no fragments.

### Where it goes

Both matrices were read back at draw time and the transform worked through by
hand. The projection is orthographic and correct for the screen:

```
proj = [0.148 0 0 0; 0 0.222 0 0; 0 0 -0.013 0; 0 0 -1 1]
```

Half-width 1/0.148 = 6.76, half-height 1/0.222 = 4.50, ratio 1.5 = 480/320.

For a batch that **does** draw, the first vertex `(-57.392, 0.610, 53.854)`
under its modelview gives eye `(-1.05, -1.71, -45.54)`, hence NDC
`(-0.155, -0.379, -0.408)` — on screen, inside the clip volume.

For a batch that draws **nothing**, the first vertex `(-51.159, 0.233, 50.455)`
under its modelview gives eye `(-72.5, -2.62, -44.39)`, hence NDC x of
**-10.7**. It is off the left of the screen by an order of magnitude. Its y
and z are both in range, so only x is wrong.

Both modelviews are proper rotations (columns orthonormal, orthogonal to each
other), so neither matrix is corrupt. The geometry is simply being placed
where the camera cannot see it.

### What that rules in and out

- Not a tapHLE GL translation bug in texturing, blending, depth, culling or
  filtering: those cannot produce zero fragments from geometry that is
  off-screen for a legitimate reason.
- Not the buffer-mapping path. `glMapBufferOES` is **never called** by this
  app; a log placed in it during a full race produced nothing. (The
  read-seeding defect found there is real and was fixed, but it is not this.)
- Not VBOs. Every draw in the race uses client arrays: `vbuf=0 ibuf=0`, one
  shared scratch pointer that Unity rewrites between draws.
- Not `--landscape-native`, which is now on and fixed a *different*, earlier
  defect.

### Next discriminator

The maintainer observes that **the track renders for about one frame at the
start of a race** and then disappears. Combined with the measurement above,
that points at an offset between the world and the camera that is near zero at
race start and grows — the track is procedurally extended as the car drives,
and world coordinates were seen at x ≈ -51 in one run and x ≈ -79 in another,
so they do grow with distance.

Test that directly before changing any GL code: capture the modelview of one
world batch on the first three frames of a race and every ~30 frames after,
and plot its NDC x. If the error grows with distance driven, the fault is in a
value the guest computes from something tapHLE supplies — a timer, a
soft-float helper, or an accumulated transform — and the place to look is what
feeds Unity's per-frame delta and the `___fixdfdi`/`___floatdidf` family this
binary calls, not the GL layer. If the error is present and constant from the
first frame, look instead at how the camera's own transform is built.

Do not resume the pre-2026-08-03 texture/combiner investigation.
