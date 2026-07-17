import copy
import importlib.util
import json
from pathlib import Path
import plistlib
import subprocess
import tempfile
import unittest
import zipfile


REPOSITORY_ROOT = Path(__file__).resolve().parents[2]
MODULE_PATH = REPOSITORY_ROOT / "dev-scripts" / "compatibility.py"
SPEC = importlib.util.spec_from_file_location("taphle_compatibility", MODULE_PATH)
assert SPEC is not None and SPEC.loader is not None
compatibility = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(compatibility)


class CompatibilityTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        record_path = REPOSITORY_ROOT / "compatibility" / "apps" / "ricky.json"
        cls.record_path = record_path
        cls.record = json.loads(record_path.read_text(encoding="utf-8"))

    def test_repository_record_is_valid(self):
        self.assertEqual(
            compatibility.validate_record(self.record, self.record_path),
            [],
        )

    def test_noncanonical_archive_url_is_rejected(self):
        record = copy.deepcopy(self.record)
        record["versions"][0]["archive_org"]["item_url"] += "?download=1"
        errors = compatibility.validate_record(record, self.record_path)
        self.assertTrue(any("canonical URL" in error for error in errors), errors)

    def test_archive_version_identity_must_match(self):
        record = copy.deepcopy(self.record)
        record["versions"][0]["archive_org"]["bundle_version"] = "2.1-ish"
        errors = compatibility.validate_record(record, self.record_path)
        self.assertTrue(any("must exactly match version identity" in error for error in errors), errors)

    def test_report_requires_current_availability_review(self):
        record = copy.deepcopy(self.record)
        archive_file = record["versions"][0]["archive_org"]["files"][0]
        record["versions"][0]["reports"] = [
            {
                "id": "2026-07-18-example-report",
                "tested_at": "2026-07-18",
                "taphle_commit": "1" * 40,
                "host": {
                    "os": "Windows",
                    "os_version": "test",
                    "architecture": "x86_64",
                    "cpu": "test",
                    "gpu": "test",
                },
                "artifact": {
                    "archive_ipa_filename": archive_file["ipa_filename"],
                    "sha1": archive_file["sha1"],
                    "verification": "archive-content-hash",
                },
                "status": "launch-blocked",
                "booted": False,
                "summary": "Synthetic test report.",
                "milestones": [],
                "blocker": "Synthetic blocker.",
                "features": {name: "unknown" for name in compatibility.FEATURE_NAMES},
            }
        ]
        errors = compatibility.validate_record(record, self.record_path)
        self.assertTrue(any("availability must be re-checked" in error for error in errors), errors)

    def test_remote_metadata_requires_exact_original_filename_and_hash(self):
        archive = self.record["versions"][0]["archive_org"]
        metadata = {
            "metadata": {"identifier": archive["identifier"]},
            "files": [
                {
                    "name": file_record["ipa_filename"],
                    "source": "original",
                    "md5": file_record["md5"],
                    "sha1": file_record["sha1"],
                }
                for file_record in archive["files"]
            ],
        }
        verified = compatibility.verify_remote_record(archive, metadata)
        self.assertEqual(set(verified), {item["ipa_filename"] for item in archive["files"]})

        metadata["files"][0]["name"] = "similar but not exact.ipa"
        with self.assertRaises(compatibility.CompatibilityError):
            compatibility.verify_remote_record(archive, metadata)

    def test_inspect_ipa_reads_embedded_identity_without_running_it(self):
        identity = self.record["versions"][0]["identity"]
        info = {
            "CFBundleIdentifier": identity["bundle_identifier"],
            "CFBundleVersion": identity["bundle_version"],
            "MinimumOSVersion": identity["minimum_os_version"],
        }
        with tempfile.TemporaryDirectory() as temporary_directory:
            ipa_path = Path(temporary_directory) / "synthetic.ipa"
            with zipfile.ZipFile(ipa_path, "w") as ipa:
                ipa.writestr("Payload/Synthetic.app/Info.plist", plistlib.dumps(info))
            observed = compatibility.inspect_ipa(ipa_path, identity)
        self.assertEqual(observed["bundle_identifier"], "com.nabilchatbi.Ricky")
        self.assertEqual(observed["bundle_version"], "2.1")
        self.assertEqual(observed["minimum_os_version"], "3.0")

    def test_git_baseline_makes_reports_append_only(self):
        with tempfile.TemporaryDirectory() as temporary_directory:
            root = Path(temporary_directory)
            app_path = root / "compatibility" / "apps" / "ricky.json"
            app_path.parent.mkdir(parents=True)
            baseline = copy.deepcopy(self.record)
            baseline["versions"][0]["reports"] = [{"id": "original-observation"}]
            app_path.write_text(json.dumps(baseline), encoding="utf-8")
            for command in (
                ["git", "init", "-q"],
                ["git", "config", "user.name", "Compatibility Test"],
                ["git", "config", "user.email", "test@example.invalid"],
                ["git", "add", "compatibility/apps/ricky.json"],
                ["git", "commit", "-q", "-m", "baseline"],
            ):
                subprocess.run(command, cwd=root, check=True, capture_output=True)

            appended = copy.deepcopy(baseline)
            appended["versions"][0]["reports"].append({"id": "new-observation"})
            compatibility.check_append_only(root, "HEAD", [(app_path, appended)])

            mutated = copy.deepcopy(appended)
            mutated["versions"][0]["reports"][0]["id"] = "rewritten-observation"
            with self.assertRaises(compatibility.CompatibilityError):
                compatibility.check_append_only(root, "HEAD", [(app_path, mutated)])


if __name__ == "__main__":
    unittest.main()
