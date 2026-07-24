# SPYmouse HD compatibility work note

- Work branch; this investigation started from `compat/spy-mouse` at
  `fec8bcb1`.
- Local test artifact: `SPYmouseHD_v1.1.1_os32.ipa`; SHA-256
  `92b4712f4c15d17c83c237692126275519583568b115ddeeb69507669adaaeb5`.
- Artifact provenance: the maintainer confirmed this local IPA is the right
  target for this work. Its canonical Archive source URL, filename, and live
  metadata hashes are still not recorded, so do not submit a public
  compatibility report until that verification is complete.
- Runtime milestone: a Windows release build of this exact artifact clears
  KVC's `dictionaryWithValuesForKeys:`, custom `NSDictionary` subclass key
  enumeration, `NSAssertionHandler`, `NSException`, `NSMachPort`, input-port
  registration, `dladdr`, `CCHmac`, `NSData` formatting, and RFC-2396 percent
  escaping. It reaches the game's Origin/ad-data setup, executes its offline
  download and tracking failure handling, and requests locale information, but
  does not yet show a stable screen or gameplay loop.
- Current implementation: reusable Foundation KVC/dictionary, data, assertion,
  exception, Mach-port, and run-loop behavior; CommonCrypto HMAC-SHA1/MD5;
  and a standard failed `dladdr` lookup. The exception and input-port fallbacks
  are explicitly bounded until Objective-C unwinding and generic port delivery
  are implemented.
- Current frontier: `allValues` is missing on tapHLE's concrete immutable
  dictionary. Implement the same storage-backed behavior as `allKeys`, then
  rerun the same exact artifact before changing any rating.
- Checks passed: `cargo metadata --no-deps --format-version 1`,
  `cargo fmt --all -- --check`, `git diff --check`, and `cargo build --release`.
  The exact IPA was launched on Windows after the release build. `cargo test
  --workspace --lib` was attempted but failed before tests because its stale
  debug target refers to an unavailable former worktree and its wrappers cannot
  find Boost/CMake; the TestApp suite likewise remains unavailable without its
  custom SDK/LLVM.
- No compatibility-database report is due: no star threshold was reached, and
  the canonical public-artifact metadata has not yet been verified.
