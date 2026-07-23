# Cops & Robbers compatibility work note

Last updated: 2026-07-23. Rating: ★★★☆☆ (3/5) In game.

## Identity and source

- Work branch: `compat/cops-and-robbers`.
- Canonical Archive item:
  <https://archive.org/download/ios-ipa-collection/Cops%20&%20Robbers%201.5.ipa>
  (identifier `ios-ipa-collection`).
- Archive filename: `Cops & Robbers 1.5.ipa`, `source: original`.
- Archive size: 6,060,474 bytes.
- Archive MD5: `66b9fc24b834627f0dfe5db79f28c07a`.
- Archive SHA-1: `da6e0e7d97af4a704be6a7dd63c8b5bbf10eeeaf`.
- Locally calculated SHA-256:
  `73d30a59a8d636e7c1cc33fd29e1a720a1f10badae40570c2f12aa5649e538a9`.
- Embedded bundle: `com.glu.thief3d`, display name `Cop&Robber`, version `1.3`,
  minimum OS `2.0`, iPhone device family.

Live metadata was verified on 2026-07-23 and the downloaded bytes match its
exact size, MD5 and SHA-1. As with Baby Monkey, the Archive filename and the
embedded version disagree: the file is named `1.5`, `tapHLE --info` reports
`1.3`. Keep the Archive filename as the canonical source name and treat the
embedded version as authoritative; do not claim these bytes are 1.5.

Availability was checked on 2026-07-23. Apple's Lookup API returned no result
for `com.glu.thief3d` in the US, GB, CA, AU, DE, FR or JP storefronts, and a
name search surfaced no current Glu title matching it. This is a bounded
availability observation, not proof of legal abandonment.

This is an early app: it loads the **armv6** slice (not armv7), it is
**nib-based** UIKit rather than a hand-rolled engine, and it renders its 3D
through EAGL at 320x480 portrait.

## Current checkpoint: gameplay runs

Reproduced from the clean committed release build of `c4d56964` (window title
`Cop&Robber (tapHLE c4d56964)`, no `-dirty`) against the hash-verified bytes, in
a normal visible window from an isolated `%TEMP%` run directory.

The whole front-of-game path works: language select, the sound-on question, the
title card, a multi-scene 3D intro cutscene with dialogue, the main menu (PLAY /
EXTRAS / HIDEOUT / OPTIONS / HELP), the 3D character customisation screen, the
tilt-controls tutorial, and then gameplay.

In gameplay the character runs down a 3D city street with obstacles,
collectibles, a countdown timer, a score, a PAUSE button and a skyline HUD. Over
one 18-second observation the timer ran 2:48.1 -> 2:31.3 and the score rose
300 -> 800, so the clock, the collection logic and the scene are all live. The
process stayed alive and animating throughout.

### Proven input recipe

Client coordinates as fractions of a 320x480 client area, with a title-bar focus
click first (see the playbook). Each tap is a real foreground mouse event.

1. `0.13,0.93` SELECT — accept the default language (English).
2. `0.13,0.93` SELECT — accept the default answer to the sound question.
3. `0.88,0.94` x4, a few seconds apart — SKIP through the intro cutscene.
4. `0.13,0.93` SELECT — PLAY, which opens character customisation.
5. `0.13,0.93` DONE — returns to the main menu.
6. `0.13,0.93` SELECT — PLAY again, which opens the tilt-controls tutorial.
7. `0.50,0.55` — tap the play area to dismiss the tutorial and start the run.

Do **not** tap SKIP on the tutorial screen: there it means "back" and returns to
the main menu. That cost several runs to notice.

Taps are confirmed to reach the game, not merely to coincide with timed
transitions: tapping `Español` on the language screen switched every later
screen to Spanish (`OMITIR` for SKIP, Spanish dialogue).

## Fixes this required

All three are general, none is specific to this game.

1. `55af471f` — `UIView`'s four co-ordinate conversion methods resolved a nil
   counterpart by asserting the receiver had a window. A nib-based app converts
   while laying out, before anything is mounted, so this aborted at launch.
   `CALayer`'s conversion already resolves a nil layer to the top of the
   hierarchy, so the nil is now passed straight down.
2. `582ace5f` — `AudioServicesAddSystemSoundCompletion` did not exist. Added
   with its Remove counterpart, and the routine actually runs: the run loop
   polls each sound's OpenAL source and fires on the playing-to-stopped edge.
3. `c4d56964` — `EAGLGetVersion` was not exported. Reports 1.0.

## Known limitations at this rating

- **Tilt control is untested.** The game steers by accelerometer, which tapHLE
  simulates with right-click-drag or a controller. The observed run used no
  steering at all, so the character ran straight. Nothing is known about whether
  steering, jumping or the speed boost behave correctly.
- Only the first run of the first mode was observed, for under a minute. Death,
  the timer expiring, the score screen, PAUSE, HIDEOUT, EXTRAS and OPTIONS are
  all untested.
- A file the app repeatedly `open`s and `remove`s does not exist, producing a
  steady trickle of warnings. It does not appear to matter, but it has not been
  identified.
- Audio was not measured. Do not claim it works or does not work without a wave
  capture: see the audio section of the playbook.
- `UIProxyObject` for `IBFirstResponder` is left unreplaced at nib load. It has
  not caused an observed problem, but it is a real gap.

## Next discriminator

Drive the accelerometer (right-click-drag, or a controller) during a run and
confirm the character steers, then play to a death or timer expiry and see
whether the score/game-over screen appears and a second run can be started.
That is the boundary between "gameplay runs" and "the mode can be completed".
