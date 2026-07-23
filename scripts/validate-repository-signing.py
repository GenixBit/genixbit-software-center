#!/usr/bin/env python3
"""Validate APT repository signing configuration without contacting repositories.

The validator is intentionally read-only. It audits an image root, requires every
active APT source to use HTTPS and an explicit per-repository keyring, rejects
insecure trust overrides, and verifies each keyring against a pinned SHA-256
manifest supplied by the image build.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import shlex
import stat
import sys
from dataclasses import dataclass
from pathlib import Path, PurePosixPath
from typing import Iterable

KEYRING_PREFIX = PurePosixPath("/usr/share/keyrings")
KEYRING_SUFFIX = ".gpg"
INSECURE_OPTIONS = {
    "trusted": "yes",
    "allow-insecure": "yes",
    "allow-weak": "yes",
    "check-valid-until": "no",
}
LEGACY_SOURCE = re.compile(
    r"^(?P<kind>deb(?:-src)?)\s+"
    r"(?:\[(?P<options>[^\]]+)\]\s+)?"
    r"(?P<uri>\S+)\s+(?P<suite>\S+)(?:\s+.*)?$"
)


@dataclass(frozen=True)
class SourceEntry:
    source: Path
    line: int
    uris: tuple[str, ...]
    signed_by: str
    options: dict[str, str]


def load_manifest(path: Path) -> tuple[dict[str, str], list[str]]:
    errors: list[str] = []
    try:
        payload = json.loads(path.read_text(encoding="utf-8"))
    except FileNotFoundError:
        return {}, [f"missing keyring manifest: {path}"]
    except (OSError, UnicodeError, json.JSONDecodeError) as error:
        return {}, [f"unable to read keyring manifest {path}: {error}"]

    if payload.get("version") != 1:
        errors.append("keyring manifest version must be 1")
    keyrings = payload.get("keyrings")
    if not isinstance(keyrings, dict) or not keyrings:
        errors.append("keyring manifest must contain a non-empty 'keyrings' object")
        return {}, errors

    normalized: dict[str, str] = {}
    for raw_path, raw_digest in keyrings.items():
        if not isinstance(raw_path, str) or not isinstance(raw_digest, str):
            errors.append("keyring manifest paths and digests must be strings")
            continue
        if not valid_keyring_path(raw_path):
            errors.append(f"invalid keyring manifest path: {raw_path!r}")
            continue
        digest = raw_digest.lower()
        if not re.fullmatch(r"[0-9a-f]{64}", digest):
            errors.append(f"invalid SHA-256 digest for {raw_path}")
            continue
        normalized[raw_path] = digest
    return normalized, errors


def valid_keyring_path(value: str) -> bool:
    if any(character.isspace() for character in value):
        return False
    path = PurePosixPath(value)
    try:
        path.relative_to(KEYRING_PREFIX)
    except ValueError:
        return False
    return path.is_absolute() and path.suffix == KEYRING_SUFFIX and ".." not in path.parts


def parse_deb822(path: Path) -> tuple[list[SourceEntry], list[str]]:
    entries: list[SourceEntry] = []
    errors: list[str] = []
    text = path.read_text(encoding="utf-8")
    paragraphs: list[tuple[int, list[str]]] = []
    current: list[str] = []
    start_line = 1
    for line_number, raw_line in enumerate(text.splitlines(), start=1):
        if not raw_line.strip():
            if current:
                paragraphs.append((start_line, current))
                current = []
            start_line = line_number + 1
            continue
        if not current:
            start_line = line_number
        current.append(raw_line)
    if current:
        paragraphs.append((start_line, current))

    for line_number, lines in paragraphs:
        fields: dict[str, str] = {}
        last_key: str | None = None
        malformed = False
        for raw_line in lines:
            if raw_line[:1].isspace():
                if last_key is None:
                    errors.append(f"{path}:{line_number}: orphan continuation line")
                    malformed = True
                    break
                fields[last_key] += "\n" + raw_line.strip()
                continue
            if ":" not in raw_line:
                errors.append(f"{path}:{line_number}: malformed Deb822 field")
                malformed = True
                break
            raw_key, raw_value = raw_line.split(":", 1)
            key = raw_key.strip().lower()
            if not key or key in fields:
                errors.append(f"{path}:{line_number}: duplicate or empty Deb822 field")
                malformed = True
                break
            fields[key] = raw_value.strip()
            last_key = key
        if malformed or fields.get("enabled", "yes").lower() == "no":
            continue

        types = fields.get("types", "").split()
        uris = tuple(fields.get("uris", "").split())
        signed_by = fields.get("signed-by", "")
        if not types or any(value not in {"deb", "deb-src"} for value in types):
            errors.append(f"{path}:{line_number}: Types must contain only deb or deb-src")
        if not uris:
            errors.append(f"{path}:{line_number}: active source has no URIs")
        if not signed_by:
            errors.append(f"{path}:{line_number}: active source has no Signed-By")
        if "\n" in signed_by:
            errors.append(f"{path}:{line_number}: inline or multiline Signed-By is forbidden")
        if types and uris and signed_by and "\n" not in signed_by:
            entries.append(
                SourceEntry(
                    source=path,
                    line=line_number,
                    uris=uris,
                    signed_by=signed_by,
                    options=fields,
                )
            )
    return entries, errors


def parse_legacy(path: Path) -> tuple[list[SourceEntry], list[str]]:
    entries: list[SourceEntry] = []
    errors: list[str] = []
    for line_number, raw_line in enumerate(path.read_text(encoding="utf-8").splitlines(), start=1):
        line = raw_line.split("#", 1)[0].strip()
        if not line:
            continue
        match = LEGACY_SOURCE.match(line)
        if match is None:
            errors.append(f"{path}:{line_number}: malformed legacy source entry")
            continue
        options: dict[str, str] = {}
        try:
            for token in shlex.split(match.group("options") or ""):
                key, separator, value = token.partition("=")
                if not separator or not key or not value:
                    raise ValueError(token)
                options[key.lower()] = value
        except ValueError:
            errors.append(f"{path}:{line_number}: malformed source option")
            continue
        signed_by = options.get("signed-by", "")
        if not signed_by:
            errors.append(f"{path}:{line_number}: active source has no signed-by option")
            continue
        entries.append(
            SourceEntry(
                source=path,
                line=line_number,
                uris=(match.group("uri"),),
                signed_by=signed_by,
                options=options,
            )
        )
    return entries, errors


def source_files(root: Path) -> list[Path]:
    apt = root / "etc" / "apt"
    files: list[Path] = []
    legacy = apt / "sources.list"
    if legacy.is_file():
        files.append(legacy)
    directory = apt / "sources.list.d"
    if directory.is_dir():
        files.extend(sorted(directory.glob("*.list")))
        files.extend(sorted(directory.glob("*.sources")))
    return files


def validate_entry(root: Path, entry: SourceEntry, manifest: dict[str, str]) -> list[str]:
    prefix = f"{entry.source}:{entry.line}"
    errors: list[str] = []
    for uri in entry.uris:
        if not uri.startswith("https://"):
            errors.append(f"{prefix}: repository URI must use HTTPS: {uri}")
    for option, forbidden in INSECURE_OPTIONS.items():
        actual = entry.options.get(option)
        if actual is not None and actual.lower() == forbidden:
            errors.append(f"{prefix}: insecure option {option}={actual} is forbidden")

    if not valid_keyring_path(entry.signed_by):
        errors.append(
            f"{prefix}: Signed-By must be one absolute .gpg path under /usr/share/keyrings"
        )
        return errors
    expected_digest = manifest.get(entry.signed_by)
    if expected_digest is None:
        errors.append(f"{prefix}: keyring is not pinned in the SHA-256 manifest")
        return errors

    keyring = root / entry.signed_by.lstrip("/")
    try:
        metadata = keyring.stat()
        content = keyring.read_bytes()
    except OSError as error:
        errors.append(f"{prefix}: unable to read keyring {entry.signed_by}: {error}")
        return errors
    if not stat.S_ISREG(metadata.st_mode) or not content:
        errors.append(f"{prefix}: keyring must be a non-empty regular file")
    if metadata.st_mode & (stat.S_IWGRP | stat.S_IWOTH):
        errors.append(f"{prefix}: keyring must not be group- or world-writable")
    actual_digest = hashlib.sha256(content).hexdigest()
    if actual_digest != expected_digest:
        errors.append(f"{prefix}: keyring SHA-256 does not match the manifest")
    return errors


def validate(root: Path, manifest_path: Path) -> tuple[int, list[str]]:
    manifest, errors = load_manifest(manifest_path)
    files = source_files(root)
    if not files:
        errors.append(f"no APT source files found below {root / 'etc/apt'}")
        return 0, errors

    entries: list[SourceEntry] = []
    for path in files:
        try:
            parsed, parse_errors = (
                parse_deb822(path) if path.suffix == ".sources" else parse_legacy(path)
            )
        except (OSError, UnicodeError) as error:
            errors.append(f"unable to read source file {path}: {error}")
            continue
        entries.extend(parsed)
        errors.extend(parse_errors)
    if not entries:
        errors.append("no valid active APT repository entries found")
    for entry in entries:
        errors.extend(validate_entry(root, entry, manifest))
    return len(entries), errors


def parser() -> argparse.ArgumentParser:
    result = argparse.ArgumentParser(description=__doc__)
    result.add_argument("--root", type=Path, required=True, help="image root to audit")
    result.add_argument(
        "--manifest",
        type=Path,
        required=True,
        help="JSON manifest mapping keyring paths to SHA-256 digests",
    )
    return result


def main(argv: Iterable[str] | None = None) -> int:
    arguments = parser().parse_args(argv)
    count, errors = validate(arguments.root.resolve(), arguments.manifest.resolve())
    if errors:
        for error in errors:
            print(f"error: {error}", file=sys.stderr)
        return 1
    print(f"Repository signing contract is valid for {count} active source entr{'y' if count == 1 else 'ies'}.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
