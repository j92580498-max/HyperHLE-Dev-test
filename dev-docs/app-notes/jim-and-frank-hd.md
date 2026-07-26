# The Jim and Frank Mysteries HD compatibility work note

- Branch: `compat/jim-and-frank-hd`. Reusable fixes graduated to `trunk`.
- Canonical artifact:
  <https://archive.org/details/ios-ipa-com.chillingo.thejimandfrankmysterieshd>,
  file `J & F HD (v1.1) [Cracked].ipa`, `source: original`; size 398,940,007
  bytes. The item lists a second name for the same bytes (the full
  "The Jim and Frank Mysteries ... HD (v1.1) [Decrypted].ipa"); the short name
  was chosen only because it is ASCII-safe on Windows. Same MD5.
- Hashes:
  - MD5: `8c20da73edbbd7d7d2114b1a8ff2ac6a`
  - SHA-1: `543f59387288315eacc46620116df06a5720d057`
- Embedded identity: bundle `com.chillingo.thejimandfrankmysterieshd`,
  version `1.1`.
- tapHLEdb: App 20, version 20, report 28 (2026-07-26, tapHLE `4e246384`,
  ★☆☆☆☆).

## Current state: 1-star, no frame

Startup aborts before anything is drawn.

## Cleared so far

Four general gaps, all on `trunk` in `7c1c6194`, each the exact next blocker:

1. `-[NSFileManager setAttributes:ofItemAtPath:error:]` — absent. Now accepted
   and reported successful; tapHLE's filesystem has no attributes to store.
2. `NSHTTPCookieStorage` — the whole class was absent. Now a per-process
   in-memory jar with a working shared instance.
3. `-[NSBundle classNamed:]` — absent. Returns the class if implemented, nil
   otherwise, matching NSClassFromString's feature-detection behaviour.
4. `CGRectContainsRect()` — absent.

## Current frontier

```text
Class "NSDecimalNumber" is unimplemented. Call to class method
"decimalNumberWithString:"
```

`NSDecimalNumber` is a genuine piece of work, not a stub: it is an
arbitrary-precision decimal with its own arithmetic, rounding behaviour and
`NSDecimal` struct representation, and it is an `NSNumber` subclass, so it has
to satisfy that interface too.

## Next discriminator

Decide the scope before writing any of it. An app that only parses and formats
decimal strings — which `decimalNumberWithString:` alone suggests — needs far
less than one doing financial arithmetic. Trace which `NSDecimalNumber`
selectors this binary references (`__objc_selrefs`) first; if the set is small
and arithmetic-free, backing it with `f64` and documenting the precision limit
is a defensible bounded implementation. If it uses the arithmetic and rounding
methods, that is a proper decimal type and a separate `feat/` in its own right.

Note this app is 398 MB, by far the largest on the target list, so each launch
is slow. Batch changes before re-running.
