# JellyCar (1, 2, 3) compatibility work note

Covers all three JellyCar entries on the target list, because they share one
blocker.

- Branch: `compat/jellycar`.
- Artifacts (all `source: original`, all hash-verified):
  - **JellyCar 1.5.4** — <https://archive.org/download/ios3-6-ipas/>,
    `JellyCar 1.5.4 (Decrypted).ipa`, 29,654,461 bytes,
    MD5 `ac69d3e8f4e656023bc7ab213d5d7b3e`,
    SHA-256 `6af50c478f8c44f2505cddbca041d293cd7f10e0dd46574e6e31e9faaf001803`.
    Bundle `com.walaber.jellycar`, version `1.5.4`.
  - **JellyCar 2 on iPad** — <https://archive.org/download/iPad-1-ipa/>,
    `JellyCar 2 on iPad.ipa`, 62,513,270 bytes,
    MD5 `6be32f8a6852cbd3f23e04088c3577ae`,
    SHA-256 `ea95c55c9ded365dcc18f8d498ce0b0b1660bf7766fd8b65a082ba0752c269de`.
  - **JellyCar 3 1.2** — <https://archive.org/download/ios3-6-ipas/>,
    `JellyCar3 1.2 (Decrypted).ipa`, 24,285,094 bytes,
    MD5 `e6de7e8d5d84140967abfa43175b7405`,
    SHA-256 `f2e43f0d678d1d70bf3521c88fe71607241d03303e18541146c1260d44e05d59`.
- tapHLEdb: JellyCar is app 23, version 23, report 31 (2026-07-26, tapHLE
  `9d6ee348`, ★☆☆☆☆). JellyCar 2 and 3 were not launched — see below.

## UPDATE: zlib landed; the blocker below is cleared

The gzip file API is implemented on `trunk` in `d6fde9d1` (gzopen, gzread,
gzwrite, gzclose, gzeof, gztell, gzrewind, gzseek, gzgetc, gzerror), so the
`gzopen` abort described below no longer happens.

**JellyCar 1.5.4 now runs further and fails elsewhere:**

```text
ERROR! no root element in bundle file:.../Documents/scenes.xml
...
Error during CPU execution: MemoryError
```

That message comes from the app's own XML parser, and it is **not** a
compression problem: the bundle contains no `.gz` files at all, only plain
`.scene` and `.softbody` files, so `scenes.xml` is something the app builds in
its Documents directory at first run. It is reading back an empty or malformed
one and then faulting.

Next discriminator: find where `scenes.xml` is written. Trace the file APIs
(`TAPHLE_LOG_MODULES=tapHLE::libc::posix_io,tapHLE::libc::stdio`) on a fresh run
directory and see whether the app writes it at all, writes it empty, or never
gets that far. Only if it writes through `gzwrite` is the new zlib code
implicated — and it should be checked against a real gzip file first, since
nothing has yet exercised the write path end to end.

JellyCar 2 and 3 are still unlaunched and should be tried now that zlib exists;
they may not share this second failure.

## Original (superseded) blocker: no zlib at all

JellyCar 1.5.4 aborts during startup:

```text
Call to unimplemented function _gzopen
```

## The blocker is zlib, and tapHLE has none

`gzopen` is zlib's gzip *file* API. This is not a single missing function:
**tapHLE has no zlib support at all.** There is no `inflate`, no `uncompress`,
and no compression crate in `Cargo.toml` — nothing to build the gz* layer on.

The game reads its level data through it, so nothing renders without it.

JellyCar 2 and 3 were deliberately **not** launched after this was found. They
are the same developer and engine lineage, so they almost certainly need the
same thing, and spending three launches to confirm one blocker three times is
not worth it. Launch them immediately after zlib lands, not before.

## Next step: a bounded zlib file layer

This is a good self-contained `feat/`, and it unblocks up to three apps on the
target list at once:

1. Add a compression crate (`flate2` is the obvious choice and already builds
   on Windows) to `Cargo.toml`.
2. Implement the gz* file API in `libc`: `gzopen`, `gzread`, `gzwrite`,
   `gzclose`, `gzeof`, `gzseek`, `gztell`, `gzerror`. These must go through
   `env.fs`, not the host filesystem, so the guest sandbox still applies —
   that is the part to get right, and the reason this cannot just wrap
   `flate2`'s file helpers directly.
3. Only then relaunch JellyCar 1, and immediately after it JellyCar 2 and 3.

The raw `inflate`/`deflate` stream API is a separate question; do not implement
it speculatively. Check whether the app references those symbols at all before
deciding, since only the gz* file path is known to be needed.
