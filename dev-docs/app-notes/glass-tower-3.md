# Glass Tower 3 compatibility work note

- Branch: `compat/glass-tower-3`. Reusable fixes graduated to `trunk`.
- Canonical artifact: <https://archive.org/details/ios-ipa-collection>,
  `Glass Tower 3 2.0.1.ipa`; size 24,515,360 bytes.
- Hashes:
  - MD5: `56bafeb6bf4fad62e5115eea5165e9cf`
  - SHA-1: `0cdb51c48abf825078851d3fa33952bd666b0410`
  - SHA-256: `123514a494fa81448264b56f128b8dd18eaf2e61594cf73a966b484c9a99e645`
- Embedded identity (`tapHLE --info`): display name `GlassTower3`, bundle
  `com.idevua.glasstower3`, version `2.0.1`, minimum OS `4.0`, iPhone + iPad.
- Options: none tried yet beyond the default.
- tapHLEdb: no report yet. **Not rated** — it does not reach a stable screen.

## Current state: does not launch (below 2 stars)

On `a91876fa` the app still aborts during startup, but much later than before.
Nothing has been rated, and no report has been submitted.

This is the first app in this series that needs the **iOS 4** API surface
rather than the iPhone OS 3 one, which is why it hits so much missing ground.

## Startup failures cleared so far

Each was a general emulator gap, each is fixed on `trunk`, and each moved the
abort strictly later. In the order they were hit:

1. `-[CALayer setMasksToBounds:]` — absent. (`b2023dfa`)
2. `-[CALayer setBorderWidth:]` / `setBorderColor:` — absent. (`b2023dfa`)
3. `-[UILabel setAdjustsFontSizeToFitWidth:]` — was `assert!(!adjusts)`, so any
   app setting it aborted. Now implemented as real shrink-to-fit, with
   `-[UIFont fontWithSize:]` added to support it. (`b2023dfa`)
4. `ADBannerView` — the whole iAd framework was missing. Added as a banner that
   never fills and says so through the delegate. (`b2023dfa`)
5. `+[UIView animateWithDuration:animations:completion:]` — the iOS 4
   block-based animation API was missing. Added on top of the existing
   begin/commit machinery, with the completion block delivered when the
   animation actually stops. (`b2023dfa`)
6. `NSClassFromString(@"GKLeaderboardViewController")` panicked instead of
   returning nil. The app is *probing* for Game Center, which is exactly what
   the function is for. (`69d3915c`)

Note on 6: `GKLeaderboardViewController` is **not** an undefined class symbol in
the binary — the app resolves it by name at run time. So this is genuine
feature detection, not a link-time dependency, and returning nil is the answer
the app is written to handle. Do not "fix" this by implementing the class.

## Current frontier

Startup now reaches guest code and dies there:

```text
Error during CPU execution: MemoryError
```

That is a guest memory fault, not a missing selector, so the next step is a
different kind of work from the six above. Per the debugging playbook: resolve
the faulting PC to its owning image, read the register dump, and disassemble
around that address before designing anything. The registers and stack trace
are already printed automatically before the panic.

## Useful facts already established

- External class references (undefined `_OBJC_CLASS_$_*` symbols) are all
  classes tapHLE already has: Foundation, UIKit, StoreKit, AVFoundation,
  OpenGLES, plus `ADBannerView`. So there is no second missing framework
  waiting behind the current fault — anything else is resolved by name.
- Layer-related selectors the app uses: `setMasksToBounds:`, `setBorderWidth:`,
  `setBorderColor:`, `setCornerRadius:`, `setBounds:`, `setPosition:`,
  `setHidden:`, and UIView's `setClipsToBounds:`. All are now implemented,
  though the compositor still neither clips nor strokes borders.
- The app uses StoreKit (`SKPaymentQueue`, `SKProductsRequest`), so a purchase
  path may appear later; the free level pack should not need it.

## Checks run

- `cargo fmt --all -- --check`
- `cargo test --workspace --lib` (91 passed)
- `cargo build --release`
- Regression sweep on Windows over Baby Monkey, Fantastic Mr. Fox, Flight
  Control HD, Percy Jackson, Ricky, SPY mouse HD, Snappers, Warlords HD and
  Glass Tower 2: all still reach their expected screens.

## Next discriminator

Resolve the `MemoryError` fault: take the printed PC, subtract the image base,
and disassemble that address in the extracted `GlassTower3` binary to identify
the owning function. Decide from the faulting operands whether this is a
missing non-lazy data import (the playbook's `___stack_chk_guard` pattern), an
uninitialised C++ container, or a genuine HLE bug.
