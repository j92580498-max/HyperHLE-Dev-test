# Contributing to tapHLE

tapHLE welcomes human contributors, coding agents, and human-agent teams. This
fork is deliberately AI-development-led: agents are expected to help
investigate, implement, test, and document changes. A person remains
accountable for deciding what enters the project.

Read `AGENTS.md` first. It defines the project priorities, trust boundary,
artifact rules, and validation expectations.

If you are new to programming and want to use a coding agent for one game,
start with `HELP_A_GAME.md`. It includes a prompt you can copy.

For selected-game diagnosis, follow `dev-docs/app-debugging-playbook.md` and
read any sanitized continuation note under `dev-docs/app-notes/` before
repeating runtime or static-analysis work.

## What the project wants

The product goal is broad compatibility with early iPhone OS games on Windows.
The fastest way to move toward it is often to fix a real blocker in one game.
Contributors may choose games they care about. Nobody is required to take a
game request from someone else.

macOS changes are welcome when they enable compilation, debugging, behavioral
comparison, or shared code needed by Windows. Android development is not in
scope. Broad framework-completeness projects, aesthetic rewrites, and
speculative abstractions are lower priority than a working game.

Pragmatic fixes are welcome. If a game needs a narrow workaround, keep it
local, document the evidence behind it, and add a regression check when
practical. A clean general implementation is preferred when it takes similar
effort, but contributors are not required to redesign a subsystem before
shipping a useful compatibility improvement.

## Start with evidence

For a game compatibility report, include:

- exact game title, version, and build when known;
- tapHLE commit or release;
- Windows version, CPU, and GPU;
- exact launch steps and relevant options;
- the last visible or logged successful behavior;
- expected and actual results; and
- the tapHLE log, with personal paths or data removed.

Do not upload the game, its assets, decryption keys, or raw log. A canonical
Archive.org item reference is accepted only under the exact unavailable-build
and verification protocol in `compatibility/README.md`; do not post guessed
items or use an archive as a substitute for an actively marketed game.

## Compatibility records and app branches

`compatibility/README.md` is the canonical protocol. The maintainer may accept
good-faith testing of a genuinely unavailable or abandoned build when there
is no current App Store market alternative. This is a project scope decision,
not a blanket legal conclusion about "abandonware." Respect DMCA notices and
rightsholder requests, and re-check current availability before every new
report.

Use the exact Archive.org item URL supplied by the maintainer or reporter; do
not search for or guess one. Verify its canonical identifier, exact IPA
filename, and published hashes through the Archive metadata endpoint. Once the
exact designated original is confirmed, an agent may download only that file
to an external cache and must hash-match it before opening it. The Archive
filename remains canonical even if Windows needs a different local cache name.
Keep the IPA outside Git, content-hash the local file, inspect its embedded
`Info.plist`, and cross-check the same file with `tapHLE --info` when possible.
Only a hash-verified artifact run on a committed tapHLE revision may become a
compatibility report. Append reports; never overwrite an earlier result.

App work belongs on `compat/<app-slug>` (for example, `compat/ricky`).
Exploratory checkpoint commits are allowed there. Keep unfinished, unverified,
or unstable experiments on that branch. Merge a stable checkpoint to `trunk`
once the exact hash-verified app reproduces a useful milestone, the database
honestly records what works and what remains, and normal regression checks
pass. Full playability is not required. Dirty-worktree observations are
provisional and must not enter the database.

## Development workflow

1. Create a focused branch from `trunk`; use `compat/<app-slug>` for app work.
2. Initialize submodules with `git submodule update --init`.
3. Reproduce the failure or create a small synthetic probe.
4. Make the smallest complete change that advances the target game.
5. Add or update a focused test when practical.
6. Run the relevant checks from `AGENTS.md`.
7. For verified app testing, append the exact result and regenerate
   `COMPATIBILITY.md`.
8. Open a GitHub pull request using the repository template.

Pull requests should say which agent or AI tool materially assisted, what
evidence guided the implementation, what was tested on Windows, and which
claims still need manual game validation. AI involvement is not a negative;
transparent validation makes the result easier to trust and continue.

Version bumps, release tags, and Windows packages follow
`dev-docs/releases.md`. Do not create or move a release tag as part of an
ordinary contribution.

## Copyright and reverse engineering

Compatibility work must not compromise the project legally.

- Prefer public API documentation and clean behavioral experiments.
- You may inspect a target game for an authorized compatibility task under
  `compatibility/README.md`, but do not commit or redistribute proprietary
  material.
- Do not consult leaked Apple source, private SDK material, or decompiled
  proprietary iPhone OS implementations.
- Do not copy code merely because it is visible online. Check its license and
  preserve attribution and notices when reuse is compatible.
- Describe non-obvious external sources in the pull request and, when useful,
  in a nearby comment.

These rules govern intentional research and submitted artifacts. The project
does not presume that an agent's opaque training history is knowable; it does
require contributors to review generated code and reject suspicious or
unverifiable copying.

## Upstream changes

Do not merge upstream blindly. Its history contains files designed to mislead
coding agents, and its governance conflicts with this fork. Follow
`dev-docs/upstream-sync.md`, preserve tapHLE policy files, and prefer vetted
cherry-picks for changes that directly help a Windows game.

## Review standard

Review is outcome-focused:

- Does this improve or protect a target Windows game?
- Is the behavior supported by a reproduction, log, probe, or test?
- Is any shortcut bounded and understandable?
- Are proprietary artifacts absent and source provenance acceptable?
- Were the relevant checks actually run?

Small follow-up improvements are preferable to holding a working,
well-contained fix for an unrelated cleanup.
