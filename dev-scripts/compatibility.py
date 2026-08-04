#!/usr/bin/env python3
"""Validate, render, and inspect tapHLE compatibility records.

The normal ``check`` command is deliberately offline. Network access and IPA
inspection happen only through the explicitly requested ``verify-archive``
command.
"""

from __future__ import annotations

import argparse
import datetime as dt
import hashlib
import json
from pathlib import Path
import plistlib
import re
import subprocess
import sys
import urllib.error
import urllib.parse
import urllib.request
import zipfile


SCRIPT_PATH = Path(__file__).resolve()
DEFAULT_ROOT = SCRIPT_PATH.parent.parent
SCHEMA_REFERENCE = "../schema-v1.json"
ARCHIVE_DETAILS_PREFIX = "https://archive.org/details/"
ARCHIVE_METADATA_PREFIX = "https://archive.org/metadata/"
IDENTIFIER_RE = re.compile(r"^[A-Za-z0-9._-]+$")
SLUG_RE = re.compile(r"^[a-z0-9]+(?:-[a-z0-9]+)*$")
HASH_RE = {
    "md5": re.compile(r"^[0-9a-f]{32}$"),
    "sha1": re.compile(r"^[0-9a-f]{40}$"),
    "sha256": re.compile(r"^[0-9a-f]{64}$"),
}
COMMIT_RE = re.compile(r"^[0-9a-f]{40}$")
REPORT_ID_RE = re.compile(r"^[0-9]{4}-[0-9]{2}-[0-9]{2}-[a-z0-9-]+$")
STATUS_LABELS = {
    "launch-blocked": "Launch blocked",
    "boots": "Boots",
    "menu": "Reaches menu",
    "in-game": "In game",
    "playable-with-issues": "Playable with issues",
    "playable": "Playable",
}
STATUS_RATINGS = {
    "launch-blocked": (1, "Broken"),
    "boots": (2, "Starts"),
    "menu": (2, "Starts"),
    "in-game": (3, "In game"),
    "playable-with-issues": (4, "Playable"),
    "playable": (5, "Fully working"),
}
assert STATUS_LABELS.keys() == STATUS_RATINGS.keys()
FEATURE_STATES = {"unknown", "broken", "partial", "working", "not-applicable"}
FEATURE_NAMES = ("graphics", "audio", "input", "saving", "network")
ARCHIVE_STATES = {"content-hash-verified", "metadata-only", "unverified"}
AVAILABILITY_STATES = {
    "maintainer-designated-unavailable",
    "unavailable-no-current-store-alternative",
    "current-market-alternative",
    "unknown",
    "rightsholder-restricted",
}


class CompatibilityError(Exception):
    """An expected validation or verification failure."""


def rating_label(status: str) -> str:
    rating, label = STATUS_RATINGS[status]
    star_bar = "★" * rating + "☆" * (5 - rating)
    return f"{star_bar} ({rating}/5) {label}"


def plain_rating_label(status: str) -> str:
    rating, label = STATUS_RATINGS[status]
    return f"{rating}/5 {label}"


def _keys(
    value: object,
    location: str,
    required: set[str],
    optional: set[str],
    errors: list[str],
) -> dict[str, object] | None:
    if not isinstance(value, dict):
        errors.append(f"{location}: expected an object")
        return None
    missing = sorted(required - value.keys())
    extra = sorted(value.keys() - required - optional)
    if missing:
        errors.append(f"{location}: missing fields: {', '.join(missing)}")
    if extra:
        errors.append(f"{location}: unexpected fields: {', '.join(extra)}")
    return value


def _string(
    value: object,
    location: str,
    errors: list[str],
    pattern: re.Pattern[str] | None = None,
) -> str | None:
    if not isinstance(value, str) or not value:
        errors.append(f"{location}: expected a non-empty string")
        return None
    if pattern is not None and pattern.fullmatch(value) is None:
        errors.append(f"{location}: invalid value {value!r}")
    return value


def _date(value: object, location: str, errors: list[str]) -> str | None:
    text = _string(value, location, errors)
    if text is None:
        return None
    try:
        parsed = dt.date.fromisoformat(text)
    except ValueError:
        errors.append(f"{location}: expected an ISO 8601 calendar date")
        return text
    if parsed.isoformat() != text:
        errors.append(f"{location}: date must use YYYY-MM-DD form")
    return text


def _hash(value: object, algorithm: str, location: str, errors: list[str]) -> str | None:
    return _string(value, location, errors, HASH_RE[algorithm])


def version_key(version: dict[str, object]) -> tuple[str, str, str | None]:
    identity = version["identity"]
    assert isinstance(identity, dict)
    return (
        str(identity.get("bundle_identifier", "")),
        str(identity.get("bundle_version", "")),
        identity.get("short_version") if isinstance(identity.get("short_version"), str) else None,
    )


