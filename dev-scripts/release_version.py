#!/usr/bin/env python3
"""Validate tapHLE release tags against the workspace Cargo version."""

from __future__ import annotations

import argparse
from datetime import date
from pathlib import Path
import re
import sys


REPOSITORY_ROOT = Path(__file__).resolve().parents[1]
TAG_PREFIX = "taphle-v"
CHANGELOG_PATH = REPOSITORY_ROOT / "CHANGELOG.md"


def workspace_version(cargo_toml: Path = REPOSITORY_ROOT / "Cargo.toml") -> str:
    in_workspace_package = False
    for line in cargo_toml.read_text(encoding="utf-8").splitlines():
        stripped = line.strip()
        if stripped.startswith("["):
            in_workspace_package = stripped == "[workspace.package]"
            continue
        if not in_workspace_package:
            continue
        match = re.fullmatch(r'version\s*=\s*"([^"]+)"\s*', stripped)
        if match is not None:
            return match.group(1)
    raise ValueError(f"could not find [workspace.package] version in {cargo_toml}")


def release_tag(version: str) -> str:
    return f"{TAG_PREFIX}{version}"


def windows_archive_name(version: str) -> str:
    return f"tapHLE-v{version}-Windows-x86_64.zip"


def validate_tag(tag: str, version: str) -> None:
    expected = release_tag(version)
    if tag != expected:
        raise ValueError(
            f"release tag {tag!r} does not exactly match Cargo version; "
            f"expected {expected!r}"
        )


def validate_changelog(
    version: str, changelog: Path = CHANGELOG_PATH
) -> None:
    heading = re.compile(
        rf"^## {re.escape(version)} - (\d{{4}}-\d{{2}}-\d{{2}})$"
    )
    for line in changelog.read_text(encoding="utf-8").splitlines():
        match = heading.fullmatch(line)
        if match is None:
            continue
        try:
            date.fromisoformat(match.group(1))
        except ValueError as error:
            raise ValueError(f"invalid release date in changelog heading: {line!r}") from error
        return
    raise ValueError(
        f"{changelog} needs an exact release heading: "
        f"'## {version} - YYYY-MM-DD'"
    )


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    subparsers = parser.add_subparsers(dest="command", required=True)

    check_parser = subparsers.add_parser("check-tag")
    check_parser.add_argument("tag")

    subparsers.add_parser("version")
    subparsers.add_parser("archive-name")

    args = parser.parse_args(argv)
    version = workspace_version()
    if args.command == "check-tag":
        try:
            validate_tag(args.tag, version)
            validate_changelog(version)
        except ValueError as error:
            print(f"Error: {error}", file=sys.stderr)
            return 1
        print(f"Validated tapHLE release tag {args.tag} and changelog heading")
    elif args.command == "version":
        print(version)
    elif args.command == "archive-name":
        print(windows_archive_name(version))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
