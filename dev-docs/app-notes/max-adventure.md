# Max Adventure Free compatibility work note

- Branch: `compat/max-adventure`.
- Canonical artifact: <https://archive.org/details/app-id-233>,
  `MaxAdventureFree 1.2.ipa`, `source: original`; size 18,985,524 bytes.
- Hashes:
  - MD5: `c5fee449c86d3e693f6445a8f6e8c37f`
  - SHA-1: `630c4639b663ae8ed477c4e37817e95298a06479`
  - SHA-256: `043ea5c352a1ac151ff6002c0cc8510357243f699980a480a0d2e098c70ac610`
- Embedded identity (`tapHLE --info`): display `MaxAdventureFree`, bundle
  `com.imangi.maxadventurefree`, version `1.2`, minimum OS `3.0`, iPhone +
  iPad. The Archive item is named only `app-id-233`, so `--info` is the only
  source of this app's identity.
- tapHLEdb: App 27, version 27, report 35 (2026-07-26, tapHLE `afcc4cf5`,
  ★☆☆☆☆).

## Current state: 1-star, dies during load

The last line logged is:

```text
tapHLE::mach_o: Loading armv7 slice for "MaxAdventureFree"
```

and then the process dies. There is **no Rust panic message**, on stderr or in
`tapHLE_log.txt`, with `RUST_BACKTRACE=1` set. Under the visible harness the
window that appears is tapHLE's own "tapHLE crashed!" message box.

## This is the same signature as JungleZuma

`jungle-zuma.md` records an app that also stops right at Mach-O load with no
panic. Two different apps, from different publishers, failing at the same point
with the same absence of a Rust panic, is much more likely to be **one bug in
the Mach-O/dyld load path** than two coincidences.

Treat them as a single investigation. Fixing it would move two list entries at
once, which makes it better value than either app alone.

## What distinguishes it from an ordinary panic

An ordinary tapHLE failure prints a panic line and usually a guest register
dump. Neither appears here, so this is not a Rust `panic!`/`unwrap` — it is
either a native fault (segmentation fault, stack overflow) inside the loader, or
an abort that bypasses the panic hook. JungleZuma additionally *hangs* rather
than exiting when run in the foreground, which is consistent with an infinite
loop that eventually exhausts the stack.

## Next discriminator

Run one of the two under a debugger, or add a bounded iteration guard and a
`log_dbg!` per load command in `src/mach_o.rs`, and see which command it stops
on. That single observation should explain both apps. Do not add speculative
Objective-C surface for either of them until this is understood: nothing in
either app has run yet, so no missing selector can be the cause.