def validate_record(record: object, path: Path) -> list[str]:
    errors: list[str] = []
    root = _keys(
        record,
        str(path),
        {"$schema", "schema_version", "slug", "title", "versions"},
        set(),
        errors,
    )
    if root is None:
        return errors

    if root.get("$schema") != SCHEMA_REFERENCE:
        errors.append(f"{path}.$schema: must be {SCHEMA_REFERENCE!r}")
    if root.get("schema_version") != 1:
        errors.append(f"{path}.schema_version: must be 1")
    slug = _string(root.get("slug"), f"{path}.slug", errors, SLUG_RE)
    _string(root.get("title"), f"{path}.title", errors)
    if slug is not None and path.name != f"{slug}.json":
        errors.append(f"{path}: filename must be {slug}.json")

    versions = root.get("versions")
    if not isinstance(versions, list) or not versions:
        errors.append(f"{path}.versions: expected a non-empty array")
        return errors

    seen_versions: set[tuple[str, str, str | None]] = set()
    seen_report_ids: set[str] = set()
    previous_version_key: tuple[str, str, str] | None = None
    for version_index, version_value in enumerate(versions):
        where = f"{path}.versions[{version_index}]"
        version = _keys(
            version_value,
            where,
            {"identity", "archive_org", "reports"},
            set(),
            errors,
        )
        if version is None:
            continue

        identity = _keys(
            version.get("identity"),
            f"{where}.identity",
            {"bundle_identifier", "bundle_version", "minimum_os_version"},
            {"short_version"},
            errors,
        )
        if identity is None:
            continue
        bundle_id = _string(
            identity.get("bundle_identifier"), f"{where}.identity.bundle_identifier", errors
        )
        bundle_version = _string(
            identity.get("bundle_version"), f"{where}.identity.bundle_version", errors
        )
        minimum_os = _string(
            identity.get("minimum_os_version"),
            f"{where}.identity.minimum_os_version",
            errors,
        )
        short_version = None
        if "short_version" in identity:
            short_version = _string(
                identity.get("short_version"), f"{where}.identity.short_version", errors
            )
        key = (bundle_id or "", bundle_version or "", short_version)
        if key in seen_versions:
            errors.append(f"{where}: duplicate exact version identity {key!r}")
        seen_versions.add(key)
        sortable_key = (key[0], key[1], key[2] or "")
        if previous_version_key is not None and sortable_key <= previous_version_key:
            errors.append(f"{where}: versions must be sorted by exact identity")
        previous_version_key = sortable_key

        archive = _keys(
            version.get("archive_org"),
            f"{where}.archive_org",
            {
                "identifier",
                "item_url",
                "bundle_identifier",
                "bundle_version",
                "files",
                "verification",
                "availability",
            },
            {"short_version"},
            errors,
        )
        archive_files: dict[str, dict[str, object]] = {}
        archive_state = None
        availability_checked_at = None
        if archive is not None:
            identifier = _string(
                archive.get("identifier"),
                f"{where}.archive_org.identifier",
                errors,
                IDENTIFIER_RE,
            )
            item_url = _string(
                archive.get("item_url"), f"{where}.archive_org.item_url", errors
            )
            if identifier is not None:
                canonical_url = f"{ARCHIVE_DETAILS_PREFIX}{identifier}"
                if item_url != canonical_url:
                    errors.append(
                        f"{where}.archive_org.item_url: must be canonical URL {canonical_url!r}"
                    )
            if archive.get("bundle_identifier") != bundle_id:
                errors.append(
                    f"{where}.archive_org.bundle_identifier: must exactly match version identity"
                )
            if archive.get("bundle_version") != bundle_version:
                errors.append(
                    f"{where}.archive_org.bundle_version: must exactly match version identity"
                )
            archive_short_present = "short_version" in archive
            identity_short_present = "short_version" in identity
            if archive_short_present != identity_short_present or (
                archive_short_present and archive.get("short_version") != short_version
            ):
                errors.append(
                    f"{where}.archive_org.short_version: presence and value must exactly match version identity"
                )

            files = archive.get("files")
            if not isinstance(files, list) or not files:
                errors.append(f"{where}.archive_org.files: expected a non-empty array")
            else:
                tested_count = 0
                previous_filename = None
                for file_index, file_value in enumerate(files):
                    file_where = f"{where}.archive_org.files[{file_index}]"
                    file_record = _keys(
                        file_value,
                        file_where,
                        {"ipa_filename", "md5", "sha1", "sha256", "tested"},
                        set(),
                        errors,
                    )
                    if file_record is None:
                        continue
                    filename = _string(
                        file_record.get("ipa_filename"), f"{file_where}.ipa_filename", errors
                    )
                    if filename is not None:
                        if "/" in filename or "\\" in filename or not filename.lower().endswith(".ipa"):
                            errors.append(
                                f"{file_where}.ipa_filename: must be an exact IPA filename, not a path"
                            )
                        if filename in archive_files:
                            errors.append(f"{file_where}.ipa_filename: duplicate filename")
                        archive_files[filename] = file_record
                        if previous_filename is not None and filename.casefold() <= previous_filename.casefold():
                            errors.append(
                                f"{file_where}: archive filenames must be sorted case-insensitively"
                            )
                        previous_filename = filename
                    _hash(file_record.get("md5"), "md5", f"{file_where}.md5", errors)
                    _hash(file_record.get("sha1"), "sha1", f"{file_where}.sha1", errors)
                    _hash(file_record.get("sha256"), "sha256", f"{file_where}.sha256", errors)
                    if not isinstance(file_record.get("tested"), bool):
                        errors.append(f"{file_where}.tested: expected a boolean")
                    elif file_record["tested"]:
                        tested_count += 1
                if tested_count != 1:
                    errors.append(
                        f"{where}.archive_org.files: exactly one exact IPA filename must be marked tested"
                    )

            verification = _keys(
                archive.get("verification"),
                f"{where}.archive_org.verification",
                {"state", "checked_at", "notes"},
                set(),
                errors,
            )
            if verification is not None:
                archive_state = verification.get("state")
                if archive_state not in ARCHIVE_STATES:
                    errors.append(
                        f"{where}.archive_org.verification.state: invalid state {archive_state!r}"
                    )
                _date(
                    verification.get("checked_at"),
                    f"{where}.archive_org.verification.checked_at",
                    errors,
                )
                _string(
                    verification.get("notes"),
                    f"{where}.archive_org.verification.notes",
                    errors,
                )

            availability = _keys(
                archive.get("availability"),
                f"{where}.archive_org.availability",
                {"checked_at", "status", "notes"},
                set(),
                errors,
            )
            if availability is not None:
                availability_checked_at = _date(
                    availability.get("checked_at"),
                    f"{where}.archive_org.availability.checked_at",
                    errors,
                )
                if availability.get("status") not in AVAILABILITY_STATES:
                    errors.append(
                        f"{where}.archive_org.availability.status: invalid status {availability.get('status')!r}"
                    )
                _string(
                    availability.get("notes"),
                    f"{where}.archive_org.availability.notes",
                    errors,
                )

        reports = version.get("reports")
        if not isinstance(reports, list):
            errors.append(f"{where}.reports: expected an array")
            continue
        previous_report_order: tuple[str, str] | None = None
        version_report_ids: set[str] = set()
        for report_index, report_value in enumerate(reports):
            report_where = f"{where}.reports[{report_index}]"
            report = _keys(
                report_value,
                report_where,
                {
                    "id",
                    "tested_at",
                    "taphle_commit",
                    "host",
                    "artifact",
                    "status",
                    "booted",
                    "summary",
                    "milestones",
                    "blocker",
                    "features",
                },
                {"supersedes", "notes"},
                errors,
            )
            if report is None:
                continue
            report_id = _string(report.get("id"), f"{report_where}.id", errors, REPORT_ID_RE)
            tested_at = _date(report.get("tested_at"), f"{report_where}.tested_at", errors)
            if (
                tested_at is not None
                and availability_checked_at is not None
                and tested_at > availability_checked_at
            ):
                errors.append(
                    f"{report_where}.tested_at: availability must be re-checked on or after this report"
                )
            if report_id is not None:
                if report_id in seen_report_ids:
                    errors.append(f"{report_where}.id: duplicate report ID")
                seen_report_ids.add(report_id)
                version_report_ids.add(report_id)
            if report_id is not None and tested_at is not None:
                order = (tested_at, report_id)
                if previous_report_order is not None and order <= previous_report_order:
                    errors.append(f"{report_where}: reports must be appended in date/ID order")
                previous_report_order = order
            _string(
                report.get("taphle_commit"),
                f"{report_where}.taphle_commit",
                errors,
                COMMIT_RE,
            )
            _validate_host(report.get("host"), f"{report_where}.host", errors)
            _validate_artifact(
                report.get("artifact"),
                f"{report_where}.artifact",
                archive_files,
                archive_state,
                errors,
            )
            status = report.get("status")
            if status not in STATUS_LABELS:
                errors.append(f"{report_where}.status: invalid status {status!r}")
            if not isinstance(report.get("booted"), bool):
                errors.append(f"{report_where}.booted: expected a boolean")
            _string(report.get("summary"), f"{report_where}.summary", errors)
            _string(report.get("blocker"), f"{report_where}.blocker", errors)
            milestones = report.get("milestones")
            if not isinstance(milestones, list):
                errors.append(f"{report_where}.milestones: expected an array")
            else:
                for milestone_index, milestone in enumerate(milestones):
                    _string(
                        milestone,
                        f"{report_where}.milestones[{milestone_index}]",
                        errors,
                    )
            _validate_features(report.get("features"), f"{report_where}.features", errors)
            if "supersedes" in report:
                supersedes = _string(
                    report.get("supersedes"), f"{report_where}.supersedes", errors
                )
                if supersedes not in version_report_ids - {report_id}:
                    errors.append(
                        f"{report_where}.supersedes: must name an earlier report for this exact version"
                    )
            if "notes" in report:
                _string(report.get("notes"), f"{report_where}.notes", errors)

    return errors


