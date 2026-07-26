# Tap Tap Revenge 3 compatibility work note

- Branch: `compat/tap-tap-revenge-3`. Reusable fixes graduated to `trunk`.
- Canonical artifact:
  <https://archive.org/download/ios3-6-ipas/com.tapulous.taptaprevengeIII.ipa>
  (identifier `ios3-6-ipas`), `source: original`; size 14,448,768 bytes.
- Hashes:
  - MD5: `60d59a05fd0f1134ffe0268d8cea4988`
  - SHA-1: `0722dec3bacba8ff3f2c6351681e54ca43969b7f`
  - SHA-256: `5c37763200da8dba06346164e8f104b7fb82b1de883d4f58d305109dbc978cec`
- tapHLEdb: no report. **Not rated** — it does not reach a stable screen.

## Current state: does not launch

Startup aborts before any frame. This is the same Tapulous codebase as Tap Tap
Revenge 2, so the nine general gaps cleared for that app (see
`tap-tap-revenge-2.md`) are prerequisites and are already on `trunk`; TTR3 then
needs more.

## Cleared so far

1. `-[NSInvocation target]` / `-selector` / `-methodSignature` — only the
   setters existed. (`4c82a0a5`)
2. `-[NSInvocation setSelector:]` asserted the selector was unset, so an
   invocation could not be reconfigured. (`4c82a0a5`)
3. `+[NSThread mainThread]`. (`4c82a0a5`)

## Current frontier

```text
NSTimer does not respond to selector
"initWithFireDate:interval:target:selector:userInfo:repeats:"
```

The designated initialiser for a timer with an explicit fire date. tapHLE's
NSTimer has the `scheduledTimerWithTimeInterval:...` factories but not this one.
It is a bounded next step: construct the timer with the given interval and
repeat flag, and honour `fireDate` as the first fire time rather than
"now + interval".

## Next discriminator

Implement that initialiser, then continue the crash-to-crash loop. Expect the
same shape as TTR2: a long tail of Foundation and networking surface before a
frame. Re-assess after the next two or three blockers — TTR2 needed nine and
still only reached its menus, so if TTR3 is not visibly closer to a frame by
then it is a poor value target compared with untried apps.
