# Omium (2 Player Shooter) compatibility work note

- Branch: `compat/omium`.
- Canonical artifact: <https://archive.org/download/ios-ipa-collection/>,
  `Omium.2.Player.Shooter.v1.0.ipa.ipa` (the doubled extension is the Archive
  filename), `source: original`; size 1,631,044 bytes.
- Hashes:
  - MD5: `914522510ea8e30d0c8d56bcb3c88c85`
  - SHA-1: `396a22fa9c342774f6d2bc6ecd2e5ee7d58ab2e0`
  - SHA-256: `e9bf9b9be42424d1b70e0f11e9b15ec499b1caf2a8cd4f525228e926966d33f8`
- Embedded identity (`tapHLE --info`): display `Omium`, bundle identifier
  **`com.eeenmachine.`** — with a trailing dot and no app segment. Version
  `1.0`, iPad. Same developer as Scoops (`com.eeenmachine.scoops`).
- tapHLEdb: App 25, version 25, report 33 (2026-07-26, tapHLE `9d6ee348`,
  ★★☆☆☆). Report 32 (app 24) is a **bad submission by the same agent** that
  guessed `com.mrlacey.omium`; it should be rejected in moderation.

## Highest milestone: 2-star (Starts / Menu), tapHLE `9d6ee348`

The menu renders and is live: the word "OMIUM" drawn entirely out of moving
point-sprite particles, over the entries Dodge, Juggle, Infinite and More
Games. Two captures seconds apart show the particle field in different
positions, so the app is animating, not frozen.

It launched with **no new fixes at all** — the point-sprite work done earlier
for Glass Tower 3 (`GL_POINT_SIZE_ARRAY_OES` reported as `GL_INVALID_ENUM`
instead of asserting, and `glPointSizePointerOES` accepted) is exactly what
this app needs, and both log on startup here.

## Frontier: touch input is vertically inverted relative to the display

Menu taps do not select, and a `TAPHLE_LOG_MODULES=tapHLE::frameworks::uikit::ui_touch`
run explains why. **The touch is delivered, but at the wrong place.** Two
measurements, on a 768x1024 client with a 768x1024 `EAGLView`:

| clicked client y | delivered to app |
|---|---|
| 631 | 393 |
| 200 | 824 |

Both satisfy `delivered_y = 1024 - client_y`. The y axis is **flipped**.

This is not a coordinate mistake in the click map: hit-testing finds the app's
`EAGLView` and `touchesBegan:`/`touchesEnded:` are both delivered, so routing
works. The point itself is mirrored, which is why tapping "Dodge" lands
somewhere near the title instead.

The display is **not** mirrored — the menu renders right way up, with "OMIUM"
above the entries and "More Games" at the bottom. So input and output disagree
about the vertical axis. That is an emulator bug, not an app quirk, and it
would affect any app tapHLE puts in the same orientation.

x could not be measured: both usable taps were at x=384, the exact horizontal
centre, where a flip is invisible. **Measure x before concluding this is a
simple vertical flip rather than a 180-degree rotation** — an iPad app whose
`Info.plist` allows `UIInterfaceOrientationPortraitUpsideDown` would plausibly
get a 180 rotation applied to input only, and that would look identical at
x=384.

### Next discriminator

One run, three clicks, all off-centre in x — say `(200, 300)`, `(600, 300)`,
`(200, 700)` — reading the delivered point from the `ui_touch` log each time.
If x also mirrors (`768 - x`), it is a 180-degree rotation; if x passes
through, it is a pure y flip. Then find where tapHLE derives the input
transform and compare it against the one used for presentation; they are
evidently not the same for this orientation.

Verifying the fix is cheap: with input and display agreeing, tapping "Dodge" at
`(384, 631)` should enter the mode, taking this app to 3 stars. Nothing else is
known to be wrong with it — it needed no fixes at all to render.

Since point sprites are drawn at a uniform size here (tapHLE does not model
`GL_POINT_SIZE_ARRAY_OES`), the particle title may also be visually wrong in a
way that matters to hit-testing if the app picks by particle proximity.