def _validate_host(value: object, location: str, errors: list[str]) -> None:
    host = _keys(
        value,
        location,
        {"os", "os_version", "architecture", "cpu", "gpu"},
        set(),
        errors,
    )
    if host is None:
        return
    if host.get("os") != "Windows":
        errors.append(f"{location}.os: tapHLE compatibility reports must be Windows tests")
    for field in ("os_version", "architecture", "cpu", "gpu"):
        _string(host.get(field), f"{location}.{field}", errors)


def _validate_artifact(
    value: object,
    location: str,
    archive_files: dict[str, dict[str, object]],
    archive_state: object,
    errors: list[str],
) -> None:
    artifact = _keys(
        value,
        location,
        {"archive_ipa_filename", "sha1", "verification"},
        set(),
        errors,
    )
    if artifact is None:
        return
    filename = _string(artifact.get("archive_ipa_filename"), f"{location}.archive_ipa_filename", errors)
    sha1 = _hash(artifact.get("sha1"), "sha1", f"{location}.sha1", errors)
    verification = artifact.get("verification")
    if verification != "archive-content-hash":
        errors.append(
            f"{location}.verification: reports require exact Archive.org content-hash verification"
        )
    if archive_state != "content-hash-verified":
        errors.append(
            f"{location}: report cannot be verified while the Archive.org source is {archive_state!r}"
        )
    source_file = archive_files.get(filename or "")
    if source_file is None:
        errors.append(f"{location}.archive_ipa_filename: not present in archive_org.files")
    else:
        if source_file.get("tested") is not True:
            errors.append(
                f"{location}.archive_ipa_filename: reports must use the exact source file marked tested"
            )
        if sha1 is not None and source_file.get("sha1") != sha1:
            errors.append(f"{location}.sha1: must exactly match the selected Archive.org file")


