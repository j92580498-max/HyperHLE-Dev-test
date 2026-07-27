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
3. Difficulty -> `(238, 190)` Easy -> track list. **This is the frontier.**
4. Track list -> `(160, 120)` first track -> fails, see below.

## Track load is fixed; frontier is now UIGraphicsBeginImageContext

The nil-NSString crash is **solved**. The caller was
`-[NSDictionary initWithContentsOfFile:]`, which passed its path argument
straight to `to_rust_string()` with no nil check. Both concrete dictionary
classes had the same gap. A nil path now returns nil, as documented.

Finding it took one run, not a rebuild: `TAPHLE_TRACE_SELECTORS=all` and
reading the **last few traced messages before the panic**. That is the
technique to reach for when a backtrace only gives a call *shape*; guessing
which method matches the shape wasted two rebuilds here on
`stringByAppendingPathComponent:` and `stringByAppendingPathExtension:`, which
had the same signature, genuinely lacked nil guards, and were not the caller.
Those guards were kept — they are correct — but they fixed nothing.

Selecting a track now proceeds to a new blocker:

```text
Call to unimplemented function _UIGraphicsBeginImageContext
```

### Next step: the UIGraphics image-context family

tapHLE has **none** of it — no `UIGraphicsBeginImageContext`, no
`...WithOptions`, no `UIGraphicsGetImageFromCurrentImageContext`, no
`UIGraphicsEndImageContext`. This is a subsystem rather than a missing symbol:
it means creating an offscreen bitmap context, making it the current UIGraphics
context so ordinary drawing lands in it, and wrapping the result as a UIImage.

It is a good self-contained `feat/`, and it is likely to help well beyond this
app — compositing an image offscreen is a routine thing for a 2009 UI to do.
The pieces tapHLE already has (`CGBitmapContextCreate`, the UIGraphics context
stack, `UIImage`) are the ones needed, so this is assembly rather than new
graphics work.

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
