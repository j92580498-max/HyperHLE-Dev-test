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

## Highest milestone (2026-07-25)

The app boots to a **correct, unclipped landscape main menu** ("Warlords: Call
to Arms v1.4" with Play / Instructions / Options / Credits over the title art)
and stays alive indefinitely — a clean 2-star stable screen. It runs with
`--landscape-native` (wired into `tapHLE_default_options.txt` for
`greyhoundgames.warlordshdapp`). Reached by a chain of small, general emulator
additions (each was the exact next hard blocker, in order):

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

- Rendering path is EAGL (OpenGL), fullscreen fast path. Warlords is a
  **landscape-native renderer**: with the portrait-shaped screen it drew a
  1024×768 scene into a 768×1024 renderbuffer (right edge clipped, ~256px band
  on the long axis) and tapHLE then rotated it, so the menu came out sideways
  and cut off.
- With `--landscape-native` the emulated screen is landscape (1024×768), so the
  fullscreen EAGL layer/renderbuffer is 1024×768, the app draws into it 1:1, and
  it is presented with identity rotation. Captured renderbuffer is now 1024×768
  and the full menu renders correctly (title, both unit sprites, all four menu
  items, corner icon).
- Contrast with a portrait+rotation landscape game (Ricky): it presents a
  480×320 Core Animation frame via composition and is unchanged by this option.

## Rejected hypotheses

- "A different `--landscape-*` flag fixes it." No — the clip was a
  renderbuffer-size mismatch (768 wide vs the app's 1024-wide viewport); it took
  a landscape-shaped screen (`--landscape-native`), not a presentation-side flag.
- "`--force-composition` fixes it." No — it changes the present path but not the
  portrait drawable the app renders into, so the clip persists.

## Next discriminator

Drive input from the menu toward a 3-star gameplay loop: tap **Play**, advance
through any campaign/level select, and confirm a battle starts and persists.
Touch coordinates are now in the 1024×768 landscape space with identity
rotation. Verify the tap recipe and record it here before claiming 3 stars.

## Checks run (this session)

- `cargo fmt --all -- --check`: clean.
- `cargo test --workspace --lib` (debug and release): pass (86 + 4 tests).
- Live run: hash-verified IPA, release build, `--landscape-left`; frame capture
  of the main menu.