def _validate_features(value: object, location: str, errors: list[str]) -> None:
    features = _keys(value, location, set(FEATURE_NAMES), set(), errors)
    if features is None:
        return
    for name in FEATURE_NAMES:
        if features.get(name) not in FEATURE_STATES:
            errors.append(f"{location}.{name}: invalid feature state {features.get(name)!r}")


def load_records(root: Path) -> list[tuple[Path, dict[str, object]]]:
    apps_dir = root / "compatibility" / "apps"
    if not apps_dir.is_dir():
        raise CompatibilityError(f"Missing compatibility app directory: {apps_dir}")
    paths = sorted(apps_dir.glob("*.json"), key=lambda path: path.name)
    if not paths:
        raise CompatibilityError("The compatibility database has no app records")
    records: list[tuple[Path, dict[str, object]]] = []
    for path in paths:
        try:
            value = json.loads(path.read_text(encoding="utf-8"))
        except (OSError, UnicodeError, json.JSONDecodeError) as error:
            raise CompatibilityError(f"Could not read {path}: {error}") from error
        if not isinstance(value, dict):
            raise CompatibilityError(f"{path}: top-level JSON value must be an object")
        records.append((path, value))
    return records


def validate_database(root: Path) -> list[tuple[Path, dict[str, object]]]:
    schema_path = root / "compatibility" / "schema-v1.json"
    try:
        json.loads(schema_path.read_text(encoding="utf-8"))
    except (OSError, UnicodeError, json.JSONDecodeError) as error:
        raise CompatibilityError(f"Could not parse {schema_path}: {error}") from error

    records = load_records(root)
    errors: list[str] = []
    seen_slugs: set[str] = set()
    for path, record in records:
        errors.extend(validate_record(record, path.relative_to(root)))
        slug = record.get("slug")
        if isinstance(slug, str):
            if slug in seen_slugs:
                errors.append(f"{path.relative_to(root)}: duplicate slug {slug!r}")
            seen_slugs.add(slug)
        canonical = json.dumps(record, indent=2, ensure_ascii=False) + "\n"
        try:
            actual = path.read_text(encoding="utf-8")
        except (OSError, UnicodeError) as error:
            errors.append(f"{path.relative_to(root)}: could not re-read file: {error}")
        else:
            if actual != canonical:
                errors.append(
                    f"{path.relative_to(root)}: JSON must use canonical two-space formatting"
                )
    if errors:
        raise CompatibilityError("Compatibility database errors:\n- " + "\n- ".join(errors))
    return records


def _git(root: Path, arguments: list[str]) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        ["git", "-C", str(root), *arguments],
        check=False,
        capture_output=True,
        text=True,
        encoding="utf-8",
        errors="replace",
    )


def check_report_commits(
    root: Path,
    current: list[tuple[Path, dict[str, object]]],
    head_ref: str = "HEAD",
) -> None:
    references: list[tuple[str, str]] = []
    for path, record in current:
        versions = record.get("versions", [])
        assert isinstance(versions, list)
        for version_index, version in enumerate(versions):
            assert isinstance(version, dict)
            reports = version.get("reports", [])
            assert isinstance(reports, list)
            for report_index, report in enumerate(reports):
                assert isinstance(report, dict)
                commit = report.get("taphle_commit")
                # validate_database() reports malformed values. Keep this Git
                # check focused on already well-formed commit IDs.
                if not isinstance(commit, str) or COMMIT_RE.fullmatch(commit) is None:
                    continue
                try:
                    relative_path = path.relative_to(root)
                except ValueError:
                    relative_path = path
                location = (
                    f"{relative_path}.versions[{version_index}].reports[{report_index}]"
                    ".taphle_commit"
                )
                references.append((location, commit))

    if not references:
        return

    head = _git(root, ["rev-parse", "--verify", f"{head_ref}^{{commit}}"])
    if head.returncode != 0:
        raise CompatibilityError(f"Git reference {head_ref!r} does not resolve to a commit")
    head_commit = head.stdout.strip()

    errors: list[str] = []
    for location, commit in references:
        verify = _git(root, ["cat-file", "-e", f"{commit}^{{commit}}"])
        if verify.returncode != 0:
            errors.append(f"{location}: {commit} does not resolve to a Git commit")
            continue
        ancestor = _git(root, ["merge-base", "--is-ancestor", commit, head_commit])
        if ancestor.returncode == 1:
            errors.append(
                f"{location}: {commit} is not an ancestor of {head_ref}; "
                "report commits must remain in tapHLE history"
            )
        elif ancestor.returncode != 0:
            detail = ancestor.stderr.strip()
            errors.append(
                f"{location}: could not check ancestry against {head_ref}"
                + (f": {detail}" if detail else "")
            )
    if errors:
        raise CompatibilityError("Report commit check failed:\n- " + "\n- ".join(errors))


