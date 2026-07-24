# Snappers compatibility work note

- Branch and starting commit: `compat/snappers` from `b315e827`.
- Canonical artifact: `https://archive.org/details/ios-ipa-ru.emergingmobile.snappers`,
  `Snappers (v1.08) [Decrypted].ipa`; MD5
  `d831248dc1c358a8ad65c197ed2b43a9`, SHA-1
  `273938ce60108d5424b19cd04c90c59aa73b40c2`, SHA-256
  `69ade8efac1958a2259ff8cebc5e79e2953606409a1787307ca62bb4dafeafc6`.
- Embedded identity verified with `tapHLE --info`: display name `Snappers`,
  bundle `ru.emergingmobile.snappers`, version `1.08`, minimum OS `3.1`.
- Availability check (2026-07-24): Apple’s US bundle-ID lookup returned no
  current listing for this exact build. This is a project-scope availability
  fact, not a legal conclusion.
- Windows evidence on an uncommitted release build (2026-07-24): a fresh
  launch reached the menu, tutorial, Level Select, and Level 1. Tapping the
  Level 1 target reduced `Taps left` from 1 to 0 and displayed
  `Completed! Score: 50`; the process remained active for at least eight
  seconds after the tap.
- Reusable paths added during that run: missing Objective-C method signature
  queries return `nil`; `ExtAudioFile` exposes file frame length; mutable
  strings and arrays implement the collection mutations the game uses;
  `NSValue` supports non-retained object wrappers; `NSURL` resolves a relative
  string against a base URL; and run-loop dispatch discards an unused object
  argument for zero-argument selectors.
- Highest clean committed milestone: pending a rerun of the exact commit.
- Next discriminator: rerun the verified artifact on the committed revision,
  then submit the 3-star report and merge the reusable fixes to `trunk`.
