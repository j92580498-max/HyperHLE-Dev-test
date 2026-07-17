# Efficient Windows app troubleshooting

This is the operational playbook for advancing a selected game without
repeating settled work. Read it with `AGENTS.md`, `agent-workflow.md`, and the
app's sanitized note under `dev-docs/app-notes/` when one exists.

## Resume from evidence, not from launch

Before building or running anything:

1. Check the current branch, worktree, recent commits, and remote tracking
   state.
2. Read the app's compatibility record and continuation note.
3. Identify the highest milestone already reproduced on a committed build.
4. Write down the one next observable boundary. Examples are “menu tap reaches
   level select,” “the queue starts,” or “the first frame presents.”
5. Choose the cheapest observation that distinguishes the live hypotheses.

Do not replay the whole investigation merely to become familiar with it. Start
from the last trustworthy milestone and test only the next frontier unless a
code change could have regressed an earlier boundary.

Keep three small lists while working:

- **Proven:** directly supported by a hash-verified run, static inspection, or
  a deterministic test.
- **Rejected:** hypotheses contradicted by evidence, including why.
- **Next discriminator:** one bounded trace, test, or input that decides what
  to implement next.

If work crosses agents or sessions, update the app note before stopping. The
note is a continuation aid, not a compatibility claim.

## Freeze the artifact identity first

For an Archive-backed target, follow `compatibility/README.md` exactly. Use the
item URL supplied by the maintainer; never search for a substitute.

Before every report-worthy run:

```powershell
Get-FileHash -Algorithm SHA256 -LiteralPath '<exact local IPA>'
```

The local hash must match the canonical Archive metadata already recorded for
that exact filename. If it does not, stop using that local copy. Confirm the
embedded bundle identifier/version and `tapHLE --info` output once per artifact
and record the result in the app note. A copied filename is not identity.

Do not duplicate IPAs between run directories. Keep them outside Git and pass
their absolute path to tapHLE.

## Follow an evidence ladder

Use the lowest-cost layer that can answer the current question:

1. Existing log or compatibility observation.
2. Source search for the exact symbol, status code, selector, or format.
3. Static inspection of the authorized app to recover a branch condition,
   call target, UI hit box, or faulting symbol.
4. A focused unit or TestApp regression.
5. A bounded diagnostic build and one controlled Windows run.
6. A clean committed release build and exact-artifact milestone run.

Static inspection and runtime tracing complement each other. Static work is
often faster for questions such as “what input advances this screen?” Runtime
work is required for questions such as “does that input reach the next level?”
Do not guess at hidden buttons, timers, formats, or ABI behavior through dozens
of blind runs when one branch condition or structure dump can answer it.

Keep static work equally bounded. Extract only the embedded `Info.plist` and
executable needed to answer the named question into a unique `%TEMP%`
directory; do not unpack an entire library of apps. Record the summarized
condition or symbol, then remove the exact temporary directory when it is no
longer needed.

For a guest crash, resolve the program counter to its owning app image,
library, or HLE boundary before designing a fix. Inspect the register values
and object/allocation lifetime around that exact operation. A native library
fault, managed-code fault, and emulator panic have different recovery paths.

## Instrument one boundary at a time

Temporary diagnostics should answer one named question and produce little
output. Gate them by the exact format, API, queue, or bundle when practical.

Good diagnostic fields include:

- an API's input structure and status code;
- buffer byte size, count, pointer route, and a four-byte signature;
- the first few packet descriptors or offsets;
- one allocation size/address and its allocate/free/reuse events; or
- one UI phase, coordinate, and resulting level/screen transition.

Do not dump entire audio buffers, textures, guest memory, app binaries, or raw
logs. Never commit a diagnostic containing proprietary bytes or personal
paths. After the question is answered, remove trace-only code or convert the
small useful failure message into a normal diagnostic.