def check_append_only(
    root: Path, baseline_ref: str, current: list[tuple[Path, dict[str, object]]]
) -> None:
    verify = _git(root, ["rev-parse", "--verify", f"{baseline_ref}^{{commit}}"])
    if verify.returncode != 0:
        raise CompatibilityError(f"Unknown Git baseline {baseline_ref!r}")
    listing = _git(
        root,
        ["ls-tree", "-r", "--name-only", baseline_ref, "--", "compatibility/apps"],
    )
    if listing.returncode != 0:
        raise CompatibilityError(
            f"Could not list compatibility records at {baseline_ref}: {listing.stderr.strip()}"
        )
    current_by_name = {path.name: record for path, record in current}
    errors: list[str] = []
    for relative in listing.stdout.splitlines():
        if not relative.endswith(".json"):
            continue
        filename = Path(relative).name
        current_record = current_by_name.get(filename)
        if current_record is None:
            errors.append(f"{relative}: an existing app record may not be removed")
            continue
        shown = _git(root, ["show", f"{baseline_ref}:{relative}"])
        if shown.returncode != 0:
            errors.append(f"{relative}: could not read baseline record")
            continue
        try:
            baseline_record = json.loads(shown.stdout)
        except json.JSONDecodeError as error:
            errors.append(f"{relative}: baseline JSON is invalid: {error}")
            continue
        current_versions = {
            version_key(version): version
            for version in current_record.get("versions", [])
            if isinstance(version, dict) and isinstance(version.get("identity"), dict)
        }
        for baseline_version in baseline_record.get("versions", []):
            if not isinstance(baseline_version, dict) or not isinstance(
                baseline_version.get("identity"), dict
            ):
                continue
            key = version_key(baseline_version)
            current_version = current_versions.get(key)
            if current_version is None:
                errors.append(
                    f"{relative}: existing exact version {key!r} may not be removed or renamed"
                )
                continue
            old_reports = baseline_version.get("reports", [])
            new_reports = current_version.get("reports", [])
            if not isinstance(old_reports, list) or not isinstance(new_reports, list):
                errors.append(f"{relative}: reports must remain arrays")
            elif new_reports[: len(old_reports)] != old_reports:
                errors.append(
                    f"{relative}: reports for exact version {key!r} are immutable; append a new report"
                )
    if errors:
        raise CompatibilityError("Append-only check failed:\n- " + "\n- ".join(errors))


