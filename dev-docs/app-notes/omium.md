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

## Frontier: menu taps do not select

A tap at `(384, 631)` in the 768x1024 client — the centre of the "Dodge"
text — does not advance. The particle field keeps animating, so the app is
alive and the tap is simply not selecting.

Two hypotheses, untested:

1. **The coordinate is wrong.** The capture is 768x1024 and the client is
   768x1024, so they map 1:1, but the hit target may not be centred on the
   glyphs.
2. **Touches are not reaching the app's picker**, the same class of problem
   Glass Tower HD had before the `viewDidLoad` fix.

## Next discriminator

Distinguish those with one run: launch with
`TAPHLE_LOG_MODULES=tapHLE::frameworks::uikit::ui_touch` and tap. If the log
shows the touch found a view and was delivered, the coordinate or the app's own
hit test is at fault and a sweep of tap positions down the menu will find it;
if no view is found, it is a routing problem.

Since point sprites are drawn at a uniform size here (tapHLE does not model
`GL_POINT_SIZE_ARRAY_OES`), the particle title may also be visually wrong in a
way that matters to hit-testing if the app picks by particle proximity.
