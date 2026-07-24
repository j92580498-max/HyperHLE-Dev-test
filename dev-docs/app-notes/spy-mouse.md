# SPYmouse HD compatibility work note

- Work branch; this investigation started from `compat/spy-mouse` at
  `fec8bcb1`.
- Local test artifact: `SPYmouseHD_v1.1.1_os32.ipa`; SHA-256
  `92b4712f4c15d17c83c237692126275519583568b115ddeeb69507669adaaeb5`.
- Artifact provenance: the maintainer confirmed this local IPA is the right
  target for this work. Its canonical Archive source URL, filename, and live
  metadata hashes are still not recorded, so do not submit a public
  compatibility report until that verification is complete.
- Runtime milestone: a Windows release build of this exact artifact passes the
  prior `_CCCrypt`, `-[NSObject self]`, selector-typed `NSInvocation`,
  `-[NSInvocationOperation initWithInvocation:]`,
  `class_copyPropertyList`, and `+[NSObject superclass]` startup blockers.
  It reaches the game's Origin/ad-data setup, but does not yet show a stable
  screen or gameplay loop.
- Current implementation: CommonCrypto AES `CCCrypt`; offline CFNetwork and
  calendar support; Objective-C method/runtime refinements; Foundation
  invocation/operation support; and property-reflection fallback. The changes
  are intentionally reusable rather than game-specific.
- Current frontier: `-[AS_RequestData dictionaryWithValuesForKeys:]` is the
  first deterministic failure after the release rerun. Implement Foundation's
  KVC dictionary helper, then rerun the same exact artifact before changing
  any rating.
- Checks passed: `cargo fmt --all -- --check`, `git diff --check`,
  `cargo metadata --no-deps --format-version 1`, and `cargo build --release`.
  The TestApp suite was not run: its custom test SDK/LLVM is unavailable in
  this checkout, and the stale debug target points at an unavailable former
  worktree.
- No compatibility-database report is due: no star threshold was reached, and
  the canonical public-artifact metadata has not yet been verified.
