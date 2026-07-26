# Glass Tower HD compatibility work note

- Branch and starting commit: `compat/glass-tower-hd` from `14a63367`.
- Canonical artifact: `https://archive.org/details/iPadGames2010`, `Glass.Tower.HD-v1.0.ipa`; size 10,727,706 bytes.
- Embedded identity (`tapHLE --info`): display name `Glass Tower`, bundle `ru.idigger.Glass-Tower`, version `1.0`, minimum OS `3.2`, iPad.
- Options: `--force-composition` required to force CoreAnimation compositor layer rendering on Windows OpenGL drivers.

## Highest milestone: 3-star (In game)

Boots to title menu and renders game levels via EAGL context and forced composition.

## Checks run
- `cargo fmt --all -- --check`
- `cargo test --workspace --lib`
