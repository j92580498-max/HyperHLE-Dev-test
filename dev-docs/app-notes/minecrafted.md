# Minecrafted compatibility work note

- Branch: `compat/minecrafted`. Reusable fix graduated to `trunk`.
- Canonical artifact: <https://archive.org/details/ios-ipa-minecrafted>, file
  `Minecrafted (v1.0) [Decrypted].ipa`, `source: original`; size 2,448,264
  bytes. (The item's `[Cracked]` name is the same bytes, same MD5.)
- Hashes:
  - MD5: `3d8f33e06ca5c5c9ad0581d916aa9e51`
  - SHA-1: `e18acc141c3af1bbad5997879292b493479b4055`
  - SHA-256: `4a38a7ce1b84e94e17204e816cec3178c6695bfde428068bbd66e4aba9dd9edf`
- Embedded identity (`tapHLE --info`): display `Minecrafted`, bundle
  identifier **`Minecrafted`** (not a reverse-DNS name), internal name
  `Minecraft.app`, minimum OS `3.2`, iPhone + iPad.
- **Required device capabilities: `opengles-2`.**
- tapHLEdb: App 22, version 22, report 30 (2026-07-26, tapHLE `9d6ee348`,
  ★★☆☆☆).

  Report 29 (app 21) is a **bad submission by the same agent**: it used a
  guessed bundle identifier `com.mojang.minecrafted` and did not mention the
  GLES2 requirement. It should be rejected in moderation. Reports are immutable,
  so it was superseded rather than edited. The mistake was submitting before
  running `tapHLE --info`.

## Highest milestone: 2-star (Starts / Menu), tapHLE `9d6ee348`

The login screen renders and is stable: the pixel-art "MINECRAFTED" title on a
stone background, two text fields, green "Themes..." and red "Login..."
buttons, the "Login requires an official minecraft account" text, and a
"Go Private" button.

## The hard ceiling: this app needs OpenGL ES 2.0

`Info.plist` lists `opengles-2` under `UIRequiredDeviceCapabilities`, and
tapHLE says so at launch:

```text
Warning: app requires OpenGL ES 2.0+ support. Only OpenGL ES 1.1 is currently
supported.
```

**This app cannot reach 3 stars until tapHLE has a GLES2 backend.** That is a
subsystem, not an app fix, and it should not be attempted from this branch.
Anyone picking this app up should weigh that first: the login screen is
reachable and everything past it is blocked behind shader-based rendering.

## Fix this required

`_nib_archive_decoder`'s numeric decoders (`decodeBoolForKey:`,
`decodeFloatForKey:`, `decodeIntegerForKey:`) each demanded the exact stored
variant matching the selector and hit `unreachable!()` otherwise. Nib archives
store each value in whatever variant is smallest, so a float key routinely
arrives as an integer. Fixed in `22185b86`; that alone took the app from
aborting during nib load to its login screen.

## Secondary frontier

Tapping "Go Private" at `(419, 297)` in the 480x320 client aborts with a bare
`not yet implemented` (a `todo!()`), with no module in the message. If the
GLES2 question is ever resolved, find that `todo!()` first by running with
`RUST_BACKTRACE=1` **and** performing the click, which a headless run will not
do.

`File::Open(...) : WARNING! File "[DOC]MC/char.png" failed to open` is logged
at startup by the app's own file layer; it is not fatal.
