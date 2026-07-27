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

## Current frontier: UIBarButtonItem

```text
Class "UIBarButtonItem" is unimplemented. Call to class method "alloc".
```

The app builds a navigation bar or toolbar item. `UIBarButtonItem` is a small
class — a target, an action, and a title or image — so this is a much smaller
job than the table view was. Check what else of the bar family the app needs
before starting: if it also wants `UINavigationBar`, `UINavigationItem` or
`UIToolbar`, they are worth doing together rather than one abort at a time,
which is the mistake the scalar type encodings taught.

Still no frame, so nothing about this app's rendering, input or audio has been
assessed and no report is filed.
