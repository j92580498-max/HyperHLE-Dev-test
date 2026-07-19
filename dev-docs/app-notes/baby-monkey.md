# Baby Monkey compatibility work note

Last updated: 2026-07-19.

## Identity and source

- Work branch: `compat/baby-monkey`.
- Provisional Archive item:
  <https://archive.org/details/ios-ipa-com.kihon.babymonkey>.
- Tested filename:
  `Baby Monkey (going backwards on a pig) (v1.3.5) [Decrypted].ipa`.
- SHA-256:
  `c1879dc8177f57ae0587847e2d0aadad264c0a4bf502bb77bdb74a6d3f654693`.
- MD5: `d4f6a78d5e31722384ea4cddcd25acda`.
- SHA-1: `cbbcc206434347f6851b7b0aa0066e0bd7d9c6a3`.
- Size: 21,978,076 bytes.
- Bundle: `com.kihon.babymonkey`, version/build `1.3.5`, minimum OS
  `4.2`.

The maintainer supplied this newest known decrypted IPA from their local
upload inventory. The Archive identifier and filename are inferred from that
inventory, but Archive.org was unavailable during intake. Do not call this an
Archive-verified artifact and do not create a compatibility report until live
metadata confirms the exact original filename, MD5 and SHA-1. Keep the local
IPA in ignored `tapHLE_apps`; never add it to Git.

## Current checkpoint

Commit `6f4e44b2` adds reusable Objective-C Blocks runtime support and advances
the exact hash-checked IPA through its bundled Iddiction SDK and into Cocos2D
graphics initialization. It also implements the adjacent
`NSOperationQueue.maxConcurrentOperationCount` and
`+[NSObject instanceMethodForSelector:]` paths reached during startup.

The current stop is `-[EAGLContext initWithAPI:2]`. tapHLE only creates an
OpenGL ES 1.1 context, so it returns `nil` for the ES 2.0 request. The game then
makes a GL call without a current context and reaches a null-page memory error.
This is not yet a compatibility-database milestone or rating. No human menu,
input, graphics, gameplay or audio result has been established.

## Fixes that moved the frontier

- Synchronous `dispatch_once`.
- In-memory named and persistent UIKit pasteboards.
- `CFMakeCollectable` identity behavior.
- Dictionary sorting by guest values and dictionary construction from C
  arrays.
- Process-unique Foundation strings and date keyed-archive encoding.
- Deterministic `if_nametoindex` and an iPhone-style link-route `sysctl`
  response.
- Core Foundation URL percent escaping.
- `NSNumber` Objective-C type encodings and wide `CFNumberGetValue` output.
- Explicit signs in float formatting.
- ABI-compatible stack, global and heap Objective-C blocks, including copy,
  release, captured object/block handling and `__block` forwarding storage.
- Class initialization for compiler-created Objective-C-compatible objects.
- Stateful `NSOperationQueue` maximum concurrency accessors.
- Guest IMP lookup through `+[NSObject instanceMethodForSelector:]`.

## Next discriminator

Do not continue adding unrelated missing constants from the startup warning
list. The proven next dependency is a working ES 2.0 context plus guest shader
entry-point dispatch.

HyperHLE has a substantial ES 2.0/3.0 implementation, but it is not a small
drop-in patch. Start with the provenance and dependency review recorded in
`dev-docs/upstream-sync.md`. Port or adapt the smallest coherent ES 2.0 stack
on this branch, preserving tapHLE naming and Windows priorities. Re-run this
exact SHA-256 from a fresh temporary sandbox after every coherent graphics
checkpoint.

Do not use `--headless` for this test. Baby Monkey enters UIKit/window code
that requires a real tapHLE window, so headless mode creates an unrelated
window unwrap failure before the useful graphics frontier.

## Verification performed

- `cargo check --release` passed at commit `6f4e44b2`.
- The Blocks reference-count unit test passed before the final checkpoint;
  the final code only tightened ABI-compatible byref ownership afterward.
- A release build from the same change series launched the exact SHA-256 in a
  fresh Windows temporary sandbox and reached the ES 2.0 request described
  above.
- The custom guest TestApp was not rebuilt because its separate SDK toolchain
  was not available in this session. Its NSOperation source now covers the new
  queue accessors and guest instance-method lookup for the next full TestApp
  run.

Before claiming an exact committed runtime result, rebuild `6f4e44b2` and
repeat the fresh-sandbox launch. Before adding a database record, also complete
the live Archive verification and a human-visible playtest.
