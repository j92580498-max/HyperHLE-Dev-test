# The Jim and Frank Mysteries HD compatibility work note

- Branch: `compat/jim-and-frank-hd`. Reusable fixes graduated to `trunk`.
- Canonical artifact:
  <https://archive.org/details/ios-ipa-com.chillingo.thejimandfrankmysterieshd>,
  file `J & F HD (v1.1) [Cracked].ipa`, `source: original`; size 398,940,007
  bytes. The item lists a second name for the same bytes (the full
  "The Jim and Frank Mysteries ... HD (v1.1) [Decrypted].ipa"); the short name
  was chosen only because it is ASCII-safe on Windows. Same MD5.
- Hashes:
  - MD5: `8c20da73edbbd7d7d2114b1a8ff2ac6a`
  - SHA-1: `543f59387288315eacc46620116df06a5720d057`
- Embedded identity: bundle `com.chillingo.thejimandfrankmysterieshd`,
  version `1.1`.
- tapHLEdb: App 20, version 20, report 28 (2026-07-26, tapHLE `4e246384`,
  ★☆☆☆☆).

## Highest milestone: 2-star (Starts / Menu), tapHLE `21f655f4`

It renders. The full main menu — Play Game, Select Profile, Options, Extras,
About, on wooden planks over the painted title art — is drawn on a clean
committed build (`J & F HD (tapHLE 21f655f4)`, no `-dirty`). Filed as report 50.

`NSDecimalNumber` is what unblocked it: the class did not exist and startup
aborted on `+decimalNumberWithString:`. It is on `trunk`, backed by a `double`
with the precision limitation stated in its own doc comment.

## Two separate problems block three stars

### 1. The scene is drawn rotated 90 degrees

Everything renders, sideways. `--landscape-native` changes the window from
768x1024 to 1024x768 but **does not** change the rendering — the captured
renderbuffer is byte-identical with and without it (1,493,279 bytes both ways).
So whatever that option does, it does not reach this app's path, and the
sideways output is not simply "the option was not passed".

### 2. Touches are discarded: the app never creates a UIWindow

```text
Couldn't find a window for touch at CGPoint { x: 110.0, y: 170.0 }, discarding
```

`ui_touch` walks `ui_view::ui_window::windows` and finds none containing the
point. Tracing `makeKeyAndVisible`, `setFrame:`, `initWithFrame:` and
`pointInside:withEvent:` across a whole startup produced **no UIWindow messages
at all**, so the list is not merely mis-positioned — nothing was ever added to
it.

That is the more interesting of the two, and it should be settled before the
rotation: an app that renders through EAGL without a UIWindow still receives
touches on a device, so tapHLE is missing whatever creates or registers that
window. Find what the app *does* create in place of one — trace class
allocation during startup and look for its own window or view-controller class
— before assuming this is a UIKit gap.

Menu geometry, if it becomes useful: in the 768x1024 capture, "Play Game" sits
at about `(110, 170)`, with the other planks to its right along the top edge.
Both that point and its landscape rotation `(853, 110)` were discarded.
