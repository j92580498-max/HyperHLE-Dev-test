# Glass Tower 2 compatibility work note

- Branch and starting commit: `compat/glass-tower-2` from `c82ec5c7`.
- Canonical artifact: `https://archive.org/details/ios-ipa-collection`, `Glass Tower 2 1.7.ipa`; size 6,060,474 bytes.
- Hashes:
  - MD5: `33643ef687ff93edb5e3647224280a14`
  - SHA-1: `3d6080e91a1ef4cf260c4db8e81faa90b25aae3e`
  - SHA-256: `5383f5eab7a54f6984bbb0797e1b295952371a7981f3132d54699e2e5a9df421`
- Embedded identity (`tapHLE --info`): display name `GlassTower2`, bundle `com.idevua.glasstower2`, version `1.7`, minimum OS `3.0`, iPhone.

## Highest milestone: 2-star (Starts / Menu)

The app initializes Cocos2d / Denshion audio manager, loads OpenAL sound buffers, handles `UIDeviceOrientationPortrait` startup orientation, renders AngelCodeFont UI, and enters the main menu. Touch/click inputs on the main menu do not yet advance into gameplay.

## Proven facts
- Renders main menu cleanly with `UIDeviceOrientationPortrait`.
- Menu touch/click input is currently unhandled or blocked from starting a level.

## Next discriminator
- Diagnose touch event routing / gesture recognizer in Cocos2d layer so clicking menu buttons enters the level loop.

## Fixes made
- Handled `UIDeviceOrientationPortrait` in `src/environment.rs` startup interface orientation matcher.

## Checks run
- `cargo fmt --all -- --check`
- `cargo test --workspace --lib`
- Verified startup and menu execution on Windows
