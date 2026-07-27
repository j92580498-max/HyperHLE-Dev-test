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
Revenge 2, so everything cleared for that app (see `tap-tap-revenge-2.md`) is a
prerequisite and is already on `trunk`; TTR3 then needs more. That shared work
grew a lot this session — declared-property metadata, NSInvocation return
values, the audio queue clock — and TTR3 picked all of it up for free.

## Cleared so far

1. `-[NSInvocation target]` / `-selector` / `-methodSignature` — only the
   setters existed. (`4c82a0a5`)
2. `-[NSInvocation setSelector:]` asserted the selector was unset, so an
   invocation could not be reconfigured. (`4c82a0a5`)
3. `+[NSThread mainThread]`. (`4c82a0a5`)

## Cleared since

4. `-[NSTimer initWithFireDate:interval:target:selector:userInfo:repeats:]`,
   the designated initialiser. Adding it also meant raising the host method
   arity limit: `impl_HostIMP!` stopped at five method arguments and this
   selector has six. `CallFromGuest` already went to eleven, so that was a
   one-line ceiling rather than a real limit.

## Current frontier

```text
Receiver 0x3001b7a0 has a nil isa while sending selector "release"
```

An object is released after it has already been deallocated, at a pool drain.

### What was tried, and why it did not settle it

Tracing the dead address backwards gives its message history — but **the
address is reused**. Between the crash and the object's creation, the same
address is allocated and freed several times, as a `_tapHLE_NSMutableString`
and then as a `_tapHLE_NSString`, so what looks like one object's history is
several objects' histories concatenated. Any conclusion drawn from reading it
straight through is wrong. This is the trap to know about before spending a run
on it.

The first attempt read that concatenated history as one object created by
`+[NSString stringWithString:]` and over-released by the bundled comScore SDK.
That reading produced a **real and independent** fidelity fix — Foundation
returns the argument itself from `stringWithString:` for an immutable receiver,
so an app that over-releases a constant string is harmless there and was fatal
here — and it is now on `trunk`. **It did not fix this crash.** The abort moved
to a different address and the same shape.

### What would settle it

The trace needs to distinguish objects, not addresses. Either:

- record allocations with a serial number rather than only an address, so a
  reused address reads as two distinct objects; or
- break at the point the pool adds the object, not at the crash — the
  `NSAutoreleasePool addObject:` line that precedes the fatal drain names the
  pool, and the object added at that moment is unambiguous.

The second needs no new tooling. It is the next step.

Also worth ruling out first, since it is cheap: this is a comScore/CSStorage
code path (`[CSStorage has:]` appears immediately before the object is built),
and analytics SDKs are exactly the code that reaches for undocumented runtime
behaviour. Confirming whether the same crash happens with that SDK's network
calls already failing — which they do here, tapHLE has no network stack for
Tapulous' services — would say whether this is on the app's error path rather
than its normal one.