def markdown_for_records(records: list[tuple[Path, dict[str, object]]]) -> str:
    lines = [
        "# tapHLE compatibility",
        "",
        "<!-- Generated by dev-scripts/compatibility.py; edit compatibility/apps/*.json instead. -->",
        "",
        "**The current record is the live database at**",
        "<https://taphle.ephun.net/compatibility>. This file is an older snapshot,",
        "generated from the `compatibility/apps` records that predate it and kept",
        "only until they are migrated. Do not add records here.",
        "",
        "Compatibility is recorded per exact app build and exact tapHLE commit. A",
        "listed Archive.org source identifies what was tested; tapHLE does not ship",
        "the app. See [the database protocol](compatibility/README.md) for the rules",
        "that a result must meet.",
        "",
        "## Rating scale",
        "",
        f"- {rating_label('launch-blocked')} — The game does not reach usable content.",
        f"- {rating_label('boots')} — An intro or menu works, but gameplay does not.",
        f"- {rating_label('in-game')} — Some gameplay works, but major problems remain.",
        f"- {rating_label('playable-with-issues')} — The whole game can be played, with small problems.",
        f"- {rating_label('playable')} — Everything important works.",
        "- — Not tested — There is no verified tapHLE Windows result.",
        "",
        "Filled and empty stars plus the numeric score are a short summary. The",
        "exact milestone and feature states below show what was really tested.",
        "The scale is adapted from the",
        "[touchHLE app database](https://appdb.touchhle.org/) under CC BY 4.0.",
        "",
        "| Game | Exact build | Latest Windows result | tapHLE commit | Tested |",
        "| --- | --- | --- | --- | --- |",
    ]
    for _path, record in records:
        title = str(record["title"])
        slug = str(record["slug"])
        versions = record["versions"]
        assert isinstance(versions, list)
        for version in versions:
            assert isinstance(version, dict)
            identity = version["identity"]
            assert isinstance(identity, dict)
            reports = version["reports"]
            assert isinstance(reports, list)
            latest = reports[-1] if reports else None
            build = _identity_label(identity)
            if latest is None:
                status, commit, tested = "— Not tested", "—", "—"
            else:
                assert isinstance(latest, dict)
                status = rating_label(str(latest["status"]))
                if latest.get("booted") and latest.get("status") == "launch-blocked":
                    status += " (app booted)"
                commit = f"`{str(latest['taphle_commit'])[:8]}`"
                tested = str(latest["tested_at"])
            lines.append(f"| [{_escape_table(title)}](#{slug}) | {_escape_table(build)} | {status} | {commit} | {tested} |")

    for _path, record in records:
        title = str(record["title"])
        slug = str(record["slug"])
        lines.extend(["", f'<a id="{slug}"></a>', f"## {title}"])
        versions = record["versions"]
        assert isinstance(versions, list)
        for version in versions:
            assert isinstance(version, dict)
            identity = version["identity"]
            archive = version["archive_org"]
            reports = version["reports"]
            assert isinstance(identity, dict)
            assert isinstance(archive, dict)
            assert isinstance(reports, list)
            tested_file = next(
                file_record
                for file_record in archive["files"]
                if isinstance(file_record, dict) and file_record.get("tested") is True
            )
            filename_role = "tested" if reports else "target"
            lines.extend(
                [
                    "",
                    f"### {_identity_label(identity)}",
                    "",
                    f"- Bundle identifier: `{identity['bundle_identifier']}`",
                    f"- Minimum OS version: {identity['minimum_os_version']}",
                    f"- Archive source: [{archive['identifier']}]({archive['item_url']})",
                    f"- Exact {filename_role} IPA filename: `{tested_file['ipa_filename']}`",
                    f"- Source verification: {str(archive['verification']['state']).replace('-', ' ')}",
                    f"- Availability review: {archive['availability']['checked_at']} ({str(archive['availability']['status']).replace('-', ' ')})",
                ]
            )
            aliases = []
            other_files = []
            for file_record in archive["files"]:
                if not isinstance(file_record, dict) or file_record.get("tested") is not False:
                    continue
                destination = (
                    aliases
                    if all(file_record[algorithm] == tested_file[algorithm] for algorithm in HASH_RE)
                    else other_files
                )
                destination.append(str(file_record["ipa_filename"]))
            if aliases:
                lines.append("- Byte-identical Archive filename aliases: " + ", ".join(f"`{name}`" for name in aliases))
            if other_files:
                lines.append(
                    "- Other Archive filenames with different content hashes (not the tested artifact): "
                    + ", ".join(f"`{name}`" for name in other_files)
                )
            if not reports:
                lines.extend(["", "No verified Windows test report has been recorded yet."])
                continue
            latest = reports[-1]
            assert isinstance(latest, dict)
            status = rating_label(str(latest["status"]))
            if latest.get("booted") and latest.get("status") == "launch-blocked":
                status += " (the app lifecycle booted before the blocker)"
            lines.extend(
                [
                    "",
                    f"Latest verified report: **{status}** on {latest['tested_at']} with tapHLE `{latest['taphle_commit']}`.",
                    "",
                    str(latest["summary"]),
                    "",
                    f"Blocker: {latest['blocker']}",
                    "",
                    "Feature state: "
                    + ", ".join(
                        f"{feature}={latest['features'][feature]}" for feature in FEATURE_NAMES
                    )
                    + ".",
                ]
            )
    lines.extend(
        [
            "",
            "## Scope of these records",
            "",
            "A result applies only to the named app build, Archive file hash, tapHLE",
            "commit, and Windows host. It is not a claim that other versions work.",
            "Archive links are provenance references, not bundled downloads or blanket",
            "legal conclusions. See `compatibility/README.md` for the project policy.",
            "",
            "## Results from other emulators",
            "",
            "The [touchHLE database](https://appdb.touchhle.org/) and",
            "[HyperHLE AppDB](https://github.com/HyperHLE/HyperHLE/tree/trunk/appdb)",
            "are useful places to find games that may be worth testing. Their results are",
            "not tapHLE results. A game is listed above only after its exact file is",
            "hash-checked and tested with a committed tapHLE build on Windows.",
            "",
        ]
    )
    return "\n".join(lines)


def _identity_label(identity: dict[str, object]) -> str:
    label = f"{identity['bundle_version']} (`{identity['bundle_identifier']}`)"
    short = identity.get("short_version")
    if short is not None and short != identity["bundle_version"]:
        label = f"{short} / build {identity['bundle_version']} (`{identity['bundle_identifier']}`)"
    return label


def _escape_table(text: str) -> str:
    return text.replace("|", "\\|").replace("\n", " ")


def render(root: Path, check_only: bool) -> None:
    records = validate_database(root)
    rendered = markdown_for_records(records)
    output_path = root / "COMPATIBILITY.md"
    if check_only:
        try:
            actual = output_path.read_text(encoding="utf-8")
        except OSError as error:
            raise CompatibilityError(f"Could not read generated {output_path}: {error}") from error
        if actual != rendered:
            raise CompatibilityError(
                "COMPATIBILITY.md is stale; run: python dev-scripts/compatibility.py render"
            )
    else:
        output_path.write_text(rendered, encoding="utf-8", newline="\n")
        print(f"Rendered {output_path.relative_to(root)}")


def find_record(
    records: list[tuple[Path, dict[str, object]]], slug: str
) -> dict[str, object]:
    for _path, record in records:
        if record.get("slug") == slug:
            return record
    raise CompatibilityError(f"Unknown compatibility app slug {slug!r}")


def find_version(record: dict[str, object], bundle_version: str) -> dict[str, object]:
    matches = []
    versions = record.get("versions")
    assert isinstance(versions, list)
    for version in versions:
        assert isinstance(version, dict)
        identity = version.get("identity")
        assert isinstance(identity, dict)
        if identity.get("bundle_version") == bundle_version:
            matches.append(version)
    if len(matches) != 1:
        raise CompatibilityError(
            f"Expected one {record['slug']!r} version with CFBundleVersion {bundle_version!r}, found {len(matches)}"
        )
    return matches[0]


def compute_hashes(path: Path) -> dict[str, str]:
    digesters = {name: hashlib.new(name) for name in ("md5", "sha1", "sha256")}
    try:
        with path.open("rb") as stream:
            while True:
                chunk = stream.read(1024 * 1024)
                if not chunk:
                    break
                for digester in digesters.values():
                    digester.update(chunk)
    except OSError as error:
        raise CompatibilityError(f"Could not read local IPA {path}: {error}") from error
    return {name: digester.hexdigest() for name, digester in digesters.items()}


