# Warlords HD compatibility work note

- Branch and starting commit: `compat/warlords-hd` from `3c6fc0bd`.
- Canonical artifact: `https://archive.org/details/ios-ipa-greyhoundgames.warlordshdapp`,
  `Warlords HD:Call To Arms (v3.065) [Decrypted].ipa`; MD5
  `c9275f6f440a8fad6bd72b4fc8905fc8`, SHA-1
  `3f74c1721a9ba98e4794f06062dca4111973b46c`, SHA-256
  `13646ccf0b14cf7c9c697a64eb132e80e481cd4568969ce4667bdddcd3692e73`.
  The Windows-safe local filename omits the canonical `:` and is explicitly
  mapped to this Archive filename.
- Embedded identity verified with `tapHLE --info`: display name `WarlordsHD`,
  bundle `greyhoundgames.warlordshdapp`, version `3.065`, minimum OS `3.2`,
  iPad device family. `Info.plist` declares `UISupportedInterfaceOrientations`
  = LandscapeLeft, LandscapeRight only; status bar hidden.
- Availability check (2026-07-25): Apple's US bundle-ID lookup returned no
  current listing for this exact build. This is a project-scope availability
  fact, not a legal conclusion.

## Highest milestone (dirty worktree, 2026-07-25)

The app boots to its **main menu** ("Warlords: Call to Arms v1.4" with Play /
Instructions / Options / Credits over the title art) and stays alive
indefinitely. Reached by a chain of small, general emulator additions (each was
the exact next hard blocker, in order):

1. `objc_msgSendSuper2_stret` — struct-return super2 dispatch (the WIP that
   started this branch).
2. `-[NSString rangeOfCharacterFromSet:options:range:]`.
3. `CFDictionaryGetValueIfPresent`.
4. `+[NSCharacterSet alphanumericCharacterSet]`.
5. `-[UIFont fontName]` / `familyName` / `pointSize` (UIFont now stores its
   name).
6. `CGFontCreateWithFontName` (substitutes a bundled Liberation face by name).

These are all reusable and graduate to `trunk`.

## Proven facts

- Rendering path is EAGL (OpenGL). The captured submitted renderbuffer is
  **768×1024** (iPad portrait), i.e. the CAEAGLLayer inherits the portrait
  `UIScreen` bounds.
- Warlords is a **landscape-native renderer**: it draws a 1024×768 landscape
  scene (viewport 1024 wide) into that 768×1024 portrait renderbuffer, so the
  raw buffer shows upright landscape art clipped on the right (1024→768) with a
  ~256px empty band on the long axis. tapHLE then applies its presentation
  rotation for `--landscape-left`, so on screen the menu appears rotated 90°
  and cut off.

## Rejected hypotheses

- "A different `--landscape-*` flag fixes the orientation." No — the clipping is
  a renderbuffer-size mismatch (768 wide vs the app's 1024-wide viewport); no
  presentation-side flag changes the drawable width.

## Next discriminator

The orientation/clipping is the frontier. tapHLE models landscape by keeping the
EAGL layer at portrait `UIScreen` bounds and rotating at presentation, which
assumes the guest draws portrait-native. Warlords instead relies on UIKit
rotating its full-screen view via a transform so the view's (and thus the EAGL
layer's) bounds are landscape 1024×768. tapHLE does not apply that view-transform
rotation to EAGL layer bounds. The fix is to give a landscape-only app's
full-screen EAGL layer landscape-native bounds and skip the presentation
rotation for it (with matching touch-coordinate mapping) — a real orientation
change to validate against known-good portrait-native games before landing.
Only after a correctly-oriented, uncut menu renders should input be driven
(Play → level/campaign select → a battle) toward a 3-star gameplay-loop result.

## Checks run (this session)

- `cargo fmt --all -- --check`: clean.
- `cargo test --workspace --lib` (debug and release): pass (86 + 4 tests).
- Live run: hash-verified IPA, release build, `--landscape-left`; frame capture
  of the main menu.
