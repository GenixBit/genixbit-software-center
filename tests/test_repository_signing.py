from __future__ import annotations

import hashlib
import importlib.util
import json
from pathlib import Path
import tempfile
import unittest

SCRIPT = Path(__file__).resolve().parents[1] / "scripts" / "validate-repository-signing.py"
SPEC = importlib.util.spec_from_file_location("repository_signing", SCRIPT)
assert SPEC is not None and SPEC.loader is not None
repository_signing = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(repository_signing)


class RepositorySigningTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name) / "root"
        self.sources = self.root / "etc" / "apt" / "sources.list.d"
        self.sources.mkdir(parents=True)
        self.keyring_path = "/usr/share/keyrings/genixbit-test-archive-keyring.gpg"
        self.keyring = self.root / self.keyring_path.lstrip("/")
        self.keyring.parent.mkdir(parents=True)
        self.keyring.write_bytes(b"test keyring fixture\n")
        self.keyring.chmod(0o644)
        self.manifest = Path(self.temporary.name) / "keyrings.json"
        self.write_manifest()

    def write_manifest(self, digest: str | None = None) -> None:
        digest = digest or hashlib.sha256(self.keyring.read_bytes()).hexdigest()
        self.manifest.write_text(
            json.dumps(
                {
                    "version": 1,
                    "keyrings": {self.keyring_path: digest},
                }
            ),
            encoding="utf-8",
        )

    def validate(self) -> tuple[int, list[str]]:
        return repository_signing.validate(self.root, self.manifest)

    def test_accepts_https_deb822_source_with_pinned_keyring(self) -> None:
        (self.sources / "genixbit.sources").write_text(
            "Types: deb deb-src\n"
            "URIs: https://packages.example.invalid/os\n"
            "Suites: stable\n"
            "Components: main\n"
            f"Signed-By: {self.keyring_path}\n",
            encoding="utf-8",
        )
        count, errors = self.validate()
        self.assertEqual(count, 1)
        self.assertEqual(errors, [])

    def test_accepts_legacy_source_with_explicit_signed_by(self) -> None:
        (self.sources / "genixbit.list").write_text(
            f"deb [arch=amd64 signed-by={self.keyring_path}] "
            "https://packages.example.invalid/os stable main\n",
            encoding="utf-8",
        )
        count, errors = self.validate()
        self.assertEqual(count, 1)
        self.assertEqual(errors, [])

    def test_rejects_source_without_signed_by(self) -> None:
        (self.sources / "unsigned.sources").write_text(
            "Types: deb\n"
            "URIs: https://packages.example.invalid/os\n"
            "Suites: stable\n"
            "Components: main\n",
            encoding="utf-8",
        )
        count, errors = self.validate()
        self.assertEqual(count, 0)
        self.assertTrue(any("no Signed-By" in error for error in errors))

    def test_rejects_http_and_insecure_overrides(self) -> None:
        (self.sources / "insecure.sources").write_text(
            "Types: deb\n"
            "URIs: http://packages.example.invalid/os\n"
            "Suites: stable\n"
            "Components: main\n"
            f"Signed-By: {self.keyring_path}\n"
            "Trusted: yes\n"
            "Check-Valid-Until: no\n",
            encoding="utf-8",
        )
        _, errors = self.validate()
        self.assertTrue(any("must use HTTPS" in error for error in errors))
        self.assertTrue(any("trusted=yes" in error for error in errors))
        self.assertTrue(any("check-valid-until=no" in error for error in errors))

    def test_rejects_unpinned_or_modified_keyring(self) -> None:
        (self.sources / "genixbit.sources").write_text(
            "Types: deb\n"
            "URIs: https://packages.example.invalid/os\n"
            "Suites: stable\n"
            "Components: main\n"
            f"Signed-By: {self.keyring_path}\n",
            encoding="utf-8",
        )
        self.write_manifest("0" * 64)
        _, errors = self.validate()
        self.assertTrue(any("SHA-256 does not match" in error for error in errors))

        self.manifest.write_text(
            json.dumps({"version": 1, "keyrings": {}}), encoding="utf-8"
        )
        _, errors = self.validate()
        self.assertTrue(any("non-empty 'keyrings'" in error for error in errors))
        self.assertTrue(any("not pinned" in error for error in errors))

    def test_rejects_inline_key_and_missing_sources(self) -> None:
        (self.sources / "inline.sources").write_text(
            "Types: deb\n"
            "URIs: https://packages.example.invalid/os\n"
            "Suites: stable\n"
            "Components: main\n"
            "Signed-By:\n"
            " -----BEGIN PGP PUBLIC KEY BLOCK-----\n",
            encoding="utf-8",
        )
        count, errors = self.validate()
        self.assertEqual(count, 0)
        self.assertTrue(any("inline or multiline" in error for error in errors))

        (self.sources / "inline.sources").unlink()
        count, errors = self.validate()
        self.assertEqual(count, 0)
        self.assertTrue(any("no APT source files" in error for error in errors))


if __name__ == "__main__":
    unittest.main()