def inspect_ipa(path: Path, expected_identity: dict[str, object]) -> dict[str, str | None]:
    try:
        with zipfile.ZipFile(path, "r") as ipa:
            candidates = [
                name
                for name in ipa.namelist()
                if re.fullmatch(r"Payload/[^/]+\.app/Info\.plist", name) is not None
            ]
            if not candidates:
                raise CompatibilityError("IPA has no Payload/*.app/Info.plist")
            matching: list[dict[str, object]] = []
            parse_errors: list[str] = []
            for candidate in candidates:
                try:
                    plist = plistlib.loads(ipa.read(candidate))
                except (KeyError, plistlib.InvalidFileException, ValueError) as error:
                    parse_errors.append(f"{candidate}: {error}")
                    continue
                if isinstance(plist, dict) and str(plist.get("CFBundleIdentifier", "")) == expected_identity.get(
                    "bundle_identifier"
                ):
                    matching.append(plist)
            if len(matching) != 1:
                detail = "; ".join(parse_errors)
                raise CompatibilityError(
                    f"Expected exactly one Info.plist for bundle {expected_identity.get('bundle_identifier')!r}, found {len(matching)}"
                    + (f" ({detail})" if detail else "")
                )
    except zipfile.BadZipFile as error:
        raise CompatibilityError(f"Local file is not a valid IPA ZIP: {error}") from error
    except OSError as error:
        raise CompatibilityError(f"Could not inspect local IPA: {error}") from error

    plist = matching[0]
    observed: dict[str, str | None] = {
        "bundle_identifier": _plist_string(plist.get("CFBundleIdentifier")),
        "bundle_version": _plist_string(plist.get("CFBundleVersion")),
        "short_version": _plist_string(plist.get("CFBundleShortVersionString")),
        "minimum_os_version": _plist_string(plist.get("MinimumOSVersion")),
    }
    for field in ("bundle_identifier", "bundle_version", "minimum_os_version"):
        if observed[field] != expected_identity.get(field):
            raise CompatibilityError(
                f"IPA Info.plist {field} is {observed[field]!r}, expected {expected_identity.get(field)!r}"
            )
    expected_short = expected_identity.get("short_version")
    if observed["short_version"] != expected_short:
        raise CompatibilityError(
            f"IPA Info.plist short_version is {observed['short_version']!r}, expected {expected_short!r}"
        )
    return observed


def _plist_string(value: object) -> str | None:
    if value is None:
        return None
    return str(value)


def fetch_archive_metadata(identifier: str, timeout: float) -> dict[str, object]:
    if IDENTIFIER_RE.fullmatch(identifier) is None:
        raise CompatibilityError(f"Invalid Archive.org identifier {identifier!r}")
    url = ARCHIVE_METADATA_PREFIX + urllib.parse.quote(identifier, safe="")
    request = urllib.request.Request(
        url,
        headers={"User-Agent": "tapHLE-compatibility-check/1 (+https://github.com/ephun/tapHLE)"},
    )
    try:
        with urllib.request.urlopen(request, timeout=timeout) as response:
            body = response.read(20 * 1024 * 1024 + 1)
    except (OSError, urllib.error.URLError) as error:
        raise CompatibilityError(f"Could not fetch {url}: {error}") from error
    if len(body) > 20 * 1024 * 1024:
        raise CompatibilityError("Archive.org metadata response exceeded 20 MiB")
    try:
        metadata = json.loads(body.decode("utf-8"))
    except (UnicodeError, json.JSONDecodeError) as error:
        raise CompatibilityError(f"Archive.org returned invalid metadata JSON: {error}") from error
    if not isinstance(metadata, dict):
        raise CompatibilityError("Archive.org metadata response was not an object")
    return metadata


def verify_remote_record(archive: dict[str, object], metadata: dict[str, object]) -> dict[str, dict[str, object]]:
    identifier = archive["identifier"]
    item_metadata = metadata.get("metadata")
    if not isinstance(item_metadata, dict) or item_metadata.get("identifier") != identifier:
        raise CompatibilityError("Archive.org metadata identifier does not exactly match the record")
    files = metadata.get("files")
    if not isinstance(files, list):
        raise CompatibilityError("Archive.org metadata has no files array")
    remote_by_name = {
        item.get("name"): item
        for item in files
        if isinstance(item, dict) and isinstance(item.get("name"), str)
    }
    verified: dict[str, dict[str, object]] = {}
    archive_files = archive["files"]
    assert isinstance(archive_files, list)
    for file_record in archive_files:
        assert isinstance(file_record, dict)
        filename = str(file_record["ipa_filename"])
        remote = remote_by_name.get(filename)
        if remote is None:
            raise CompatibilityError(
                f"Exact IPA filename {filename!r} is not listed by the supplied Archive.org item"
            )
        if remote.get("source") != "original":
            raise CompatibilityError(f"Archive.org file {filename!r} is not marked as an original")
        for algorithm in ("md5", "sha1"):
            remote_hash = str(remote.get(algorithm, "")).lower()
            if remote_hash != file_record[algorithm]:
                raise CompatibilityError(
                    f"Archive.org {algorithm} for {filename!r} is {remote_hash!r}, record says {file_record[algorithm]!r}"
                )
        verified[filename] = remote
    return verified


