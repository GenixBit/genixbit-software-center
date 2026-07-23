from __future__ import annotations

import importlib.util
import json
from pathlib import Path
import tempfile
import unittest

SCRIPT = Path(__file__).resolve().parents[1] / "scripts" / "validate-release-rollback.py"
SPEC = importlib.util.spec_from_file_location("release_rollback", SCRIPT)
assert SPEC is not None and SPEC.loader is not None
release_rollback = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(release_rollback)


class ReleaseRollbackTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        (self.root / "data").mkdir()
        (self.root / "release").mkdir()
        (self.root / "Cargo.toml").write_text(
            '[workspace]\n[workspace.package]\nversion = "0.2.0"\n',
            encoding="utf-8",
        )
        (self.root / "data" / "com.genixbit.SoftwareCenter.metainfo.xml").write_text(
            '<component><releases><release version="0.2.0"/></releases></component>\n',
            encoding="utf-8",
        )
        self.asset = "data/runtime.asset"
        (self.root / self.asset).write_text("asset\n", encoding="utf-8")
        self.policy = self.root / "release" / "release-policy.json"
        self.payload = {
            "schema_version": 1,
            "application_id": "com.genixbit.SoftwareCenter",
            "current": self.record("0.2.0"),
            "rollback_targets": [self.record("0.1.0")],
            "required_assets": [self.asset],
        }
        self.write_policy()

    @staticmethod
    def record(version: str) -> dict[str, int | str]:
        return {
            "version": version,
            "dbus_api": 1,
            "transaction_journal": 1,
            "event_journal": 1,
            "settings_format": 1,
            "system_profile_format": 1,
        }

    def write_policy(self) -> None:
        self.policy.write_text(json.dumps(self.payload), encoding="utf-8")

    def validate(self) -> list[str]:
        return release_rollback.validate(self.root, self.policy)

    def test_accepts_compatible_rollback_contract(self) -> None:
        self.assertEqual(self.validate(), [])

    def test_rejects_workspace_and_appstream_version_drift(self) -> None:
        self.payload["current"]["version"] = "0.3.0"
        self.write_policy()
        errors = self.validate()
        self.assertTrue(any("does not match workspace version" in error for error in errors))
        self.assertTrue(any("AppStream metadata has no release entry" in error for error in errors))

    def test_rejects_newer_or_duplicate_rollback_targets(self) -> None:
        self.payload["rollback_targets"] = [self.record("0.2.0"), self.record("0.2.0")]
        self.write_policy()
        errors = self.validate()
        self.assertTrue(any("must be older" in error for error in errors))
        self.assertTrue(any("duplicate rollback target" in error for error in errors))

    def test_rejects_incompatible_persistent_formats(self) -> None:
        target = self.record("0.1.0")
        target["transaction_journal"] = 0
        target["dbus_api"] = 2
        self.payload["rollback_targets"] = [target]
        self.write_policy()
        errors = self.validate()
        self.assertTrue(any("dbus_api must equal" in error for error in errors))
        self.assertTrue(any("transaction_journal must be a positive integer" in error for error in errors))

    def test_rejects_target_that_cannot_read_current_format(self) -> None:
        self.payload["current"]["settings_format"] = 2
        self.write_policy()
        errors = self.validate()
        self.assertTrue(any("cannot read current format 2" in error for error in errors))

    def test_rejects_missing_or_duplicate_assets(self) -> None:
        self.payload["required_assets"] = [self.asset, self.asset, "data/missing.asset"]
        self.write_policy()
        errors = self.validate()
        self.assertTrue(any("duplicate required asset" in error for error in errors))
        self.assertTrue(any("missing required release asset" in error for error in errors))


if __name__ == "__main__":
    unittest.main()
