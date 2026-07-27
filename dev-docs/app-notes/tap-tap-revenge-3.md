# Tap Tap Revenge 3 compatibility work note

- Branch: `compat/tap-tap-revenge-3`. Reusable fixes graduated to `trunk`.
- Canonical artifact:
  <https://archive.org/download/ios3-6-ipas/com.tapulous.taptaprevengeIII.ipa>
  (identifier `ios3-6-ipas`), `source: original`; size 14,448,768 bytes.
- Hashes:
  - MD5: `60d59a05fd0f1134ffe0268d8cea4988`
  - SHA-1: `0722dec3bacba8ff3f2c6351681e54ca43969b7f`
  - SHA-256: `5c37763200da8dba06346164e8f104b7fb82b1de883d4f58d305109dbc978cec`
- tapHLEdb: App 15, version 31, report 48 (2026-07-27, tapHLE `b953ef9c`,
  ★★☆☆☆).

## Highest milestone: 2-star (Starts / Menu), tapHLE `b953ef9c`

It launches. The main menu renders in full: the title, the tutorial banner, the
avatar panel, the "You are offline" notice, the Online / 1 Player / 2 Player
mode wheel, and the Play button. Verified on a clean committed build (window
title `Tap Tap (tapHLE b953ef9c)`, no `-dirty`).

This is the same Tapulous codebase as Tap Tap Revenge 2, so everything cleared
for that app is a prerequisite and is already on `trunk`. TTR3 needed nine more,
all general and all on `trunk`:

1. `-[NSTimer initWithFireDate:interval:target:selector:userInfo:repeats:]`,
   the designated initialiser. Adding it meant raising the host method arity
   ceiling: `impl_HostIMP!` stopped at five method arguments and this selector
   has six. `CallFromGuest` already went to eleven, so that was an oversight
   rather than a limit.
2. An already-deallocated object found in a draining autorelease pool now warns
   and skips instead of aborting.
3. `-[UIView autoresizingMask]` and `-autoresizesSubviews` — setters that
   discarded their argument, with no getters at all.
4. `-[UIWebView initWithCoder:]`, which was `todo!()` and took the whole nib
   down with it.
5. `-setNeedsLayout` and `-layoutIfNeeded`, deferred to the run loop.
6. `-[NSScanner scanInteger:]`.
7. `-[NSArray makeObjectsPerformSelector:]` and the `withObject:` form.
8. CGPath, plus path fill and stroke on CGContext — neither existed at all.
9. Re-parenting a guest subclass of a Foundation cluster class onto tapHLE's
   concrete implementation.

## UITableView is implemented; the frontier has moved

`UITableView`, `UITableViewCell` and `UITableViewController` now exist on
`trunk`, along with `NSIndexPath` (which was an empty stub) and
`UILocalizedIndexedCollation`. Tapping Play gets through all of them.

The table view **builds every row up front instead of recycling cells**. That
is a deliberate trade documented in its own module: the protocols, selection
and scrolling work without visible-rectangle bookkeeping, at the cost of being
wrong for a table with thousands of rows. This app's track list has tens.

## Current frontier: a guest MemoryError after the track list loads

```text
Error during CPU execution: MemoryError
```

No missing selector, no unimplemented class — the guest itself faults. That is
a different class of problem from everything cleared so far and needs the
register dump and a disassembly around the faulting PC, per the debugging
playbook, rather than another round of "add the method it asked for".

Worth checking first, because it is cheap and this app has form for it: three
apps on this target list die inside bundled analytics SDKs (see
`jellycar.md`). Look at what runs between the table appearing and the fault
before assuming the table view is at fault.

Do **not** assume the new table view is the cause without evidence. It is the
newest code in the path and therefore the obvious suspect, which is exactly why
it deserves a measurement rather than a hunch — the eight-app regression sweep
and Ragdoll Blaster Lite both exercise it without faulting.
