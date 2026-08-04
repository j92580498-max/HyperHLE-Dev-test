# Cubed Rally Redline compatibility work note

- Branch and last pushed commit: `compat/cubed-rally-redline`, see below for the
  tested revision.
- Artifact: maintainer's local copy, filename
  `Cubed Rally Redline (v1.32) [Decrypted].ipa`,
  SHA-256 `837ED1C107602EC6738CBD29DFE53BCD15F6DD57A429D7F9F88A6BDAD3F7DE87`.
  Not Archive-backed; no public provenance recorded.
- Identity, read from `tapHLE --info` (not from the filename): display name
  `Redline`, bundle identifier `com.nocanwin.redline`, bundle version `1.32`,
  minimum OS version `5.0`, armv7, iPhone + iPad, landscape only.
- Highest clean committed milestone: **the gameplay loop starts and
  persists, but the game is not playable**. A race starts, the HUD counts
  distance and score, the car responds to physics, the player crashes, the
  RESULT screen appears, and its play button starts another race that also
  runs to a result. **Do not read that as three stars.** The world geometry is
  shredded into diagonal streaks and menu artwork is covered by opaque white
  blocks, so a person cannot actually see where the road is. Rating stays at
  **two stars** until the rendering is fixed; the loop persisting is a
  necessary part of three stars, not a sufficient one.

## What this app is

A Unity 4.0.1f2 game, built against the iOS 6.0 SDK. That makes it much newer
than tapHLE's usual targets and the first Unity title to run, so most of what
was fixed for it is engine-shaped rather than game-shaped: the Mono runtime, its
allocator, and Core Foundation string and data bridging. `--info` warns that the
app wants iOS 5.0 and tapHLE supports 4.0 and earlier; that warning is not fatal
and did not need addressing.

`cryptid` is 0, so the binary is genuinely decrypted and load failures would be
real results rather than an artefact of a bad dump.

## Click map

Launch options: none (no entry in `tapHLE_default_options.txt`). Window client
area 480x320, landscape, matching the app's own resolution 1:1.

Wait about 55 s after launch before the first tap — the Mono runtime, scene load
and asset unload take that long.

**Press duration matters: use ~300 ms.** A 90 ms press is not seen at all, and
that produced a convincing false negative — the title screen animates on its own
for a few seconds after load, so a frame comparison right after a missed tap
still shows a change. Confirm the screen is static first, then tap.

| From | Client tap | To |
| --- | --- | --- |
| Title / mode select | (242, 141) | Car select |
| Car select | (247, 264) | Controls screen |
| Controls screen | (239, 287) | Race |

## Proven

- Unity engine initialises, Mono loads every assembly including
  `Assembly-CSharp.dll`, and scenes load (4711 objects for the race scene).
- `applicationDidBecomeActive()` fires and the Core Animation compositor
  presents.
- Gameplay is real, not a still frame: `stdout` records `NoMetric:Play`, then
  `Player: Dead`, then `NoMetric:EndTimed:Game`, and OS-level screenshots hash
  differently between samples during the race.
- These helpers use the base AAPCS convention (operands and results in core
  registers, not VFP), read off `___fixdfdi` and `___floatdidf` call sites in
  this binary rather than assumed.

## Rejected

- "The soft-float helpers return in `d0` because iOS is AAPCS-VFP." Disproved by
  disassembly: the app does `vmov r0, r1, d16` immediately before calling
  `___fixdfdi` and reads a `___floatdidf` result from `r0`/`r1`.
- "The app freezes because it is waiting for input." It was spinning on an
  `mmap` that could not be satisfied; frames were byte-identical for 30 s.
- "The first tap did not land because touch routing is broken." The press was
  simply too short.

