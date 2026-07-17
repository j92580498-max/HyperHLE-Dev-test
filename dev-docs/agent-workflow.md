# Agent workflow for game compatibility

This guide expands the contribution loop in `AGENTS.md`. It is optimized for a
maintainer who names a Windows game and wants useful iteration quickly.

## 1. Capture a reproducible case

Record these facts before diagnosing:

| Field | Example |
| --- | --- |
| Game identity | Title, version, regional/build identifier |
| tapHLE identity | Release or Git commit |
| Host | Windows version, CPU architecture, GPU and driver |
| Launch | Exact path form and tapHLE options |
| Progress | Last screen, sound, input, or log event that works |
| Failure | Crash, hang, rendering defect, missing input, or wrong behavior |
| Expected result | The next observable behavior that should occur |

Keep a baseline log. Sanitize usernames, local paths, tokens, and proprietary
data before sharing it. Never add the game or its assets to the repository.

For an app selected as an ongoing compatibility target, create or continue
`compat/<app-slug>` (for example, `compat/ricky`). Checkpoint commits are
allowed there. Follow `compatibility/README.md` for exact Archive.org source,
content-hash, app metadata, availability, and database rules. Do not record a
dirty-worktree observation as a compatibility result.

If the game is not available to the agent, ask the maintainer for a log or a
small observation that distinguishes competing hypotheses. Continue with
source inspection or a synthetic TestApp case when that can answer part of the
question.

## 2. Localize the failure

Follow the last trustworthy evidence rather than implementing every nearby
stub. Typical boundaries are:

- loader/linker: `bundle`, `mach_o`, `dyld`, and missing-symbol logs;
- CPU/ABI/memory: `cpu`, `abi`, `mem`, and crashes near guest calls;
- Objective-C dispatch: `objc` and unknown class/selector logs;
- framework behavior: the matching module under `src/frameworks`;
- files and preferences: `fs`, `paths`, and Foundation file APIs;
- graphics/input/windowing: `gles`, UIKit views, and `window`;
- audio: `audio`, AudioToolbox, AVFoundation, and OpenAL.

Add temporary diagnostics when useful, but remove noisy or proprietary output
before handoff. Prefer a focused probe over a broad implementation based only
on a guess.

## 3. Choose the shortest honest fix

The project accepts three kinds of fix:

1. A general implementation when the required behavior is clear and small.
2. A partial implementation covering the observed inputs with explicit
   fallback behavior.
3. A game-specific workaround when evidence shows it is the fastest reliable
   route.

For partial or game-specific behavior, state:

- which game/version or input pattern needs it;
- the observation it reproduces;
- how the condition is bounded so other games are unaffected; and
- what evidence would justify replacing it later.

Do not pretend a stub is a complete API. A clear narrow implementation is more
useful than an overbroad claim.

## 4. Climb the validation ladder

Stop at the highest affordable level and report where you stopped:

1. A unit test next to deterministic logic.
2. A TestApp probe for a guest-visible API or ABI behavior.
3. `cargo test -- --skip test_app` when the custom SDK is unavailable.
4. Full `cargo test` with the SDK and LLVM from `tests/README.md`.
5. A release build and launch of the exact target game on Windows.

The fifth level is the only proof that a game compatibility claim is true.
Passing lower levels still provides useful confidence when game access is
awaiting the maintainer.

For database entry, level five must use an Archive content-hash-verified
artifact against a committed tapHLE revision. Full playability is not required:
a reproducible boot, menu, or in-game milestone can justify promoting the
branch when its remaining blocker is recorded and regression checks pass.

## 5. Leave a continuation-quality handoff

Summarize:

- the user-visible improvement;
- the root cause or best-supported hypothesis;
- files and subsystem changed;
- checks run and their results;
- exact game/Windows validation performed; and
- the next observation needed if the target still fails.

Append the verified report under `compatibility/apps`, run
`python dev-scripts/compatibility.py render`, and then run the offline
`python dev-scripts/compatibility.py check`. Never edit an earlier report.

Agent work should make the next iteration cheaper, even when one turn cannot
reach the game menu.
