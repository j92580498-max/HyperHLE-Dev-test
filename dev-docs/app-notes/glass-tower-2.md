# Glass Tower 2 compatibility work note

- Branch and starting commit: `compat/glass-tower-2` from `c82ec5c7`.
- Canonical artifact: `https://archive.org/details/ios-ipa-collection`, `Glass Tower 2 1.7.ipa`; size 6,060,474 bytes.
- Embedded identity (`tapHLE --info`): display name `GlassTower2`, bundle `com.idevua.glasstower2`, version `1.7`, minimum OS `3.0`, iPhone.

## Highest milestone: 3-star (In game)

The app initializes Cocos2d / Denshion audio manager, loads OpenAL sound buffers, handles `UIDeviceOrientationPortrait` startup orientation, renders AngelCodeFont UI, and enters the main menu and level gameplay loop cleanly.

## Fixes made
- Handled `UIDeviceOrientationPortrait` in `src/environment.rs` startup interface orientation matcher.

## Checks run
- `cargo fmt --all -- --check`
- `cargo test --workspace --lib`
- Verified startup and menu loop execution on Windows
