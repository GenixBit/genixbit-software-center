from __future__ import annotations
import hashlib, importlib.util, json, tempfile, unittest
from pathlib import Path

SCRIPT = Path(__file__).resolve().parents[1] / "scripts" / "validate-release-rollback.py"
SPEC = importlib.util.spec_from_file_location("release_rollback", SCRIPT)
assert SPEC and SPEC.loader
release_rollback = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(release_rollback)


class ReleaseRollbackTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        (self.root / "current.deb").write_bytes(b"current")
        (self.root / "previous.deb").write_bytes(b"previous")
        self.manifest = self.root / "release.json"
        self.write_manifest()

    def release(self, role: str, version: str, artifact: str) -> dict[str, str]:
        content = (self.root / artifact).read_bytes()
        return {"role": role, "package": "genixbit-software-center", "version": version,
                "architecture": "amd64", "artifact": artifact,
                "sha256": hashlib.sha256(content).hexdigest()}

    def write_manifest(self, *, current="0.2.0", rollback="0.1.0") -> None:
        self.manifest.write_text(json.dumps({"version": 1, "releases": [
            self.release("current", current, "current.deb"),
            self.release("rollback", rollback, "previous.deb")],
            "rollback_policy": {"retained_releases": 1, "automatic": False}}), encoding="utf-8")

    def validate(self) -> list[str]:
        return release_rollback.validate(self.root, self.manifest)

    def test_accepts_valid_release_and_rollback_pair(self) -> None:
        self.assertEqual(self.validate(), [])

    def test_rejects_tampered_artifact(self) -> None:
        (self.root / "previous.deb").write_bytes(b"tampered")
        self.assertTrue(any("SHA-256 mismatch" in error for error in self.validate()))

    def test_rejects_non_older_rollback(self) -> None:
        self.write_manifest(current="0.2.0", rollback="0.2.0")
        self.assertTrue(any("older" in error for error in self.validate()))

    def test_rejects_identity_architecture_and_automatic_rollback(self) -> None:
        data = json.loads(self.manifest.read_text())
        data["releases"][1]["package"] = "other"
        data["releases"][1]["architecture"] = "arm64"
        data["rollback_policy"]["automatic"] = True
        self.manifest.write_text(json.dumps(data))
        errors = self.validate()
        self.assertTrue(any("identity" in error for error in errors))
        self.assertTrue(any("architecture" in error for error in errors))
        self.assertTrue(any("automatic" in error for error in errors))


if __name__ == "__main__":
    unittest.main()
