# Scoops compatibility work note

- Branch: `compat/scoops`. Reusable fixes graduated to `trunk`.
- Canonical artifact:
  <https://archive.org/download/com.fossilsoftware.appzilla-ios4.0-clutch-2.0.4/>,
  `com.eeenmachine.scoops-iOS3.0-(Clutch-2.0.4).ipa`, `source: original`;
  size 6,488,237 bytes.
- Hashes:
  - MD5: `3ab0adc8149eed3dbe536dcb1edbbc28`
  - SHA-1: `72e52ca87ca577da11c55592732f4286c4861c9c`
  - SHA-256: `3d4ce40592e352da6bbfdd894deac870183cd633a2f9b54c1adc70a8788dc45e`
- Embedded identity (`tapHLE --info`): display `Scoops`, bundle
  `com.eeenmachine.scoops`, version **`2.5.1`** — note the Archive filename says
  nothing about the version, and the item name mentions iOS 3.0, not the app
  version. Minimum OS `3.0`, iPhone. Same developer as Omium.
- tapHLEdb: App 26, version 26, report 34 (2026-07-26, tapHLE `afcc4cf5`,
  ★☆☆☆☆).

## Current state: 1-star, no frame

## Cleared so far

Both general, both on `trunk` in `7a4d2c4b`:

1. `+[NSObject load]` did not exist. The app's `NGPlatform` class implements
   `+load` and ends it with `[super load]`, which failed with "superclass does
   not respond to selector". A no-op root implementation is correct — tapHLE
   drives `+load` itself.
2. `sel_getUid()` did not exist. It is the older name for
   `sel_registerName()`, so it forwards.

## Current frontier

```text
Error during CPU execution: MemoryError
```

A guest memory fault, so this is now a different class of problem from the two
missing-symbol aborts above: the next step is to read the register dump and
stack trace tapHLE prints just before the panic (they were not captured before
context ran out) and resolve the faulting PC in the extracted binary, per the
debugging playbook.

## Next discriminator

Relaunch, capture the register dump, and disassemble around the faulting PC. If
the faulting instruction dereferences a register loaded from a global, check the
`unhandled non-lazy symbol` warnings in the same log first — that pattern
accounted for several startup faults elsewhere in this session, and is much
cheaper to check than a full trace.
