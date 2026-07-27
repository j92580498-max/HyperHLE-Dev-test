# Ragdoll Blaster Lite compatibility work note

- Branch: work done directly on `trunk` (reusable fixes only so far).
- Canonical artifact:
  <https://archive.org/download/ragdoll-lite-v-1.4-clutch-1.3/Ragdoll%20Lite-v1.4-(Clutch-1.3).ipa>
  (identifier `ragdoll-lite-v-1.4-clutch-1.3`), `source: original`;
  size 5,789,885 bytes.
- Hashes:
  - MD5: `4977342b062e26f6c3fe9344434ac702`
  - SHA-1: `8dac50a93c1eefdc638fdb15f83bc2113ba048fb`
  - SHA-256: `a9c888b0aacbf3ecb9fa9e0d7a777c6e162ff796d2fc9ffcfa6afcb7aeb3b678`
- Embedded identity (`tapHLE --info`): display name `Ragdoll Lite`, bundle
  `com.backflipstudios.RagdollBlasterLite`, version `1.4`, minimum OS `2.2.1`,
  iPhone. Note the bundle identifier says **RagdollBlasterLite** while the
  display name is just "Ragdoll Lite"; the Archive item is named for the
  latter.
- tapHLEdb: no report yet. It does not reach a screen, so there is nothing
  measured to file beyond "does not start", which is filed once the frontier
  below is either cleared or confirmed as the ceiling.

## Current state: no frame

## Cleared

1. `-[NSBundle initWithPath:]` did not exist. Adding it also needed
   `+[NSBundle allocWithZone:]`, because `+alloc` on NSBundle fell through to
   NSObject and produced an object with no `NSBundleHostObject` behind it —
   `-initWithPath:` would have panicked on the first borrow. It returns nil for
   a path that does not exist, which Apple documents and which is how an app
   asks "is there a bundle here?".

## UITableViewController is implemented; the frontier has moved

`UITableView`, `UITableViewCell` and `UITableViewController` now exist on
`trunk`, with `NSIndexPath` filled in underneath them. This app was the first
test for that work, as planned — it hits the class during startup, which made
the edit-run cycle minutes shorter than Tap Tap Revenge 3's four-tap path.

## The bar family is done; the frontier is now setjmp

`UIBarItem`, `UIBarButtonItem` and a real `UINavigationItem` are on `trunk`,
along with `-[UIViewController navigationItem]`. Checking the binary first paid
off: it references the whole navigation family, and tapHLE already had
`UINavigationBar`, `UINavigationController` and a stub `UINavigationItem`, so
only the item classes were actually missing.

**Nothing draws a navigation bar or toolbar.** The items are stored and answer
their accessors, and their target and action are kept so an app that reads them
back or fires them itself behaves — but tapHLE will never fire one. An app whose
only route onward is a bar button is stuck, and the missing piece is the *bar*,
not these classes.

## Current frontier: `_setjmp`

```text
Call to unimplemented function __setjmp
```

`setjmp`/`longjmp` are how C code unwinds out of an error deep in a library —
libpng and libjpeg both use them for exactly that, which is the likely caller
here.

This is implementable in an emulator, and more cleanly than on real hardware:
`setjmp` saves the callee-saved guest registers, `sp` and `lr` into the
`jmp_buf` and returns 0; `longjmp` restores them, sets the return value, and
resumes at the saved `lr`. The awkward part is the non-local jump itself —
making a host function resume the guest somewhere other than where it was
called from — so read how `GuestFunction` returns control before starting.

It is also worth checking whether the caller is a decoder that only *needs*
setjmp on the error path. If so, a `setjmp` that always returns 0 and a
`longjmp` that aborts loudly would get this app running while being honest that
the error path is unimplemented — but that is a decision to take with the
caller identified, not before.

Still no frame, so nothing about this app's rendering, input or audio has been
assessed and no report is filed.
