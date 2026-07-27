# JellyCar (1, 2, 3) compatibility work note

> **UNSUBMITTED RESULTS — tapHLEdb was returning HTTP 504 (2026-07-26).**
> Two results below were measured on committed builds but could not be filed:
>
> - **JellyCar 2 on iPad**, `com.disney.jellyCar2foriPad` 1.1 — **2 stars** on
>   tapHLE `d335f7bd`. Frontier: mode-select and Classic instruction cards
>   render; the cards are static, so gameplay is not reached.
> - **JellyCar 3**, `com.disney.JellyCar3` 1.2 — **1 star** on tapHLE
>   `f8a085d8`. Frontier: past resource enumeration, guest MemoryError.
>
> One earlier JellyCar 3 attempt used a *guessed* identifier
> (`com.walaber.jellycar3`) and also 504'd, so it may or may not exist as a
> stray row; if it does, reject it. Submit the two results above when the
> endpoint is healthy, and verify no duplicate was created first.


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

### JellyCar 3 was launched; it does not share this failure

JellyCar 3 1.2 needed `+[NSBundle pathsForResourcesOfType:inDirectory:]` and
its instance counterpart, both of which were missing (added on `trunk`). With
those it gets past resource enumeration and now faults in guest code with
`Error during CPU execution: MemoryError`, at a different point from JellyCar 1
and with no `scenes.xml` message.

**A hypothesis that was tested and failed:** `pathsForResourcesOfType:` looked
like the obvious way JellyCar 1 would build its scenes list from the bundle's
`.scene` files, so adding it might have fixed both. It did not — JellyCar 1's
behaviour is byte-for-byte unchanged. Whatever writes `scenes.xml` is
something else, so do not re-try that idea.

### JellyCar 2 reaches menus (2 stars)

**JellyCar 2 on iPad, bundle `com.disney.jellyCar2foriPad`, version 1.1**
(iPad, 1024x768). Verified with `tapHLE --info`; note the publisher is Disney,
not Walaber, and the same is true of JellyCar 3 (`com.disney.JellyCar3`).

Three general gaps had to be closed, all on `trunk` in `826f8144`:
`UIPushButton` (a pre-UIButton private class reached through an old nib), a
`UIButton initWithCoder:` assertion that a button must carry
`UIButtonStatefulContent`, and `UISlider`'s missing value/min/max.

With those it reaches the **mode-select screen** (Classic, Factory, Long Jump,
2P Tether, 2P Race, My Levels) and, from Classic, a series of **instruction
cards** showing a car, terrain, a goal flag and drive controls.

**Those cards are not gameplay.** Two captures taken five seconds apart are
byte-for-byte identical, and the timer on them (51.482, then 49.076 on the next
card) is static artwork, not a running clock. Tapping the continue arrow
advances from one card to the next. So this is a stable screen — 2 stars — and
the gameplay loop has not been reached.

Next discriminator: page through the remaining instruction cards to whatever
follows them, and check for a *changing* timer as the signal that the
simulation is actually running. Do not treat a card that merely depicts the
game as evidence of gameplay; that mistake was nearly made here.

JellyCar 3 1.2 is 1 star; see its own entry above.

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
