import importlib.util
from pathlib import Path
import tempfile
import unittest


REPOSITORY_ROOT = Path(__file__).resolve().parents[2]
MODULE_PATH = REPOSITORY_ROOT / "dev-scripts" / "release_version.py"
SPEC = importlib.util.spec_from_file_location("taphle_release_version", MODULE_PATH)
assert SPEC is not None and SPEC.loader is not None
release_version = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(release_version)


class ReleaseVersionTests(unittest.TestCase):
    def test_repository_version_and_names(self):
        version = release_version.workspace_version()
        self.assertRegex(version, r"^\d+\.\d+\.\d+(?:[-+][0-9A-Za-z.-]+)?$")
        self.assertEqual(
            release_version.release_tag(version),
            f"taphle-v{version}",
        )
        self.assertEqual(
            release_version.windows_archive_name(version),
            f"tapHLE-v{version}-Windows-x86_64.zip",
        )

    def test_names_for_prerelease(self):
        version = "1.2.3-rc.4"
        self.assertEqual(release_version.release_tag(version), "taphle-v1.2.3-rc.4")
        self.assertEqual(
            release_version.windows_archive_name(version),
            "tapHLE-v1.2.3-rc.4-Windows-x86_64.zip",
        )

    def test_workspace_version_reads_requested_manifest(self):
        with tempfile.TemporaryDirectory() as directory:
            manifest = Path(directory) / "Cargo.toml"
            manifest.write_text(
                '[workspace.package]\nversion = "1.2.3-rc.4"\n',
                encoding="utf-8",
            )
            self.assertEqual(release_version.workspace_version(manifest), "1.2.3-rc.4")

    def test_tag_must_match_exactly(self):
        release_version.validate_tag("taphle-v0.3.0-alpha.1", "0.3.0-alpha.1")
        for invalid in [
            "v0.3.0-alpha.1",
            "taphle-v0.3.0-alpha.2",
            "taphle-v0.3.0-alpha.1-dirty",
            "taphle-v0.3.0-alpha.1-1-gabc1234",
        ]:
            with self.subTest(invalid=invalid):
                with self.assertRaises(ValueError):
                    release_version.validate_tag(invalid, "0.3.0-alpha.1")

    def test_changelog_requires_exact_version_and_valid_date(self):
        with tempfile.TemporaryDirectory() as directory:
            changelog = Path(directory) / "CHANGELOG.md"
            changelog.write_text(
                "# Changelog\n\n## 1.2.3-rc.4 - 2026-07-18\n",
                encoding="utf-8",
            )
            release_version.validate_changelog("1.2.3-rc.4", changelog)

            for invalid in [
                "## Unreleased for 1.2.3-rc.4",
                "## 1.2.3-rc.3 - 2026-07-18",
                "## 1.2.3-rc.4 - 2026-02-30",
            ]:
                with self.subTest(invalid=invalid):
                    changelog.write_text(f"# Changelog\n\n{invalid}\n", encoding="utf-8")
                    with self.assertRaises(ValueError):
                        release_version.validate_changelog("1.2.3-rc.4", changelog)


if __name__ == "__main__":
    unittest.main()
