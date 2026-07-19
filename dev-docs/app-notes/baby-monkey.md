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

The branch now passes the former ES 2.0 frontier and reaches the game's display
loop. Commit `2ddbfc29` supplies compiler stack-protector symbols, basic Grand
Central Dispatch queues, and useful register/stack diagnostics. Commit
`fd543d42` adds the reviewed native ES 2.0 slice and iOS GL extension entry
points. Commit `3fbcb0e9` adds `UIWindow.rootViewController` ownership/mounting
and `performSelector:onThread:withObject:waitUntilDone:` compatibility.

On Windows, the exact hash-checked IPA now creates two native OpenGL ES 2.0
contexts using the Intel Iris Xe driver. It proceeds through Cocos2D graphics
setup, mounts its root controller, performs its initial orientation decision,
and begins display-loop setup. With CADisplayLink and Caches directory mapped, the current stop is
`-[NSProcessInfo environment]`, which is not implemented.

This is still not a compatibility-database milestone or rating. No human menu,
presented frame, input, gameplay or audio result has been established.

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
- Compiler stack canary imports (`___stack_chk_guard` and
  `__stack_chk_fail`).
- Basic dispatch queue creation, main-queue lookup, inline async/sync block
  invocation, and dispatch-object release compatibility.
- Native Windows OpenGL ES 2.0 contexts, EAGL API 2 selection, shader/program
  calls, ES2 renderbuffer presentation, and native OES vertex arrays.
- Imported Apple multisample and discard entry points with conservative
  single-sample/no-op fallbacks when the Windows driver lacks those exact
  Apple extensions.
- Retained `UIWindow.rootViewController` mounting and immediate
  `performSelector:onThread:withObject:waitUntilDone:` compatibility.

## Next discriminator

Do not continue adding unrelated missing constants from the startup warning
list. Implement and test `-[NSProcessInfo environment]` which is where the game
currently crashes. Determine what environment variables the game expects from
this dictionary (e.g. system properties) to proceed further towards a presented frame.

The native ES 2.0 work was adapted from the smaller HyperHLE snapshot at
`ec06f12b886a166b220df94d44861a2de78299b3`, with authorship retained in the
port commit. This result supports the existing decision to review and port
coherent subsystems rather than switching tapHLE's base. It is not the whole
of HyperHLE's later ES 2.0/3.0 stack.

Do not use `--headless` for this test. Baby Monkey enters UIKit/window code
that requires a real tapHLE window, so headless mode creates an unrelated
window unwrap failure before the useful graphics frontier.

## Verification performed

- `cargo check --release` passes with one dead-code warning group for GLES
  trait methods that are not exported yet.
- A full Windows release build succeeded after each runtime-facing checkpoint.
- The exact SHA-256 was recalculated before every launch.
- Successive bounded windowed runs proved these former stops were passed:
  EAGL API 2 rejection, null stack-canary import, missing
  `_dispatch_queue_create`, missing `_glGenVertexArraysOES`, missing
  `-[UIWindow setRootViewController:]`, and missing
  `performSelector:onThread:withObject:waitUntilDone:`.
- The most recent run reached the `CADisplayLink timestamp` stop described
  above. Its raw log remains temporary evidence and is not committed.

Before claiming an exact committed runtime result, rebuild the current branch
tip and repeat the windowed launch. Before adding a database record, also
complete live Archive verification and a human-visible playtest.
