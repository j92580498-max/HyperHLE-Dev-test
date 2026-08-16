# Flight Control HD compatibility work note

- Branch and starting commit: `compat/flight-control-hd` from `84b2f387`.
- Canonical artifact: `https://archive.org/details/iPad-1-ipa`,
  `Flight Control HD 1.07.ipa`; MD5 `ab7fce73c48fc491330983979e9dc43b`, SHA-1
  `0a824bbbb9c1021f89ee54d1b2aa0e3719451a03`, SHA-256
  `eb3387b86eddcf078d3d1c382172565750c14ff08a7c00599976b0e3c52fa1ed`,
  size 18,814,789 bytes. MD5/SHA-1/size match the live Archive metadata
  (source=original). Filename is Windows-safe as-is.
- Embedded identity (`tapHLE --info`): display name `FlightCtrl HD`, bundle
  `com.firemint.flightcontrolipad`, version `1.07`, minimum OS `3.2`, iPad.
  `Info.plist` supports all four orientations; status bar hidden. Runs portrait
  by default and renders correctly, so no orientation option is needed.
- Availability check (2026-07-25): not re-confirmed against Apple lookup; do so
  before treating the DB report as final.

## Highest milestone (2026-07-25): 3-star gameplay loop

The app boots to its full main menu (High Score, stats tabs, map select, PLAY!,
Multiplayer) and, on tapping PLAY!, enters live gameplay: the island map with
runways renders, the HUD shows Aircraft Landed / Hi Score, and aircraft spawn
and fly across the map (two captures seconds apart show planes moving with drawn
flight paths). The gameplay loop starts and persists — a 3-star result.

Two general fixes were needed (both graduate to trunk):
- `-[UIViewController interfaceOrientation]` (returns the window's current
  orientation, mirroring `-[UIApplication statusBarOrientation]`). The app
  queried it during startup.
- Foundation constant `NSFileOwnerAccountName` (plus `NSFileGroupOwnerAccountName`)
  was unexported, so its non-lazy symbol pointer was null and the app crashed
  dereferencing it while reading file attributes (null-page access at PC 0x4416,
  a `ldr r2,[r0]` where r0 was the null constant).

## Click map

- Launch: no options (portrait); window 768×1024; ~16 s menu-load wait.
- `main menu -> client (185,830) [PLAY!] -> live gameplay (island map, HUD,
  flying aircraft)`.

## Proven facts

- Renders portrait-native (768×1024); the menu is a fullscreen EAGL layer, the
  in-game view presents through the Core Animation compositor.
- The game logs many failed `open()` probes for optional files during startup;
  they do not block startup or gameplay.

## Next discriminator

3 stars reached. Beyond it is human-tested (4+): land aircraft by drawing paths
(needs a tap-drag, which the current tap harness does not do), survive to a
score, and check pause/resume, map switching, and audio.

## Checks run

- Artifact hash verification (MD5/SHA-1/size vs live metadata): pass; SHA-256
  recorded above.
- `cargo fmt --all -- --check`; `cargo test --workspace --lib`.

## 2026-07-27: 3 stars re-confirmed on tapHLE `87acd74a`

Re-driven on a clean committed build after a session of broad UIKit changes.
`PLAY!` at `(185, 830)` enters live gameplay: the island map with its runways
and helipads, an aircraft in flight, the `AIRCRAFT LANDED` and `HI SCORE`
readouts, and the pause and fast-forward controls. Two captures ten seconds
apart differ by SHA-256, so the loop is running.

### A false alarm worth recording

An earlier check in the same session reported that this app "renders but taps
produced no change", which read as a possible regression. It was not. The taps
were at `(384, 512)` — the middle of the screen, chosen without reading this
note — and `PLAY!` is at `(185, 830)`.

The lesson is narrow and practical: **read the app's click map before
concluding anything from a tap that did nothing.** A blind tap landing on
nothing is indistinguishable from broken input, and this note has had the right
coordinates in it the whole time.
