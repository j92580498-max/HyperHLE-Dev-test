# Ricky compatibility work note

Last updated: 2026-07-17.

## Identity and branch

- Work branch: `compat/ricky`.
- Canonical item: `https://archive.org/details/ios-ipa-com.nabilchatbi.ricky`.
- Exact local/canonical filename used for runs:
  `Ricky (v2.1) [Cracked].ipa`.
- SHA-256:
  `74380d6f009a43d430fe3bceeccbb1c9d2b20f40a02088688b500b71eb937a5f`.
- Bundle: `com.nabilchatbi.Ricky`, version `2.1`, minimum OS `3.0`.
- Never use the older local `Ricky (v2.1) [Decrypted].ipa`; it does not match
  the canonical artifact hash.

## Highest clean milestone

Commit `46dce243448bd88c8d23a80b38feba749633d7e9` launches the exact artifact,
renders the main menu, enters Story Mode and level 2, and accepts movement input
on Windows. The process remained alive while the character moved and the scene
scrolled. Compatibility report commit `39e09d56` conservatively records the
earlier menu milestone; gameplay and audio still require a superseding report
after a clean committed rerun.

The runtime host used for this observation was Windows 11 Pro 10.0.22631 on an
Intel i7-13700H with Intel Iris Xe driver 32.0.101.6881.

## Repeatable input recipe

The tapHLE client area is 480 by 320. Use foreground mouse input and restore the
previous cursor/window afterward:

1. Story Mode at `(75, 295)`; tap twice when the first tap only activates the
   window.
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
- A Mono signal/exception recovery implementation would not fix the observed
  native bundled-libstdc++ tree fault; that path was investigated and rejected.

## Current audio frontier

A bounded dirty-build trace reached level 2 with the canonical hash and proved:

- the AudioQueue format is genuine MPEG-1 Layer III, 44.1 kHz stereo, 1,152
  frames per packet (`.mp3`), not float PCM or mislabeled PCM;
- Ricky allocates three regular 16,384-byte queue buffers;
- each enqueue supplies 39 **direct** packet descriptions through a transient,
  reused pointer; embedded descriptions are null/zero;
- encoded frames are contiguous 417/418-byte packets beginning with MP3 sync
  bytes; and
- the current unsupported-format path produces AudioQueue error `-66681`.

Rejected approaches: relabeling the data as PCM, decoding an entire queue buffer
as one MP3 packet, or creating a new decoder per buffer. Packet descriptions
must be copied at enqueue time, and one stateful decoder must survive across
buffers for the Layer III bit reservoir.

## In-progress implementation

The worktree after the trace contains an uncommitted streaming MP3 wrapper in
`src/audio/symphonia_formats.rs` plus its `src/audio.rs` re-export. It uses the
existing Symphonia dependency, validates decoded rate/channels, converts to
interleaved signed 16-bit PCM, and has two passing synthetic-frame/reset tests.
`core_audio_types.rs` also has the MP3 format constant and packed packet
description type. AudioQueue integration is not complete and must not be
treated as a tested compatibility change.

Next step: finish the direct-description snapshot and persistent-decoder path
in `audio_queue.rs`, reset decoder state with the queue, run focused/full
library tests, commit the implementation, rebuild from that exact commit, and
replay the input recipe. Confirm that `-66681` disappears, callbacks recycle
beyond the initial buffers, gameplay input remains functional, and audio is
sane before appending an `in-game` compatibility report.
