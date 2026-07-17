# Ricky compatibility work note

Last updated: 2026-07-17.

## Identity and branch

- Work branch: `compat/ricky`.
- Initial troubleshooting-playbook checkpoint: `4f6337fc`.
- Canonical item: `https://archive.org/details/ios-ipa-com.nabilchatbi.ricky`.
- Exact local/canonical filename used for runs:
  `Ricky (v2.1) [Cracked].ipa`.
- SHA-256:
  `74380d6f009a43d430fe3bceeccbb1c9d2b20f40a02088688b500b71eb937a5f`.
- Bundle: `com.nabilchatbi.Ricky`, version `2.1`, minimum OS `3.0`.
- Never use the older local `Ricky (v2.1) [Decrypted].ipa`; it does not match
  the canonical artifact hash.

## Highest clean milestone

Commit `d9bc6a5d02322f1902e14ba4a451e8b3a4ade2ee` launches the exact artifact,
navigates Story Mode, enters the first playable level, is sent a two-second hold
on the on-screen right movement control, and remains alive for
approximately 18 seconds after level entry on Windows. The same route previously
reached `AudioQueuePrime` and panicked; this clean run primed the packetized MP3
queue without error `-66681`, MP3 decode warnings, or the non-null output-pointer
panic. Compatibility report `2026-07-17-windows-story-in-game` supersedes the
earlier menu-only report.

The earlier clean `46dce243` runtime visually established that the level
rendered, the character moved, and the scene scrolled. A System.Drawing
`Graphics.CopyFromScreen` attempt on the `d9bc6a5d` OpenGL window produced a
black bitmap and was excluded from evidence; agents should not repeat that
capture method as proof of rendering. It is not evidence that the rendered
window was black. Audible audio output has not been confirmed, so audio remains
`partial` rather than `working`.

The runtime host used for this observation was Windows 11 Pro 10.0.22631 on an
Intel i7-13700H with Intel Iris Xe driver 32.0.101.6881.

The full Archive.org verification protocol passed immediately before the
`d9bc6a5d` run, including remote metadata hashes, local SHA-256, embedded bundle
identity, and `tapHLE --info` cross-checks.

## Repeatable input recipe

The tapHLE client area is 480 by 320. The coordinates below are client
coordinates and are valid only for that client size. Use foreground mouse
input, verify the window still belongs to the spawned tapHLE process before
every event, and restore the previous cursor/window afterward:

1. Acquire and verify foreground ownership, then tap Story Mode at `(75, 295)`
   once. Do not use an unconditional double tap; an earlier run needed a second
   click only because its first click activated the window.
2. New Game at `(397, 295)`.
3. PLAY at `(236, 203)`.
4. Advance the story at `(240, 295)` exactly twice.
5. Hold the right movement control near `(137, 289)` to verify gameplay input.

Static inspection established that the story screen advances only on a touch
begin in its bottom 50-pixel strip. It has three textures and loads level 2
after two qualifying taps; blind center-screen tapping and waiting are rejected
strategies.

## Proven fixes and risks

- `9b31d508`: `dlsym(RTLD_DEFAULT)` resolves symbols from loaded app binaries
  and returns null for a missing symbol.
- `46dce243`: Ricky zeroes new allocations and quarantines freed allocator
  chunks of exactly `0x40` bytes. This avoids two Unity delayed-call red-black
  tree crashes. The quarantine retains those chunks for the process lifetime,
  so long-session memory growth remains a risk to test.
- `fad70bc1`: AudioQueue owns Ricky's transient direct packet descriptions and
  decodes packetized MP3 through a persistent Symphonia decoder so the Layer III
  bit reservoir survives across queue buffers.
- `d9bc6a5d`: `AudioQueuePrime` accepts Ricky's non-null prepared-frame output
  pointer, implements zero/requested-frame preparation semantics, and reports
  the number of frames prepared.
- A Mono signal/exception recovery implementation would not fix the observed
  native bundled-libstdc++ tree fault; that path was investigated and rejected.

## Resolved audio failure path

A bounded diagnostic trace reached the first playable level with the canonical
hash and proved:

- the AudioQueue format is genuine MPEG-1 Layer III, 44.1 kHz stereo, 1,152
  frames per packet (`.mp3`), not float PCM or mislabeled PCM;
- Ricky allocates three regular 16,384-byte queue buffers;
- each enqueue supplies 39 **direct** packet descriptions through a transient,
  reused pointer; embedded descriptions are null/zero;
- encoded frames are contiguous 417/418-byte packets beginning with MP3 sync
  bytes; and
- the old unsupported-format path produced AudioQueue error `-66681`.

Rejected approaches: relabeling the data as PCM, decoding an entire queue buffer
as one MP3 packet, or creating a new decoder per buffer. Packet descriptions
must be copied at enqueue time, and one stateful decoder must survive across
buffers for the Layer III bit reservoir.

Commits `fad70bc1` and `d9bc6a5d` implement that design. Focused AudioQueue
tests pass, the full release library suite passed after MP3 integration, and an
exact-commit runtime crossed the former failure point without the old error or
panic. Embedded packet descriptions remain unsupported, but Ricky supplies
direct descriptions and does not require that path.

## Next compatibility frontier

Confirm audible output and continuity across recycled AudioQueue buffers. Then
exercise action buttons, hazards, death/restart, level completion, and saving.
Run a longer gameplay session while monitoring memory because the `0x40`-byte
allocation quarantine deliberately retains matching chunks for the process
lifetime. Do not promote audio to `working` until a human or a trustworthy
capture confirms sane audible output; successful decode/prime and process
survival prove only `partial` audio support.
