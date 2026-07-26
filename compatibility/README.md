# Compatibility database protocol

The tapHLE compatibility database records reproducible Windows observations for
exact early iPhone OS app builds. It does not contain apps. A result ties an app
to an exact bundle identity, the exact artifact tested, and a dated rating on a
committed tapHLE revision.

This document is the protocol: what a result must satisfy before it may be
recorded, and how an Archive.org-backed artifact is verified. Those rules are
the same wherever the result is stored. The next section says where that is.

## Where the database lives

The compatibility database is a live web application, **tapHLEdb** — a fork of
[app-compatibility-db](https://github.com/hikari-no-yume/app-compatibility-db),
the same app that powers touchHLE's database — self-hosted by the maintainer at
**<https://taphle.ephun.net/compatibility>**. Its source is
[ephun/tapHLEdb](https://github.com/ephun/tapHLEdb). A live application is the
right shape for this data: it is edited continuously by humans, coding agents
and (later) tapHLE telemetry, which does not fit a commit-per-edit Git workflow.

That deployment is now the record of tapHLE compatibility. Every new result goes
there. The JSON records still in `compatibility/apps` predate it and remain only
until the maintainer migrates them; do not add records there.

The rest of this document still applies to both. The Archive.org verification
protocol, the testing policy, and the rule that a rating requires an actual
tapHLE run on Windows are properties of how a result is earned, not of where it
is stored.

What lives where, so the two never duplicate each other:

- **tapHLEdb — the database.** Structured data only: an app's identity, its
  versions, and dated reports carrying a 1–5 rating, the tapHLE version, the
  Windows host, the source of the result, and a one-line frontier. Each report
  is a dated snapshot and is never revised, so its frontier records where the
  app stood *at that commit*. It holds no narrative. It answers *"where does
  this app stand?"*
- **`dev-docs/app-notes/<app>.md` — the notebook.** The debugging narrative:
  evidence, root causes and the next discriminator, kept current. Its frontier
  is where the app stops *now*. It answers *"how do I push this app further?"*
  It is explicitly not a compatibility claim.

The two frontiers are different facts, not duplicates: one is history, one is
present. Read the database to see how an app is doing; read the note to continue
its work.

A record exists only because tapHLE actually ran that app and produced a rating.
Results from touchHLE or HyperHLE are testing leads, never imported ratings, and
apps are never listed speculatively.

## How an agent records a result

A coding agent submits through tapHLEdb's token-authenticated endpoint:

```
POST https://taphle.ephun.net/compatibility/api/report
```

It is documented in `API.md` in the tapHLEdb repository. The agent token lives
at `~/.taphledb-token` and nowhere else: read it inline as
`$(cat ~/.taphledb-token)` at the moment of use, never echo it, and never copy
it into this repository, a commit message, an app note, or any command whose
output is recorded. If it is missing, say so and keep working. Submissions
always land unapproved and appear publicly only after the maintainer approves
them.

Crossing a star threshold does two things, not one: the reusable fix graduates
to `trunk` *and* the report goes to the database. Do only the first and a real
result stays invisible; do only the second and the claim cannot be reproduced.

### Threshold-publication closeout

Do not call a new rating complete until all four conditions hold:

1. A content-hash-verified artifact has reproduced the milestone on a committed
   implementation revision.
2. `POST /api/report` has accepted the report for that exact revision. A
   `pending_moderation` response is successful submission; it is not a reason
   to wait for approval before continuing.
3. The compatibility checkpoint has been merged into `trunk` without removing
   the tested revision, and `trunk` has been pushed to `origin`.
4. `git merge-base --is-ancestor <tested-commit> origin/trunk` exits zero after
   the push.

A pushed `compat/<app-slug>` branch is a checkpoint, not threshold completion.
If canonical provenance or the submission token is missing, record that exact
blocker and keep threshold publication open; do not silently leave the report
or the `trunk` promotion undone.

**Run `tapHLE --info` before composing a report, every time.** The bundle
identifier is the field the endpoint matches on, so guessing it silently
creates a duplicate app row that a moderator then has to reject, and the
report cannot be edited afterwards — only superseded. Real identifiers are
routinely not what the app's name suggests: two apps on the 2026-07-26 target
list turned out to be plain `Minecrafted` and `com.eeenmachine.` (with a
trailing dot). Copy the identifier and version out of `--info` output; never
infer them from the app name, the Archive filename, or the developer.

Submit when the rating changes, in either direction — a regression is a result.
Do not submit a rerun that reproduces a rating already recorded for the same
tapHLE revision; the endpoint does not deduplicate, so that is pure moderation
noise.

To choose what to work on, read the list:

```
GET https://taphle.ephun.net/compatibility/api/apps
```

No credential is needed. The lowest-rated apps need the most help, and an app
listed there with no `compat/<slug>` branch is unclaimed work an agent may start
without being asked.

An agent may assign at most **three stars**: two when the app reaches a stable
screen, and three when the gameplay loop demonstrably starts and persists for a
short while. **Four and five stars require human testing**, and an agent must
never assign them.

## Simple rating scale

The public list uses five simple levels:

- ★☆☆☆☆ (1/5) Broken — The game does not reach usable content.
- ★★☆☆☆ (2/5) Starts — An intro or menu works, but gameplay does not.
- ★★★☆☆ (3/5) In game — Some gameplay works, but major problems remain.
- ★★★★☆ (4/5) Playable — The whole game can be played, with small problems.
- ★★★★★ (5/5) Fully working — Everything important works.
- — Not tested — There is no verified tapHLE Windows result.

The filled and empty stars and numeric score are only a short summary. The
exact report, feature states, app file, tapHLE commit, and Windows host say what
was really tested. `boots` and `menu` both display as two stars, while the
stored status keeps the difference.

The scale is adapted from the [touchHLE app database](https://appdb.touchhle.org/),
whose database content is published under the Creative Commons Attribution 4.0
license. Results from touchHLE or HyperHLE are useful testing leads, but they do
not become tapHLE ratings until the exact app file is hash-checked and run with
a committed tapHLE build on Windows.

## Project testing policy

tapHLE may compatibility-test a build that the maintainer has determined in
good faith to be genuinely unavailable or abandoned when there is no current
App Store market alternative for that build. The database may reference the
canonical Archive.org item and exact IPA filename used for a test.

That is a project scope decision, not a blanket claim that “abandonware” is a
legal category or that every archived copy may be downloaded or redistributed.
A public archive listing alone is not proof of legal status. Do not use this
policy to substitute for a game that is actively sold or otherwise offered by
its rightsholder. Respect DMCA notices and rightsholder requests. Stop testing
and alert the maintainer if an item is removed, restricted, disputed, or gains
a current legitimate market alternative.

Before adding a source or a new report, re-check availability and update the
record's `availability.checked_at`, status, and factual notes. The maintainer
makes the final project-scope decision. Contributors remain responsible for
following the law that applies to them. The offline validator rejects a report
dated after the record's most recent availability check.

## What one record means

An exact version is identified by:

- `CFBundleIdentifier` (`bundle_identifier`);
- `CFBundleVersion` (`bundle_version`);
- `CFBundleShortVersionString` when the IPA contains one (`short_version`);
  and
- `MinimumOSVersion` (`minimum_os_version`).

The Archive.org source repeats the version identity deliberately. The offline
validator rejects any mismatch. Every source also contains the exact item
identifier, its canonical `https://archive.org/details/<identifier>` URL, and
the exact IPA filename and hashes. Multiple original filenames may be listed
only when each has been verified; exactly one is marked as the tested file.

Reports apply only to that exact artifact, tapHLE commit, Windows host, and
date. `booted` records whether the application lifecycle began independently
of the overall status. Feature values remain `unknown` until they were
actually exercised.

Reports are immutable and append-only. Never rewrite a previous observation
because a later commit works better. Append a report. If an old report needs a
correction, append a report with `supersedes` and explain it; the validator can
compare a branch with a Git baseline to prevent removal or mutation.

## Exact Archive.org verification protocol

Follow these steps in order. Agents must use the Archive.org item URL supplied
by the maintainer or reporter. Do not search for, guess, or “grope around” for
an item when no exact URL was supplied.

1. Confirm that the URL has exactly the canonical form
   `https://archive.org/details/<identifier>`. Record both the identifier and
   that URL without redirects, query strings, fragments, or alternate hosts.
2. Re-check the app's current market availability under the project policy
   above. Record the date and a narrow factual result; do not present the
   result as a legal conclusion.
3. Fetch only `https://archive.org/metadata/<identifier>` to verify that each
   exact IPA filename exists as an original file and to capture its published
   MD5 and SHA-1. Never select a file merely because its name looks similar.
4. Only after step 3 succeeds, an agent may download the exact
   maintainer-designated original to an external cache outside the repository.
   Do not extract it or open it as an IPA while it is downloading or before the
   hash gate in step 5 passes.
5. Keep the IPA outside Git. Match the local file's MD5 and SHA-1 to the item
   metadata and record a locally calculated SHA-256. The record always retains
   the metadata's exact `ipa_filename`; a Windows-safe local cache name is only
   a path alias. A similarly named or same-sized file is not equivalent.
6. Read `Payload/*.app/Info.plist` from the hash-matched IPA and verify the
   bundle identifier, bundle version, optional short version, and minimum OS
   version. Cross-check the same local file with `tapHLE --info` when a built
   executable is available.
7. Only after those checks may `archive_org.verification.state` be
   `content-hash-verified`, and only a content-hash-verified file may support a
   compatibility report.

The dependency-free helper performs steps 1, 3, 5, and 6 against a record. It
uses only Python's `urllib`, `zipfile`, `plistlib`, `hashlib`, and related
standard-library modules. Network access occurs only for this explicit
command; it never searches Archive.org and never downloads the IPA. The exact
original download authorized in step 4 must already be complete:

```powershell
python .\dev-scripts\compatibility.py verify-archive ricky `
  --bundle-version 2.1 `
  --ipa 'C:\private\Ricky (v2.1) [Cracked].ipa' `
  --taphle-exe .\target\release\tapHLE.exe
```

If the local Windows-safe filename differs, add
`--archive-filename 'the exact Archive.org filename.ipa'`. This maps the local
path to the canonical filename; it does not rename the database artifact. Hash
matching is still mandatory. A failed check means agents must not inspect,
execute, or use that local copy, even for a provisional diagnosis. Find the
exact canonical copy through the maintainer rather than treating the mismatch
as “close enough.” Re-run verification before each report-worthy clean run.

For a manual `tapHLE --info` cross-check, use:

```powershell
.\target\release\tapHLE.exe 'C:\private\Game.ipa' --info
```

## App compatibility branch workflow

Work on an app in `compat/<app-slug>`; for example, Ricky work belongs on
`compat/ricky`. Exploratory checkpoint commits are allowed on that branch so
investigations are reproducible and do not depend on a dirty worktree.
When publishing is authorized, push useful checkpoints to the matching remote
branch so another agent can resume them. Never force-push or otherwise rewrite
a commit referenced by a compatibility report.

Merge or otherwise promote the branch only after:

1. an Archive content-hash-verified artifact produces a reproducible
   compatibility milestone on a committed tapHLE revision;
2. the exact achieved state and remaining blocker are appended to the
   database; and
3. the relevant normal regression checks pass.

Meeting these gates defines a stable compatibility checkpoint. Merge it to
`trunk` with its limitations recorded even when more app work remains. Leave
unfinished, unverified, or unstable experiments on the compatibility branch.
Full playability is not required. A smaller verified milestone is useful when
the database states it honestly. Provisional results from a dirty worktree do
not enter the compatibility database. Commit the implementation checkpoint,
rerun the exact hash-verified IPA, and then append its report using that exact
commit hash. Preserve every commit named by a report when merging: use a
fast-forward or merge commit, not a squash or rebase that removes the tested
commit from `HEAD` history. If history must change, rerun the artifact on a new
preserved commit before recording the result.

## Editing and checking records

**Do not create new records here.** New results go to the live database; see
"How an agent records a result" above. This section covers the legacy
`compatibility/apps/*.json` records, which remain readable and checkable until
they are migrated, and `compatibility/schema-v1.json` documents their shape.

Run the offline commands from the repository root:

```powershell
python .\dev-scripts\compatibility.py list
python .\dev-scripts\compatibility.py show ricky
python .\dev-scripts\compatibility.py check
python .\dev-scripts\compatibility.py check --baseline-ref origin/trunk
```

`check` validates exact identities, canonical URLs, hashes, report ordering,
and that every report's tapHLE commit exists and is an ancestor of `HEAD`,
without accessing the network. With `--baseline-ref`, it also proves that
existing reports are an unchanged prefix of the new report list.

Never commit an IPA, extracted app, game asset, decryption key, save data,
personal path, or raw tapHLE log. Summarize only the minimum diagnostic facts
needed for the report. Keep local apps in the ignored `tapHLE_apps` directory
or another private path.
