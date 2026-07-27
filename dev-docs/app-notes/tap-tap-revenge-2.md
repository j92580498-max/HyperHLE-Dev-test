# Tap Tap Revenge 2 compatibility work note

- Branch: `compat/tap-tap-revenge-2`. Reusable fixes graduated to `trunk`.
- Canonical artifact: <https://archive.org/download/ios3-6-ipas/com.tapulous.taptaprevengeII.ipa>
  (identifier `ios3-6-ipas`), `source: original`; size 11,862,689 bytes.
- Hashes:
  - MD5: `d9d1bcdfec1eac422fb198ef5ddc98be`
  - SHA-1: `cce3134212a8d6ce8fe6f76c38395c8ed838a68c`
  - SHA-256: `1de81bcc507a333468df1d2884142654d5591f9f32b1b7f205322c00e0f22186`
- Embedded identity (`tapHLE --info`): display name `Tap Tap`, bundle
  `com.tapulous.taptaprevengeII`, version `2.6.4`, minimum OS `2.0`, iPhone.
  The Archive filename carries no version; `--info` is the only source for it.
- Options: none. Window is 320x480 portrait.
- tapHLEdb: App 13, version 13, report 21 (2026-07-26, tapHLE `102300c2`,
  ★★☆☆☆).

## Highest milestone: 2-star (Starts / Menu), tapHLE `102300c2`

Reproduced from a clean committed release build (window title
`Tap Tap (tapHLE 102300c2)`, no `-dirty`) against the hash-verified bytes.

Every menu works and is navigable: the animated title screen with Play / Free
Tracks / Options, the Play menu (One Player, Two Player, Career, Play Online),
difficulty select (Kids/Easy/Medium/Hard/Extreme), and the track list, which
renders album art, song titles, artists and durations for the two bundled
tracks. The welcome alert's text appears in the log.

**It is not in game.** Selecting a track fails, so no note chart is ever played.

### Click map

No launch options; window 320x480 portrait; wait ~28 s for the title screen
(this app does a lot of network and database setup first).

1. Title -> `(160, 313)` Play -> Play menu.
2. Play menu -> `(82, 200)` One Player -> difficulty select.
3. Difficulty -> `(238, 190)` Easy -> track list.
4. Track list -> `(170, 128)` first track -> the game loads and runs, but
   draws nothing. **This is the frontier.** This tap is timing-sensitive:
   allow ~14 s after the difficulty tap or it lands before the list is live,
   which looks identical to the tap being ignored.

## Track selection works; the game runs but does not draw

Selecting a track used to abort. It now loads the theme, parses the note chart
and starts the audio queue, and it stays alive indefinitely. **The gameplay
screen renders nothing** — the window stays white — so this is still 2 stars,
not 3.

Ten separate blockers were cleared to get from "selecting a track aborts" to
"the game runs". Each was a general gap, and none is specific to this app:

1. `-[NSDictionary initWithContentsOfFile:]` passed a nil path straight to
   `to_rust_string()`. Both concrete dictionary classes had the gap.
2. `UIGraphicsBeginImageContext` and friends — assembled from the existing
   `CGBitmapContext` and the UIGraphics context stack, not new drawing code.
3. `-[CALayer renderInContext:]`, deliberately partial: background colour and
   `contents` only, no transforms or masking, and it says so.
4. `-[NSMutableString initWithContentsOfFile:]` — the sibling-class trap again.
5. `CC_MD5_Init`/`_Update`/`_Final`.
6. `object_getInstanceVariable` and its setter.
7. Declared-property metadata: `class_getProperty` was a stub returning null.
8. Return values for `NSInvocation` — `-invoke` asserted the return type was
   void.
9. `AudioQueueGetCurrentTime`, `AudioQueueEnqueueBufferWithParameters`'s
   `outActualStartTime`, and `kAudioFilePropertyPacketToFrame`.
10. `CFRunLoopTimerCreate` asserted `order == 0`; `MPVolumeSettingsAlert*`;
    `-[NSCalendar components:fromDate:toDate:options:]`.

### The Lua bridge was the interesting one

The app's theme is Lua, and it failed with

```text
[string "theme.cfg"]:1044: attempt to compare function with number
Error setting up taps: no columns
```

Line 1044 of the app's own `game_defaults.cfg` (loaded under the chunk name
`theme.cfg`) reads `game.gameController.currentFrameRate < 16`. The bridge
resolves a property by calling `class_getProperty`, which tapHLE stubbed out to
return null, so it fell through to handing Lua a *bound method* instead of the
value — hence comparing a function with a number. With property metadata parsed
from the binary the chain `currentFrameRate` -> `gameView` -> `view` ->
`framesPerSecond` resolves and the theme loads clean.

The binary's imports named the mechanism before any tracing did:
`class_getProperty`, `property_getAttributes` and `object_getInstanceVariable`
together are a scripting bridge, and all three were missing or stubbed.

### A regression this session introduced, and the rule behind it

