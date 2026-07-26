# Glass Tower HD compatibility work note

- Branch and starting commit: `compat/glass-tower-hd` from `14a63367`.
- Canonical artifact: `https://archive.org/details/iPadGames2010`, `Glass.Tower.HD-v1.0.ipa`; size 10,727,706 bytes.
- Hashes:
  - MD5: `cafb203ca6720949f4dbbb4e2010bed3`
  - SHA-1: `f3acfe7922dc42e9e5507a107ab1f6d0d37d3c63`
  - SHA-256: `80589a0a7d48117641a6badad06300907d1349c42396561ce46bab9954cfe879`
- Embedded identity (`tapHLE --info`): display name `Glass Tower`, bundle `ru.idigger.Glass-Tower`, version `1.0`, minimum OS `3.2`, iPad.
- Options: `--force-composition` required to force CoreAnimation compositor layer rendering on Windows OpenGL drivers.
- tapHLEdb Record: App ID 10, Report ID 16, rated ★★★☆☆ (3/5 - In game).

## Highest milestone: 3-star (In game)

Boots to title menu and renders game levels via EAGL context and forced composition.

## Checks run
- `cargo fmt --all -- --check`
- `cargo test --workspace --lib`