Read structures before reinterpreting data. For compressed audio, for example,
capture the `AudioStreamBasicDescription`, packet-description route/count, and
the first sync bytes before treating the buffer as PCM. For a stale-pointer
crash, prove allocation/free/reuse behavior before disabling an allocator rule
globally.

Use existing dependencies and subsystem abstractions before introducing a new
decoder, parser, or host library.

## Make Windows runs isolated and repeatable

Use a uniquely named directory under `%TEMP%` as tapHLE's working directory.
Link its `tapHLE_dylibs` and `tapHLE_fonts` to the checkout and hard-link the
tracked default-options file rather than copying large trees. Do not add local
options unless the experiment is specifically testing one.

For each meaningful run, record:

- exact tapHLE commit or that the build was dirty;
- verified IPA hash;
- tapHLE arguments and options source;
- client-area input coordinates and timing;
- last screen/event reached and whether the process stayed alive; and
- the narrow log lines supporting the conclusion.

Use real foreground mouse input when Windows message injection does not reach
SDL. Save and restore the previous cursor position and foreground window, and
stop only the exact process launched by the run. Visual inspection of a
screenshot can support a rendering observation, but the screenshot stays in
the ignored temporary directory.

Reuse a proven input recipe. Change one step at the frontier rather than
inventing a new route on every launch. A successful click path is evidence and
belongs in the app note.

## Conserve time and disk

- Check free space before a large build or extraction when the drive has been
  under pressure.
- Reuse Cargo's incremental/target cache; do not run `cargo clean` as routine
  troubleshooting.
- Build once for a batch of trace questions instead of rebuilding after every
  log line.
- Filter a large log for the exact symbol, status code, or subsystem plus a few
  context lines; do not repeatedly reread or paste the full file.
- Prefer junctions/hard links over copied runtime support trees.
- Remove only the exact temporary extraction/run directories created for the
  completed experiment. Never use a broad wildcard or an unresolved path.
- Keep the small sanitized conclusion, not gigabytes of intermediate data.

## Choose and bound the fix

The fix may be general, intentionally partial, or game-specific. Prefer the
smallest complete behavior supported by evidence.

- A general implementation should validate its inputs and fail safely for
  unsupported variants.
- A partial implementation should state the exact supported shape and leave a
  visible fallback/TODO for other routes.
- A game workaround should be gated as narrowly as possible and explain the
  observed behavior it preserves.

Do not broaden scope to adjacent formats, platforms, or APIs just because they
are nearby. An implementation for the route the target actually uses is a
valid checkpoint. Add a synthetic regression that contains no proprietary
fixture whenever deterministic logic can be isolated.

## Commit, then make compatibility claims

A dirty-worktree run is a useful experiment but never database evidence.

1. Commit the focused implementation on `compat/<app-slug>`.
2. Build and run the relevant tests from that exact commit.
3. Re-verify the IPA hash and replay the milestone on Windows.
4. If the milestone is reproducible, append a compatibility report referencing
   the tested implementation commit.
5. Commit the generated compatibility view separately when practical.
6. Push checkpoints so another agent can resume them; promote to `trunk` only
   at the documented milestone boundary.

Keep implementation, agent-policy documentation, and compatibility reports in
separable commits. This makes incomplete app experiments easy to continue or
revert without losing durable process improvements.

## Continuation-note template

Create `dev-docs/app-notes/<app-slug>.md` on the app branch when work will span
more than one focused turn. Keep it sanitized and concise:

```markdown
# <App> compatibility work note

- Branch and last pushed commit:
- Canonical artifact URL, filename, SHA-256, bundle ID/version:
- Highest clean committed milestone:
- Proven input recipe:
- Proven facts:
- Rejected hypotheses:
- Current uncommitted diagnostics or code:
- Checks already run:
- Known risks/regressions to watch:
- Next discriminator or implementation step:
```

Do not put app code, decompiled listings, assets, screenshots, raw logs, keys,
or personal paths in the note. Exact public provenance, hashes, symbol names,
addresses, API shapes, and summarized behavior are sufficient for continuity.
