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

## 2026-07-27: the touch problem narrowed to the main nib

The earlier note said the app "never creates a UIWindow" and suggested finding
what it creates instead. That framing was wrong in an instructive way: **the app
does not create a window because it does not have to.**

Established, in order:

1. The binary references `UIWindow` and `makeKeyAndVisible`, so it is not
   avoiding UIKit windows on principle.
2. Tracing shows it calls `[UIApplication keyWindow]` and
   `[UIApplication windows]` and **never** `makeKeyAndVisible`. It *queries* for
   a window rather than making one — which is correct iOS code for an app whose
   window comes from its main nib.
3. Its `Info.plist` has `NSMainNibFile = MainWindow` and the bundle ships
   `MainWindow.nib`. There is no `NSMainNibFile~ipad`, and
   `Bundle::main_nib_filename` falls back to the plain key correctly, so the
   right name is resolved.
4. `UIApplicationMain` does load the main nib, and nib machinery demonstrably
   runs — the log carries a `UIProxyObject` line from that load.
5. `-[UIWindow initWithFrame:]` **and** `-initWithCoder:` both register the new
   window in `ui_view.ui_window.windows`, so a window created by either route
   would be found by hit-testing.

### Answered: the window is created, then deallocated

That prediction was wrong too, and measuring it took one run. The nib logging
added to `ui_nib` reports:

```text
Nib instantiated 4 top-level object(s) (of 6 decoded):
  ["OdysseyAppDelegate", "UIWindow", "OdysseyAppController", "EAGLView"]
```

The window **is** produced. And the touch failure now reports the candidates:

```text
Couldn't find a window for touch at ..., discarding. Windows: []
```

Created and registered, then gone. Nothing retains the main nib's top-level
objects, so they are autoreleased and die at the first pool drain — taking the
window's registration with them via `-[UIWindow dealloc]`. UIKit's contract is
the opposite: a nib's top-level objects come back at +1 and the loader owns
them.

### Why the obvious fix is not committed

Retaining them in `UIApplicationMain` **does** fix touches — the discard message
disappears entirely. It also turns the screen grey: the nib's `UIWindow`, now
alive, composites over the `EAGLView` the app actually draws through, and the
window is empty.

So the app trades "renders correctly, ignores every tap" for "accepts taps,
draws nothing". That is not an improvement and it is not committed. The two
diagnostics that found all this are, since they are useful regardless.

### What the real fix has to reconcile

The `EAGLView` is itself a top-level object of the same nib, so the intended
arrangement is presumably window-contains-view. Establish why the two are not
connected — whether the nib's view hierarchy is being unarchived without its
parent-child links, or the window simply never has the view added — before
retaining anything. Retaining the objects without fixing that just makes an
empty window visible.

This is still expected to be general: any app whose window comes from its main
nib rather than from code has the same lifetime problem.
