#!/usr/bin/env python3
from __future__ import annotations
import argparse, hashlib, json, re, sys
from pathlib import Path

PKG = "genixbit-software-center"
ARCH = re.compile(r"^[a-z0-9][a-z0-9-]*$")
VER = re.compile(r"^[0-9]+(?:\.[0-9]+){2}$")
SHA = re.compile(r"^[0-9a-f]{64}$")


def version_tuple(value: str) -> tuple[int, int, int]:
    if not VER.fullmatch(value):
        raise ValueError(value)
    return tuple(int(part) for part in value.split("."))  # type: ignore[return-value]


def validate(root: Path, manifest_path: Path) -> list[str]:
    errors: list[str] = []
    try:
        data = json.loads(manifest_path.read_text(encoding="utf-8"))
    except Exception as error:
        return [f"unable to read release manifest: {error}"]
    if data.get("version") != 1:
        errors.append("manifest version must be 1")
    releases = data.get("releases")
    if not isinstance(releases, list) or len(releases) != 2:
        return errors + ["manifest must contain current and rollback releases"]
    parsed = []
    for index, release in enumerate(releases):
        label = "current" if index == 0 else "rollback"
        if release.get("role") != label:
            errors.append(f"release {index} role must be {label}")
        if release.get("package") != PKG:
            errors.append(f"{label} package identity mismatch")
        version = release.get("version", "")
        architecture = release.get("architecture", "")
        if not isinstance(version, str) or not VER.fullmatch(version):
            errors.append(f"{label} version must be semantic x.y.z")
        if not isinstance(architecture, str) or not ARCH.fullmatch(architecture):
            errors.append(f"{label} architecture is invalid")
        artifact = release.get("artifact", "")
        digest = release.get("sha256", "")
        if not isinstance(artifact, str) or Path(artifact).name != artifact:
            errors.append(f"{label} artifact must be a basename")
            continue
        if not isinstance(digest, str) or not SHA.fullmatch(digest):
            errors.append(f"{label} SHA-256 is invalid")
            continue
        path = root / artifact
        try:
            content = path.read_bytes()
        except OSError as error:
            errors.append(f"unable to read {label} artifact: {error}")
            continue
        if not content:
            errors.append(f"{label} artifact is empty")
        if hashlib.sha256(content).hexdigest() != digest:
            errors.append(f"{label} artifact SHA-256 mismatch")
        parsed.append((version, architecture))
    if len(parsed) == 2:
        current, rollback = parsed
        if current[1] != rollback[1]:
            errors.append("rollback architecture must match current release")
        try:
            if version_tuple(rollback[0]) >= version_tuple(current[0]):
                errors.append("rollback version must be older than current release")
        except ValueError:
            pass
    policy = data.get("rollback_policy", {})
    if policy.get("retained_releases") != 1:
        errors.append("rollback policy must retain exactly one previous release")
    if policy.get("automatic") is not False:
        errors.append("automatic rollback must remain disabled")
    return errors


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", type=Path, required=True)
    parser.add_argument("--manifest", type=Path, required=True)
    args = parser.parse_args()
    errors = validate(args.root, args.manifest)
    for error in errors:
        print(f"error: {error}", file=sys.stderr)
    return 1 if errors else 0


if __name__ == "__main__":
    raise SystemExit(main())
