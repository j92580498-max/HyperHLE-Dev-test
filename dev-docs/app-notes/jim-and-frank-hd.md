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

Traced with the retain applied as an experiment (not committed), the app *does*
build its hierarchy: `[UIWindow addSubview:]` is sent twice, `[EAGLView
addSubview:]` once, and both the window and the `CAEAGLLayer` get `setHidden:`.
So window-contains-view is established, and the window is not empty.

Two things came out of that run. **The first turned out not to be a second bug
at all**, which is worth spelling out because it changes the shape of the work:

**1. The nil `addSubview:` receiver *is* the dead window.** Tracing everything
and reading the messages either side of it gives the whole story in eight lines:

```text
[UIWindow dealloc]                                  <- nib objects unretained
[OdysseyAppDelegate applicationDidFinishLaunching:]
[UIApplication keyWindow]                           <- returns nil, it just died
[OdysseyAppController view]
[nil ((null)) addSubview:]                          <- keyWindow's nil result
```

The app asks `UIApplication` for the key window and adds its view to whatever
comes back. The window has already been deallocated, so it adds to nil. This is
not an independent missing reference — it is the same lifetime bug one step
later, and fixing the retain fixes both.

**2. The grey is a compositing conflict, not an empty window.** tapHLE presents
a fullscreen `CAEAGLLayer` directly (`find_fullscreen_eagl_layer`) *and*
composites the Core Animation tree. With the window dead, only the GL path drew
and the app looked correct. With the window alive, the window's layer draws over
the GL content. Retaining the nib objects is right on its own terms — UIKit's
ownership contract says so — but it cannot land until that conflict is resolved,
or it turns a rendering app into a grey one.

So there is **one** blocker, not two: retain the nib's top-level objects, and
resolve the compositing overlap that retaining exposes. The retain is correct on
its own terms and is deliberately absent from `trunk` only because, alone, it
turns a rendering app into a grey one.

### The compositing question, now measured

`find_fullscreen_eagl_layer` logs why it declines, and with the retain applied it
says the same thing every frame:

```text
Not a fullscreen EAGL layer: bounds 768x1024 (screen 768x1024),
  anchor (0.5,0.5), position (384,512), hidden false,
  opacity 0.5, identity transform false
```

Bounds, origin, anchor, position and visibility are all exactly right. **Two
tests fail: `opacity 0.5`, and a non-identity affine transform.**

The transform is the 90° rotation this app uses — it is a landscape game, which
is also why it renders sideways under the direct path. And that rejection is
already anticipated in the function itself:

```rust
// TODO: support affine transforms that result in a full-screen layer
//       (typical example is 90° rotation).
```

So the remaining work is that TODO, plus deciding what a half-opaque fullscreen
layer should mean. Neither is app-specific: any landscape app that gets its
window from a nib will land here, which is a large share of iPad titles.

### One more hypothesis, also wrong: composition does apply the rotation

It would be natural to conclude from the rejection above that the sideways
rendering is composition ignoring the layer's transform. It is not.
`CALayerHostObject::superlayer_to_layer_transform()` concatenates
`self.affine_transform`, and `composition.rs` multiplies that into its
cumulative transform for every layer. Rotation is handled.

So the sideways image is the app drawing landscape content, which is what it
does on a device held sideways — a window-orientation matter rather than a
rendering bug, and consistent with the earlier observation that
`--landscape-native` changes the window size while leaving the captured
renderbuffer byte-identical.

**Four hypotheses have now been tested on this app and three were wrong.** Each
was settled by one cheap measurement — a trace, a log line, or reading the
function being blamed. That ratio is the useful thing to carry forward: on this
app, measure first, because reasoning about it has a poor record.

### Summary of the whole chain

One bug and one gap, in order:

1. A nib's top-level objects are not retained, so the window dies. That causes
   both the discarded touches and the nil `addSubview:`.
2. Retaining it exposes the compositing path, which declines direct EAGL
   presentation for a rotated, half-opaque layer and composites a window over
   the GL content instead.

Fixing (1) without (2) trades a rendering app for a grey one, which is why the
retain is not on `trunk`.

This is still expected to be general: any app whose window comes from its main
nib rather than from code has the same lifetime problem.
