# SPYmouse HD compatibility work note

- Work branch; this investigation started from `compat/spy-mouse` at
  `fec8bcb1`.
- Local test artifact: `SPYmouseHD_v1.1.1_os32.ipa`; SHA-256
  `92b4712f4c15d17c83c237692126275519583568b115ddeeb69507669adaaeb5`.
- Artifact provenance: the maintainer confirmed this local IPA is the right
  target for this work. Its canonical Archive source URL, filename, and live
  metadata hashes are still not recorded, so do not submit a public
  compatibility report until that verification is complete.
- Runtime milestone: on Windows, the `bddd7c91` release build of this exact
  artifact clears KVC's `dictionaryWithValuesForKeys:`, custom `NSDictionary`
  subclass key enumeration, `NSAssertionHandler`, `NSException`, `NSMachPort`,
  input-port registration, `dladdr`, `CCHmac`, `NSData` formatting, RFC-2396
  percent escaping, immutable dictionary values, guest `statfs`, and
  `CTTelephonyNetworkInfo`. It executes the game's offline ad/tracking failure
  handling, initializes its tracking database, displays the SPYmouse splash,
  and enters an active level after a tap. The level's HUD, mouse, cheese, and
  board remained visible and the process remained responsive 20 seconds later.
- Rating evidence: this is a three-star agent result: a gameplay loop starts
  and persists on the confirmed local IPA. The online-ad requests still fail
  offline as expected, and the runtime logs repeated missing-file warnings,
  but neither prevented the level from running.
- Current implementation: reusable Foundation KVC/dictionary, data, assertion,
  exception, Mach-port, and run-loop behavior; CommonCrypto HMAC-SHA1/MD5;
  a standard failed `dladdr` lookup; and a CoreTelephony device-information
  fallback which reports the documented no-cellular-provider result. The
  exception and input-port fallbacks are explicitly bounded until Objective-C
  unwinding and generic port delivery are implemented.
- Current frontier: the three-star milestone is complete. A human may assess
  the game for four stars; do not change this agent rating without a new exact
  artifact run.
- Checks passed: `cargo metadata --no-deps --format-version 1`,
  `cargo fmt --all -- --check`, `git diff --check`, and `cargo build --release`.
  The exact IPA was launched on Windows after that release build and reached a
  persistent active level. `cargo test --workspace --lib` was attempted but
  failed before tests because the debug target refers to an unavailable former
  worktree, Boost is absent, and CMake is not on the debug-build PATH. The
  TestApp suite likewise remains unavailable without its custom SDK/LLVM.
- A compatibility-database report is required by the new rating but cannot yet
  be submitted: the canonical Archive URL, filename, and live metadata hashes
  for the local IPA have not been verified. Once they are supplied and checked,
  submit one three-star report for `bddd7c91` with `source_type: agent`.
