# Compatibility database protocol

The tapHLE compatibility database records reproducible Windows observations
for exact early iPhone OS app builds. It does not contain apps. Each record in
`compatibility/apps` maps an app to an exact bundle identity, one or more exact
Archive.org IPA filenames, and an append-only sequence of reports. The root
`COMPATIBILITY.md` is generated from those records.

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
4. Keep the IPA outside Git. Match the local file's MD5 and SHA-1 to the item
   metadata and record a locally calculated SHA-256. A renamed local file must
   be associated with an explicit exact archive filename. A similarly named
   or same-sized file is not equivalent.
5. Read `Payload/*.app/Info.plist` from the hash-matched IPA and verify the
   bundle identifier, bundle version, optional short version, and minimum OS
   version. Cross-check the same local file with `tapHLE --info` when a built
   executable is available.
6. Only after those checks may `archive_org.verification.state` be
   `content-hash-verified`, and only a content-hash-verified file may support a
   compatibility report.

The dependency-free helper performs steps 1, 3, 4, and 5 against a record. It
uses only Python's `urllib`, `zipfile`, `plistlib`, `hashlib`, and related
standard-library modules. Network access occurs only for this explicit
command; it never searches Archive.org and never downloads the IPA:

```powershell
python .\dev-scripts\compatibility.py verify-archive ricky `
  --bundle-version 2.1 `
  --ipa 'C:\private\Ricky (v2.1) [Cracked].ipa' `
  --taphle-exe .\target\release\tapHLE.exe
```

If a local file was renamed, add
`--archive-filename 'the exact Archive.org filename.ipa'`. Hash matching is
still mandatory. A failed check means the artifact cannot support a verified
report.

For a manual `tapHLE --info` cross-check, use:

```powershell
.\target\release\tapHLE.exe 'C:\private\Game.ipa' --info
```

## App compatibility branch workflow

Work on an app in `compat/<app-slug>`; for example, Ricky work belongs on
`compat/ricky`. Exploratory checkpoint commits are allowed on that branch so
investigations are reproducible and do not depend on a dirty worktree.

Merge or otherwise promote the branch only after:

1. an Archive content-hash-verified artifact produces a reproducible
   compatibility milestone on a committed tapHLE revision;
2. the exact achieved state and remaining blocker are appended to the
   database; and
3. the relevant normal regression checks pass.

Full playability is not required. A smaller verified milestone is useful when
the database states it honestly. Provisional results from a dirty worktree do
not enter the compatibility database. Commit the implementation checkpoint,
rerun the exact hash-verified IPA, and then append its report using that exact
commit hash. Preserve every commit named by a report when merging: use a
fast-forward or merge commit, not a squash or rebase that removes the tested
commit from `HEAD` history. If history must change, rerun the artifact on a new
preserved commit before recording the result.

## Editing and checking records

Create one lowercase, hyphenated file at
`compatibility/apps/<app-slug>.json`. Start from the fields in an existing
record and consult `compatibility/schema-v1.json`. Do not copy a previous
version's identity or hashes without verifying them.

Run the offline commands from the repository root:

```powershell
python .\dev-scripts\compatibility.py list
python .\dev-scripts\compatibility.py show ricky
python .\dev-scripts\compatibility.py render
python .\dev-scripts\compatibility.py check
python .\dev-scripts\compatibility.py check --baseline-ref origin/trunk
```

`check` validates exact identities, canonical URLs, hashes, report ordering,
the generated Markdown, and that every report's tapHLE commit exists and is an
ancestor of `HEAD`, without accessing the network. With `--baseline-ref`, it
also proves that existing reports are an unchanged prefix of the new report
list.

Never commit an IPA, extracted app, game asset, decryption key, save data,
personal path, or raw tapHLE log. Summarize only the minimum diagnostic facts
needed for the report. Keep local apps in the ignored `tapHLE_apps` directory
or another private path.
