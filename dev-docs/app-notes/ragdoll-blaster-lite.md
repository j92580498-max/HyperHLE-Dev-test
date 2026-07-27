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

## Current frontier: UITableViewController

```text
Class "UITableViewController" is unimplemented. Call to class method "initialize".
```

**This is the same wall Tap Tap Revenge 3 hits**, and tapHLE has no
`UITableView` either — so this is not a missing method but a missing class
cluster: the table view, its data source and delegate protocols, cells, reuse
and selection.

Two apps on the current target list now stop here, which makes it the highest
value piece of UIKit left undone. Whoever takes it should use **Ragdoll Blaster
Lite as the first test and TTR3 as the second**: this app hits it during
startup, which is a much faster edit-run cycle than TTR3's four-tap,
ninety-second path to the same class.

Nothing else about this app has been assessed — it has never drawn a frame, so
its rendering, input and audio are all unknown rather than working.
