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

## Frontier: Scoreloop needs a Security/Keychain implementation

Scoreloop builds a **Keychain query dictionary** and repeatedly derefs the next
still-null `kSec*` constant at the same guest PC (`0xcb18e`, a `ldr r3,[r2]`
after loading a null non-lazy symbol). Remaining unhandled at last run:
`kSecMatchItemList`, `kSecMatchLimit`, `kSecMatchLimitOne`, and
`kABPersonEmailProperty` (AddressBook). Adding the constants only advances to
the next; once the query dict is complete the app will call
`SecItemCopyMatching` / `SecItemAdd` etc., which tapHLE does not implement.

## Next discriminator

Implement enough Security framework for Scoreloop: the remaining `kSec*`
constants (`kSecClass`, `kSecClassGenericPassword`, `kSecAttrService`,
`kSecAttrAccount`, `kSecMatch*`, ...) and the `SecItem*` functions, returning a
graceful "item not found" so the SDK proceeds offline. Then re-check the next
blocker (likely more Scoreloop networking). Only after a frame renders should
gameplay be driven. This is a self-contained Security/Keychain subsystem, a good
scoped `feat/` if pursued; the reusable startup fixes above already graduate to
trunk regardless.

## Checks run

- Artifact hash verification vs live metadata: pass; SHA-256 recorded above.
- `cargo fmt --all -- --check`; `cargo test --workspace --lib`.
