# Parachute Panic HD compatibility work note

- Branch and starting commit: `compat/parachute-panic-hd` from `186e18ab`.
- Canonical artifact: `https://archive.org/details/iOSObscura`, file
  `iPhoneOS 3/com.FDGMobileGamesGbR.ParachutePanic/ParaPanicHD-(com.FDGMobileGamesGbR.ParachutePanic)-1.9.1-(iOS_3.0)-2f707eb5e2c7ce80b3b76984a7ef46f9.ipa`;
  MD5 `2f707eb5e2c7ce80b3b76984a7ef46f9`, SHA-1
  `da09acc0f7018d1e2daeedb8d1eea831b97d750a`, SHA-256
  `6834b6c15a2b57f8ba51aa0599c0eadcf8deb531468de3911fae04f387613a31`,
  size 19,722,854 bytes. MD5/SHA-1/size match the live Archive metadata
  (source=original). Local Windows-safe filename `ParachutePanicHD 1.9.1.ipa`
  maps to the Archive filename above (use `--archive-filename` when reporting).
  A separate 1.6 build also exists in the item; the maintainer URL is 1.9.1.
- Embedded identity: display `ParaPanicHD`, bundle
  `com.FDGMobileGamesGbR.ParachutePanic`, version `1.9.1`, minimum OS `3.0`,
  universal iPhone/iPad.
- Availability: not re-checked against Apple lookup yet.

## Highest milestone

None yet (no screen). Startup gets deep into the app but crashes inside the
bundled **Scoreloop** SDK before any frame.

## Fixes made (general; graduate to trunk)

Each was the exact next startup blocker; all are reusable beyond this game:
1. `+[NSBundle bundleWithIdentifier:]` and `+[NSBundle bundleWithPath:]`.
2. `-[NSData subdataWithRange:]`.
3. CommonCrypto `CC_SHA224/256/384/512` (via the `sha2` crate).
4. Minimal KVO on NSObject: `addObserver:forKeyPath:options:context:` and the
   two `removeObserver:` forms (accepted, no notifications delivered).
5. `-[NSProcessInfo processIdentifier]` (reuses libc `getpid`).
6. Foundation/Security string constants that were unhandled non-lazy symbols and
   crashed on dereference: `NSHTTPCookieName/Value/Domain/Path`,
   `NSUnderlyingErrorKey`, and keychain keys `kSecReturnData/Attributes/`
   `PersistentRef`, `kSecValueData`.

## Frontier (2026-07-26): still no frame; parked as a poor value target

The keychain frontier below is **cleared**. Startup now gets past Scoreloop's
keychain query and several steps beyond it, but has still never drawn a frame.
Work here is paused by agreement with the maintainer in favour of the other
apps on the target list; everything found on the way is general and already on
`trunk`.

### Cleared since the keychain frontier

1. A real Security framework: the `kSec*` query constants plus working
   `SecItemAdd` / `SecItemCopyMatching` / `SecItemUpdate` / `SecItemDelete`
   over an in-memory generic-password store. (`744fa375`)
2. `-[NSOperationQueue setSuspended:]` / `-isSuspended`, with a pending list so
   operations added while suspended run in order on resume. (`744fa375`)
3. `NSNumberFormatter`, which did not exist at all. (`aa10bf46`)

### Why it is parked

Scoreloop subclasses `NSNumberFormatter` as `CreditsFormatter` and configures
it exhaustively. Each missing setter aborts the app, and each costs a full
release rebuild to discover:

`setCurrencyCode:` -> `setPositivePrefix:` -> `setMultiplier:` ->
`setPaddingPosition:` -> `setMinusSign:` -> ...

Enumerating the binary's `__objc_selrefs` up front (the technique that worked on
Glass Tower 3) did **not** bound this: `setMinusSign:` is reached through a
selector the sweep did not surface, so the real surface is larger than a static
list of formatter-shaped names predicts. The remaining set is the
`NSNumberFormatter` symbol properties — minus sign, plus sign, percent symbol,
zero symbol, nil symbol, exponent symbol, and so on.

That work is all shallow and mechanical, but it is a long tail, and behind it
sit further Scoreloop subsystems (networking, the request queue) before
anything renders. Six distinct subsystems have been implemented for this app so
far and it has not reached a first frame, which is a poor return compared with
apps that have not been tried at all.

### If resumed, do this

Implement the whole `NSNumberFormatter` symbol-property block in one pass —
`minusSign`, `plusSign`, `percentSymbol`, `perMillSymbol`, `zeroSymbol`,
`nilSymbol`, `notANumberSymbol`, `positiveInfinitySymbol`,
`negativeInfinitySymbol`, `exponentSymbol`, `currencyGroupingSeparator`,
`currencyDecimalSeparator`, `internationalCurrencySymbol` — as stored
properties, several of which genuinely affect output and should be wired into
`-stringFromNumber:` alongside the affixes already there. Only then resume the
crash-to-crash loop, and re-assess after the *next* subsystem boundary rather
than continuing indefinitely.

## Checks run

- Artifact hash verification vs live metadata: pass; SHA-256 recorded above.
- `cargo fmt --all -- --check`; `cargo test --workspace --lib`.
