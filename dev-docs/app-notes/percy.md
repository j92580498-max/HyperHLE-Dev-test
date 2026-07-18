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

No Windows runtime milestone has been recorded. The exact Archive original is
content-hash verified, but the compatibility record intentionally has no
reports and renders as **Not tested**.

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

## Rejected hypotheses

- The Archive item is not mislabeled merely because its identifier contains
  `com.deluxe.pipes`: that is Percy's embedded bundle identity.
- A local cache filename is not evidence of artifact identity. Only the exact
  Archive association plus matching content hashes is accepted.

## Current diagnostics or code

None. No app-derived data, raw log, screenshot, or runtime diagnostic is in
the repository.

## Checks already run

- Exact Archive metadata, MD5, and SHA-1 verification.
- Local MD5, SHA-1, SHA-256, and embedded identity verification.
- Apple US availability lookups by exact app ID and bundle identifier.

## Known risks and next discriminator

No graphics, audio, input, saving, network, shutdown, or gameplay behavior is
known yet. Before the first run, repeat full Archive verification and
cross-check the same file with `tapHLE --info`. Then build a clean committed
`compat/percy` release binary and make one bounded Windows launch. The first
discriminator is whether the app reaches its lifecycle or stops earlier at a
loader, missing-symbol, or emulator boundary. Record only a sanitized summary;
do not add a compatibility report until an exact committed build reproduces a
Windows milestone.