def verify_archive(args: argparse.Namespace, root: Path) -> None:
    records = validate_database(root)
    record = find_record(records, args.slug)
    version = find_version(record, args.bundle_version)
    identity = version["identity"]
    archive = version["archive_org"]
    assert isinstance(identity, dict)
    assert isinstance(archive, dict)
    metadata = fetch_archive_metadata(str(archive["identifier"]), args.timeout)
    verify_remote_record(archive, metadata)

    ipa_path = Path(args.ipa).resolve()
    selected_filename = args.archive_filename or ipa_path.name
    archive_files = archive["files"]
    assert isinstance(archive_files, list)
    selected = next(
        (
            file_record
            for file_record in archive_files
            if isinstance(file_record, dict) and file_record.get("ipa_filename") == selected_filename
        ),
        None,
    )
    if selected is None:
        raise CompatibilityError(
            f"Local IPA must correspond to an exact archive_org.files filename; {selected_filename!r} is not recorded"
        )
    hashes = compute_hashes(ipa_path)
    for algorithm in ("md5", "sha1", "sha256"):
        if hashes[algorithm] != selected[algorithm]:
            raise CompatibilityError(
                f"Local IPA does not match Archive.org file {selected_filename!r}: {algorithm} mismatch"
            )
    observed = inspect_ipa(ipa_path, identity)
    taphle_checked = False
    if args.taphle_exe:
        cross_check_taphle(Path(args.taphle_exe), ipa_path, identity)
        taphle_checked = True
    result = {
        "archive_identifier": archive["identifier"],
        "archive_item_url": archive["item_url"],
        "archive_ipa_filename": selected_filename,
        "hashes": hashes,
        "info_plist": observed,
        "taphle_info_cross_check": taphle_checked,
    }
    print(json.dumps(result, indent=2, ensure_ascii=False))


def cross_check_taphle(executable: Path, ipa_path: Path, identity: dict[str, object]) -> None:
    try:
        result = subprocess.run(
            [str(executable), str(ipa_path), "--info"],
            check=False,
            capture_output=True,
            text=True,
            encoding="utf-8",
            errors="replace",
            timeout=60,
        )
    except (OSError, subprocess.TimeoutExpired) as error:
        raise CompatibilityError(f"Could not run tapHLE --info: {error}") from error
    output = result.stdout + "\n" + result.stderr
    expected_lines = [
        f"- Version: {identity['bundle_version']}",
        f"- Identifier: {identity['bundle_identifier']}",
        f"- Minimum OS version: {identity['minimum_os_version']}",
    ]
    if result.returncode != 0 or any(line not in output for line in expected_lines):
        raise CompatibilityError(
            "tapHLE --info did not confirm the exact bundle identifier, version, and minimum OS"
        )


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--root",
        type=Path,
        default=DEFAULT_ROOT,
        help=argparse.SUPPRESS,
    )
    subparsers = parser.add_subparsers(dest="command", required=True)

    check_parser = subparsers.add_parser("check", help="validate records without network access")
    check_parser.add_argument(
        "--baseline-ref",
        help="also enforce append-only reports against this Git commit/ref",
    )
    subparsers.add_parser("list", help="list app slugs, exact versions, and latest status")
    show_parser = subparsers.add_parser("show", help="print one app record as JSON")
    show_parser.add_argument("slug")

    verify_parser = subparsers.add_parser(
        "verify-archive",
        help="explicitly verify Archive.org metadata, local IPA hashes, and embedded Info.plist",
    )
    verify_parser.add_argument("slug")
    verify_parser.add_argument("--bundle-version", required=True)
    verify_parser.add_argument("--ipa", required=True, help="path to a local, uncommitted IPA")
    verify_parser.add_argument(
        "--archive-filename",
        help="exact Archive.org filename when the local copy was renamed",
    )
    verify_parser.add_argument(
        "--taphle-exe",
        help="optionally cross-check the IPA with this tapHLE executable's --info output",
    )
    verify_parser.add_argument("--timeout", type=float, default=30.0)
    return parser


def main(argv: list[str] | None = None) -> int:
    parser = build_parser()
    args = parser.parse_args(argv)
    root = args.root.resolve()
    try:
        if args.command == "check":
            records = validate_database(root)
            check_report_commits(root, records)
            if args.baseline_ref:
                check_append_only(root, args.baseline_ref, records)
            print(f"Compatibility database is valid ({len(records)} app record(s)); no network used.")
        elif args.command == "list":
            records = validate_database(root)
            for _path, record in records:
                versions = record["versions"]
                assert isinstance(versions, list)
                for version in versions:
                    assert isinstance(version, dict)
                    identity = version["identity"]
                    reports = version["reports"]
                    assert isinstance(identity, dict)
                    assert isinstance(reports, list)
                    status = plain_rating_label(str(reports[-1]["status"])) if reports else "Not tested"
                    print(f"{record['slug']}\t{record['title']}\t{_identity_label(identity)}\t{status}")
        elif args.command == "show":
            record = find_record(validate_database(root), args.slug)
            print(json.dumps(record, indent=2, ensure_ascii=False))
        elif args.command == "verify-archive":
            verify_archive(args, root)
        else:
            parser.error(f"unknown command {args.command!r}")
    except CompatibilityError as error:
        print(f"error: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
