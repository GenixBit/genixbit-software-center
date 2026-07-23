from __future__ import annotations

import copy
import importlib.util
import json
from pathlib import Path
import sys
import tempfile
import unittest

SCRIPT = Path(__file__).resolve().parents[1] / "scripts" / "validate-release-rollback.py"
SPEC = importlib.util.spec_from_file_location("release_rollback", SCRIPT)
assert SPEC is not None and SPEC.loader is not None
release_rollback = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = release_rollback
SPEC.loader.exec_module(release_rollback)

BASE_CONTRACT = {
    "schema": 1,
    "description": "Deterministic release and rollback test contract.",
    "package": "genixbit-software-center",
    "version": "0.2.0",
    "application_id": "com.genixbit.SoftwareCenter",
    "service": "genixpkgd.service",
    "architectures": ["amd64", "arm64"],
    "crate_manifests": [
        "crates/package-model/Cargo.toml",
        "crates/software-center/Cargo.toml",
        "crates/genixpkgd/Cargo.toml",
    ],
    "required_source_assets": [
        "Cargo.lock",
        "data/com.genixbit.SoftwareCenter.desktop",
        "data/com.genixbit.SoftwareCenter.metainfo.xml",
        "data/com.genixbit.SoftwareCenter.css",
        "data/icons/hicolor/scalable/apps/com.genixbit.SoftwareCenter.svg",
        "data/icons/hicolor/symbolic/apps/com.genixbit.SoftwareCenter-symbolic.svg",
        "dbus/com.genixbit.PackageManager1.xml",
        "systemd/genixpkgd.service",
        "os-image/default-packages.list",
    ],
    "required_bundle_paths": [
        "usr/bin/genixbit-software-center",
        "usr/libexec/genixpkgd",
        "usr/share/applications/com.genixbit.SoftwareCenter.desktop",
        "usr/share/dbus-1/interfaces/com.genixbit.PackageManager1.xml",
        "usr/share/genixbit-software-center/com.genixbit.SoftwareCenter.css",
        "usr/share/icons/hicolor/scalable/apps/com.genixbit.SoftwareCenter.svg",
        "usr/share/icons/hicolor/symbolic/apps/com.genixbit.SoftwareCenter-symbolic.svg",
        "usr/share/metainfo/com.genixbit.SoftwareCenter.metainfo.xml",
        "usr/lib/systemd/system/genixpkgd.service",
    ],
    "rollback": {
        "minimum_retained_releases": 1,
        "same_major_only": True,
        "automatic": False,
    },
}


class ReleaseRollbackTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        self.contract = copy.deepcopy(BASE_CONTRACT)
        self.contract_path = self.root / "release-contract.json"
        self.write_contract()

    def write_contract(self) -> None:
        self.contract_path.write_text(
            json.dumps(self.contract, indent=2, sort_keys=True) + "\n",
            encoding="utf-8",
        )

    def create_bundle(
        self,
        name: str,
        version: str,
        architecture: str = "amd64",
    ) -> tuple[Path, dict[str, object]]:
        root = self.root / name
        for relative in self.contract["required_bundle_paths"]:
            path = root / relative
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(f"{version}:{relative}\n".encode())
            path.chmod(0o644)
        manifest, errors = release_rollback.build_manifest(
            root, self.contract, version, architecture
        )
        self.assertEqual(errors, [])
        return root, manifest

    def test_accepts_valid_release_and_rollback_pair(self) -> None:
        current_root, current = self.create_bundle("current", "0.2.0")
        rollback_root, rollback = self.create_bundle("rollback", "0.1.0")
        self.assertEqual(
            release_rollback.validate_release_pair(
                self.contract, current_root, current, rollback_root, rollback
            ),
            [],
        )

    def test_rejects_tampered_and_unmanifested_files(self) -> None:
        current_root, current = self.create_bundle("current", "0.2.0")
        rollback_root, rollback = self.create_bundle("rollback", "0.1.0")
        (rollback_root / "usr/libexec/genixpkgd").write_text(
            "tampered\n", encoding="utf-8"
        )
        extra = rollback_root / "usr/share/untracked"
        extra.parent.mkdir(parents=True, exist_ok=True)
        extra.write_text("unexpected\n", encoding="utf-8")
        errors = release_rollback.validate_release_pair(
            self.contract, current_root, current, rollback_root, rollback
        )
        self.assertTrue(any("SHA-256 mismatch" in error for error in errors))
        self.assertTrue(any("unmanifested files" in error for error in errors))

    def test_rejects_non_older_or_cross_major_rollback(self) -> None:
        current_root, current = self.create_bundle("current", "0.2.0")
        same_root, same = self.create_bundle("same", "0.2.0")
        errors = release_rollback.validate_release_pair(
            self.contract, current_root, current, same_root, same
        )
        self.assertTrue(any("must be older" in error for error in errors))

        old_major_root, old_major = self.create_bundle("old-major", "1.9.0")
        future_root, future = self.create_bundle("future", "2.0.0")
        future_contract = copy.deepcopy(self.contract)
        future_contract["version"] = "2.0.0"
        errors = release_rollback.validate_release_pair(
            future_contract, future_root, future, old_major_root, old_major
        )
        self.assertTrue(any("current major" in error for error in errors))

    def test_rejects_architecture_or_identity_mismatch(self) -> None:
        current_root, current = self.create_bundle("current", "0.2.0", "amd64")
        rollback_root, rollback = self.create_bundle("rollback", "0.1.0", "arm64")
        rollback["package"] = "other-package"
        errors = release_rollback.validate_release_pair(
            self.contract, current_root, current, rollback_root, rollback
        )
        self.assertTrue(any("architectures must match" in error for error in errors))
        self.assertTrue(any("package does not match" in error for error in errors))

    def test_manifest_creation_requires_complete_bundle(self) -> None:
        root, _ = self.create_bundle("bundle", "0.2.0")
        missing = root / self.contract["required_bundle_paths"][0]
        missing.unlink()
        _, errors = release_rollback.build_manifest(
            root, self.contract, "0.2.0", "amd64"
        )
        self.assertTrue(any("missing required paths" in error for error in errors))

    def test_validates_source_version_and_metadata_contract(self) -> None:
        source = self.root / "source"
        (source / "crates/package-model").mkdir(parents=True)
        (source / "crates/software-center").mkdir(parents=True)
        (source / "crates/genixpkgd").mkdir(parents=True)
        (source / "data/icons/hicolor/scalable/apps").mkdir(parents=True)
        (source / "data/icons/hicolor/symbolic/apps").mkdir(parents=True)
        (source / "dbus").mkdir()
        (source / "systemd").mkdir()
        (source / "os-image").mkdir()

        (source / "Cargo.toml").write_text(
            "[workspace]\nmembers=[]\n[workspace.package]\nversion='0.2.0'\n",
            encoding="utf-8",
        )
        for relative in self.contract["crate_manifests"]:
            (source / relative).write_text(
                "[package]\nname='fixture'\nversion.workspace=true\n",
                encoding="utf-8",
            )
        (source / "Cargo.lock").write_text("version = 4\n", encoding="utf-8")
        (source / "data/com.genixbit.SoftwareCenter.desktop").write_text(
            "[Desktop Entry]\n"
            "Exec=genixbit-software-center\n"
            "Icon=com.genixbit.SoftwareCenter\n",
            encoding="utf-8",
        )
        (source / "data/com.genixbit.SoftwareCenter.metainfo.xml").write_text(
            "<component>"
            "<id>com.genixbit.SoftwareCenter</id>"
            "<icon type='stock'>com.genixbit.SoftwareCenter</icon>"
            "<releases><release version='0.2.0'/><release version='0.1.0'/></releases>"
            "</component>",
            encoding="utf-8",
        )
        for relative in self.contract["required_source_assets"]:
            path = source / relative
            if not path.exists():
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_text("fixture\n", encoding="utf-8")
        (source / "os-image/default-packages.list").write_text(
            "genixbit-software-center\n", encoding="utf-8"
        )

        self.assertEqual(
            release_rollback.validate_source_contract(source, self.contract_path),
            [],
        )
        self.contract["version"] = "0.3.0"
        self.write_contract()
        errors = release_rollback.validate_source_contract(source, self.contract_path)
        self.assertTrue(any("workspace.package.version" in error for error in errors))
        self.assertTrue(any("AppStream releases" in error for error in errors))


if __name__ == "__main__":
    unittest.main()
