# Percy compatibility work note

Last updated: 2026-07-18.

## Identity and branch

- Work branch: `compat/percy`.
- Canonical item: `https://archive.org/details/ios-ipa-com.deluxe.pipes`.
- Exact Archive filename selected for testing:
  `Percy Jackson & The Olympians: The Lightning Thief (v1.0) [Decrypted].ipa`.
- MD5: `84a94d85e26af00b4594d7bba03cae77`.
- SHA-1: `d6860631a08b4909dd93dc11b50fb22a49ac2809`.
- SHA-256:
  `67a4981113d76f796fd706be321db92d2ca376e3ec83c333478cc8fc63f680ca`.
- Bundle: `com.deluxe.Pipes`, version `1.0`, minimum OS `3.1.2`.
- The `Pipes` bundle identity belongs to Percy; it is not a different target.
- On Windows, pass the exact Archive filename through `--archive-filename`
  when verifying a local cache whose filesystem-safe name cannot contain the
  Archive filename's colon. Do not rely on the local filename as identity.

## Highest clean committed milestone

Commit `1ae13a3460d1f59a29ce0cf3dc8b131308189865` exports the two
archived navigation classes needed by Percy's main NIB. Its exact release
binary has SHA-256
`021bfded3645d290aaed1d17f324b5bb46aa498c0f065e507ba579a1650849df`.
After a fresh full Archive verification, Percy completed NIB class resolution,
initialized its audio session, created a 480x320 window, and stayed alive for
a controlled 17-second observation. It still displayed no usable content and
is recorded as **★☆☆☆☆ (1/5) Broken (app booted)**.

## Proven facts

- Archive.org metadata lists the selected filename as an original file with
  the MD5 and SHA-1 above.
- The local artifact matched both published hashes and produced the SHA-256
  above.
- Its embedded `Info.plist` identifies `com.deluxe.Pipes`, version `1.0`, with
  minimum OS `3.1.2`.
- Apple's US lookups for app ID `356468446` and bundle identifier
  `com.deluxe.Pipes` each returned zero results on 2026-07-18. This supports
  the maintainer's project-scope decision; it is not a legal conclusion.
- The clean baseline at `4a2476c0` panicked while keyed-unarchiving
  `MainWindow.nib` because `UINavigationBar` was not exported.
- `UINavigationBar` can inherit the existing `UIView` allocation and NIB coder
  path. `UINavigationItem` needs a bounded `initWithCoder:` because `NSObject`
  does not provide one.
- The exact `1ae13a34` build crossed both class boundaries, reached app setup,
  and accepted a process-owned close request without a panic or forced stop.
- No guest EAGL frame was submitted. The navigation controller instead logged
  that it created an empty plain `UIView`.

## Rejected hypotheses

- The Archive item is not mislabeled merely because its identifier contains
  `com.deluxe.pipes`: that is Percy's embedded bundle identity.
- A local cache filename is not evidence of artifact identity. Only the exact
  Archive association plus matching content hashes is accepted.
- The loader and GLES surface were not the first startup blocker. The exact
  baseline reached CPU execution and created the GLES1-on-GL2 context before
  failing at navigation-class resolution.

## Current diagnostics or code

The implementation checkpoint is committed and pushed. There are no
uncommitted diagnostics. Raw logs and both attempted frame captures remain
outside the repository in unique temporary run directories.

## Checks already run

- Exact Archive metadata, MD5, and SHA-1 verification.
- Local MD5, SHA-1, SHA-256, and embedded identity verification.
- Apple US availability lookups by exact app ID and bundle identifier.
- `cargo fmt --all -- --check`.
- `cargo test --workspace --lib`: all 47 workspace library tests passed.
- `cargo clippy --workspace --lib -- -D warnings`.
- Clean release build from exact commit `1ae13a34`.
- Full Archive verification immediately before the exact-commit Windows run.
- Two bounded exact-commit runs from fresh sandboxes: no panic, no forced
  termination, and the same empty navigation-controller milestone.

The broader `cargo clippy --workspace --all-targets -- -D warnings` check is
currently blocked by six pre-existing warnings in the integration harness and
an `NSString` test-module layout. None is in the Percy change.

## Known risks and next discriminator

The next blocker is navigation-controller archive restoration. Percy's main
NIB records a view-controller stack, child controllers, a hidden navigation
bar, and a parent-controller relationship, but tapHLE's
`UINavigationController` currently inherits the generic `UIViewController`
coder and discards those relationships. The next focused implementation should
decode and retain that graph without firing view lifecycle callbacks while the
NIB is still recursively unarchiving. The next discriminator is whether the
restored root controller submits the first guest frame or reveals a narrower
missing API.

Audio effects, input, saving, networking, and gameplay remain unknown. Static
inspection suggests AIFF decoding and optional photo/gallery APIs may be later
gaps, but neither should be implemented before the navigation frontier moves.
