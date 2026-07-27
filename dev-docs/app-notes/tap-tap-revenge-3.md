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

## Current frontier: UITableViewController, and UITableView under it

```text
Class "UITableViewController" is unimplemented. Call to class method "initialize".
```

Tapping Play stops here. This is **not** a small gap: tapHLE has no
`UITableView` either, so the track list needs the whole class implemented —
data source and delegate protocols, cells, reuse, selection, and scrolling —
rather than a missing method filled in.

That is a substantial and highly reusable piece of UIKit, so it is worth doing
on its own terms rather than as a step in this app. Whoever picks it up should
treat this app as the acceptance test: it uses a plain grouped list.

### The over-release, and a trap worth knowing

Before the pool change, startup died with an object released after it had been
deallocated. The obvious investigation is a trap: **the dead address is reused**
several times between the object's creation and the crash, so reading its traced
history straight through concatenates several objects into one plausible, wrong
story.

A fix did come out of the first, wrong reading — Foundation returns the argument
itself from `+[NSString stringWithString:]` for an immutable receiver, which is
real and is on `trunk` — but it did not fix the crash. The crash is now simply
tolerated at the point of drain, which is the right place for it: the pool is
never the culprit, only the finder.
