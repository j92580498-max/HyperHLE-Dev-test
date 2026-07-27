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
Receiver 0x3001bd30 has a nil isa while sending selector "release"
```

An object is released after it has already been deallocated. The traced
messages immediately before it are a run of `NSData` and `_tapHLE_NSString`
deallocs at neighbouring addresses, which is what draining an autorelease pool
looks like, and the dead object sits in the middle of that address range. So
the shape is: something in the pool was released once too often, or was
deallocated while the pool still held it.

That points at an ownership mismatch on a returned object — a method handing
back +1 where the caller expects +0, or the reverse — rather than at a missing
method. The next step is to find which object `0x3001bd30` is: rerun with
`TAPHLE_TRACE_SELECTORS=all`, search the trace backwards for that address, and
read the `alloc`/`retain`/`release` history it accumulated before the pool
drained. The register dump is captured too (`PC 0x00134ee4`, `LR 0x000035dd`)
if the guest side is needed.

Do not guess at the guilty method from the shape alone. That approach wasted
two rebuild cycles on Tap Tap Revenge 2 and the trace answered it in one run.