`-[UIViewController view]` was sending `viewDidLoad` whenever the view was
non-nil. That is wrong, and this app is what proved it: `TTRGameController`
builds its OpenGL view by hand and calls `-setView:`, and its `viewDidLoad` is
a **teardown** — it sent `unloadResources`, `removeFromSuperview` and
`setView:nil`, destroying the game view microseconds after it was created.

The rule is that `viewDidLoad` reports *loading*, so it belongs on exactly two
paths: after `-loadView` runs, and after a nib supplies the view. A controller
whose view the app assigned itself never loaded one and must not be told it
did. Glass Tower HD, which is what motivated sending `viewDidLoad` in the first
place, still reaches gameplay after the narrowing — that was verified, not
assumed.

## Current frontier: the EAGL view never binds a drawable

The game view is a standard `EAGLView`: `+[TTRGameView layerClass]` returns
`CAEAGLLayer`, the layer gets `setDrawableProperties:`, and
`-[TTRRenderer initWithContext:drawable:]` runs. But
`-[EAGLContext renderbufferStorage:fromDrawable:]` is **never called** —
confirmed by tracing that selector across a whole session and getting zero
hits — while `presentRenderbuffer:` is called 887 times, each one logging

```text
Can't present a renderbuffer 0 not bound to a drawable!
```

So the renderer is initialised, believes it has a surface, and presents every
frame into nothing. The app's own `createFramebuffer` selector is never sent
either, which places the failure inside `-[TTRRenderer initWithContext:
drawable:]`, on a path that gives up before creating the framebuffer without
logging anything.

Two contexts are created with `initWithAPI:` and there are 1777
`setCurrentContext:` calls, so a plausible line of enquiry is whether the
context that owns the drawable binding is the one current at present time —
tapHLE keys `renderbuffer_drawable_bindings` per `EAGLContextHostObject`, and
`initWithAPI:` with no sharegroup means two contexts share nothing.

That is a hypothesis, not a finding. The measured facts are the three above:
`renderbufferStorage:fromDrawable:` zero calls, `createFramebuffer` zero calls,
`presentRenderbuffer:` 887 calls with binding 0.

## Nine general gaps cleared to get here

All on `trunk`; none is specific to this game. In the order they were hit:

1. `-[NSBundle load]` and friends — a resource-only `.bundle` (FBConnect's
   images) has no code to load and is trivially loaded. (`3abf3073`)
2. `+[NSBundle bundleForClass:]`. (`3abf3073`)
3. `NSNetServiceBrowser` / `NSNetService` — no Bonjour, so both report the
   documented failure rather than leaving the app waiting. (`3abf3073`)
4. `-[NSString initWithBytesNoCopy:length:encoding:freeWhenDone:]`.
   (`3abf3073`)
5. `NSURLRequest`'s missing accessors and mutable setters. (`3abf3073`)
6. `-[UIApplication setNetworkActivityIndicatorVisible:]`. (`3abf3073`)
7. `NSSearchPathForDirectoriesInDomains()` asserted the user domain.
   (`3abf3073`)
8. `+[CATransaction setValue:forKey:]` unwrapped a missing implicit
   transaction. (`3abf3073`)
9. `NSSortDescriptor`, `NSCountedSet`, `NSData
   dataWithContentsOfFile:options:error:`, keyed-unarchiver `Data`/`Boolean`
   leaves, a real `UIAlertView` dismissal, optional `CAAnimationDelegate`
   methods, `CFDictionaryCreateMutable` capacity as a hint, and an unknown
   `AudioServicesPlaySystemSound` ID. (`d51d051b`)

### Two lessons worth carrying forward

**Sibling concrete classes do not inherit from each other.**
`_tapHLE_NSMutableArray` descends from `NSMutableArray`, not from
`_tapHLE_NSArray`, and the same holds for `_tapHLE_NSMutableString` vs
`_tapHLE_NSString`. Anything `NSArray`/`NSString` declares has to be added to
**both** concrete classes. This cost three separate rebuild cycles in this
session (`initWithObjects:`, `initWithBytesNoCopy:...`,
`sortedArrayUsingDescriptors:`) and also revealed that
`-sortedArrayUsingSelector:` had been missing from the mutable array all along.

**Optional delegate methods must be probed.** Sending a Cocoa delegate protocol
method blind turns a delegate that implements only the callbacks it cares about
into a "does not respond to selector" abort. `CAAnimation` was doing this.

## Known limitations at this rating

- No gameplay at all; the note chart never loads.
- Two Player, Career and Play Online were not tried. Online will not work:
  tapHLE has no network stack for Tapulous' services, and the app's own
  Bonjour-based local multiplayer is answered with "no network".
- The welcome alert is auto-dismissed via cancel rather than shown, so its
  "import your existing profile" branch is never taken.
- Audio was not measured. Two system-sound IDs are played that tapHLE never
  created, and are ignored.

## Checks run

- `cargo fmt --all -- --check`
- `cargo test --workspace --lib` (93 passed)
- `cargo build --release`
