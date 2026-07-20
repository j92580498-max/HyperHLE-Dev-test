# Baby Monkey compatibility work note

Last updated: 2026-07-19.

## Identity and source

- Work branch: `compat/baby-monkey`.
- Canonical Archive item:
  <https://archive.org/details/ios-ipa-com.kihon.babymonkey>.
- Maintainer-designated newest filename:
  `Baby Monkey (going backwards on a pig) (v1.3.5) [Decrypted].ipa`.
- Archive original size: 9,947,604 bytes.
- Archive MD5: `cf3eaa2326db3d2614b589f4e438312b`.
- Archive SHA-1: `02d70acdac7c8cd79640dab2336a1eaaf1382a5f`.
- Locally calculated SHA-256:
  `5ae6373838f9cff4a8a2001bc30253be88f0fb05f707af8c7bf8259330cd3346`.
- Embedded bundle: `com.kihon.babymonkey`, version/build `1.01`, minimum OS
  `4.2`.

Archive metadata was live-verified on 2026-07-19 and the downloaded bytes
match its exact original size, MD5, and SHA-1. `tapHLE --info` identifies the
embedded app version as `1.01`, not `1.3.5`. The Archive originals named
`v1.01`, `v1.2.3`, and `v1.3.5` are byte-for-byte identical. The local upload
manifest explains the mistake: all three destination names point to the same
`BabyMonkey (v1.01) [Decrypted].ipa` source. Keep the requested highest-version
Archive filename as the canonical source filename, but report the embedded
version as authoritative and do not claim that these bytes contain 1.3.5.

An earlier local file at the same picker path was 21,978,076 bytes with
SHA-256 `c1879dc8177f57ae0587847e2d0aadad264c0a4bf502bb77bdb74a6d3f654693`.
It did not match any decrypted Archive original. It is preserved with a
non-IPA extension under ignored `tapHLE_apps/_quarantine` and must not be run
or cited as compatibility evidence.

Availability was checked on 2026-07-19. Apple's public Lookup API returned no
result for App Store ID `447960108` in the United States and 23 other sampled
storefronts, and the former US product URL returned HTTP 404. Kihon's surviving
games page is historical and its app-specific site was unavailable. No current
official purchase or download route was found. This is a bounded availability
observation, not proof of legal abandonment or universal unavailability.

## Current canonical checkpoint

The release build for commit `ee21050a` was launched in a normal Windows
window with the exact hash-verified Archive bytes. It loads the armv7 slice,
selects landscape orientation, creates an OpenGL ES 1.1 context through the
GLES1-on-GL2 layer, creates the app's EAGL context, and obtains the host country
and preferred language. No app-rendered frame was established before it
exited.

The canonical stop was a null-page read at guest PC `0xc1010`. Static Mach-O
analysis proved that the preceding instructions load non-lazy slot `0xf4688`,
which maps to the unresolved `_NSLocaleLanguageCode` import. The containing
guest method is `-[CBNetwork makeRequest:params:]`; it calls
`[NSLocale currentLocale]` and then asks it for that key.

Commit `7dffbc97` exports `_NSLocaleLanguageCode` as an NSString constant and
returns the stored locale language for both the Foundation and Core Foundation
language-code keys. Its exact release build passed PC `0xc1010`, constructed
the Chartboost install/get requests, and reached Flurry startup. The language
symbol is no longer unresolved.

The next crash sends `setObject:forKey:` to an immutable
`_tapHLE_NSDictionary`. The app reached this state because
`+[NSDictionary dictionaryWithObject:forKey:]` is inherited by
`NSMutableDictionary` and returns `instancetype`, but tapHLE's implementation
forced allocation through the immutable NSDictionary concrete class. The
current branch allocates through the receiving class and shares the existing
key/value initialization path, so a call through NSMutableDictionary produces
the mutable concrete class. All 78 release library tests and
`cargo check --release` pass. This class-cluster correction still needs an exact
committed release build and visible canonical-IPA launch before the next
runtime frontier is claimed.

## Earlier noncanonical work

The branch also contains reusable emulator work developed while the mismatched
21.9 MB local file was mistakenly treated as the target. That work includes
Foundation/Core Foundation behavior, Objective-C blocks and initialization,
dispatch queues, native ES2 and iOS GL extension support, UIKit controller
mounting, standard-stream routing, `NSProcessInfo.environment`, and Darwin
`sigaltstack` behavior.

The implementations have their own automated checks, and the signal-stack
work received a focused ABI/semantics review. However, the old file's progress
through display-loop setup and `_sigaltstack` is not Baby Monkey compatibility
evidence. Revalidate relevant milestones against the canonical Archive bytes
instead of continuing from the discarded file's last log.

The native ES2 provenance recorded in older branch documentation was also too
imprecise: HyperHLE commit `ec06f12b` is a later tree snapshot, not the
originating ES2 change. The implementation first appears in HyperHLE commit
`d640dd4ddba1deb4c5eac9761239921bb3245601`, authored by Бусик and co-authored
by Devin AI. Preserve this correction prospectively without rewriting the
published tapHLE commit.

## Next discriminator

1. Commit the reviewed dictionary class-cluster correction.
2. Build that exact commit in release mode.
3. Recalculate the canonical IPA hash and launch it visibly from an isolated
   Windows run directory.
4. Confirm that the mutable dictionary no longer has immutable concrete class
   `_tapHLE_NSDictionary` and that `setObject:forKey:` succeeds.
5. Resolve only the next evidenced API, ABI, lifetime, or rendering boundary.

Do not use `--headless`: UIKit/EAGL startup needs a real tapHLE window and a
headless unwrap failure would be unrelated. Do not add a compatibility report
or star rating until a human-visible milestone is reproduced from a committed
build using these exact canonical bytes.

## Verification performed

- Archive metadata and the exact downloaded original agree on filename, size,
  MD5, and SHA-1; SHA-256 was calculated locally.
- `tapHLE ee21050a --info` reports version `1.01`, bundle
  `com.kihon.babymonkey`, minimum OS `4.2`, and iPhone device family.
- Static armv7 disassembly, indirect symbols, and runtime registers all agree
  on the `_NSLocaleLanguageCode` null dereference.
- The focused locale export test passes.
- The exact `7dffbc97` windowed run passes the locale dereference and reaches
  the dictionary class-cluster error described above.
- The release library test binary passes all 78 tests.
- `cargo check --release` passes with the existing GLES dead-code warning
  group.
- The full integration test remains unavailable because the reviewed custom
  test SDK and `tests/llvm/bin/clang.exe` are not installed; the failure is an
  environment prerequisite, not an app result.
