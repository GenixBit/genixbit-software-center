#!/usr/bin/env python3
"""Validate GenixBit release bundles and retained rollback candidates.

The validator is offline and does not install, remove, upgrade, refresh, or
control services. It validates source metadata, creates deterministic staged
bundle manifests, and verifies a current/rollback bundle pair before an image
or repository release is published.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import stat
import sys
import tomllib
import xml.etree.ElementTree as ET
from pathlib import Path, PurePosixPath
from typing import Any, Iterable

SEMVER = re.compile(r"^(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)$")
ARCHITECTURE = re.compile(r"^[a-z0-9][a-z0-9-]{0,31}$")
SHA256 = re.compile(r"^[0-9a-f]{64}$")


def read_json(path: Path) -> tuple[dict[str, Any], list[str]]:
    try:
        payload = json.loads(path.read_text(encoding="utf-8"))
    except FileNotFoundError:
        return {}, [f"missing JSON file: {path}"]
    except (OSError, UnicodeError, json.JSONDecodeError) as error:
        return {}, [f"unable to read JSON file {path}: {error}"]
    if not isinstance(payload, dict):
        return {}, [f"JSON root must be an object: {path}"]
    return payload, []


def safe_relative_path(value: str) -> bool:
    if not value or "\\" in value or any(character.isspace() for character in value):
        return False
    path = PurePosixPath(value)
    return not path.is_absolute() and ".." not in path.parts and "." not in path.parts


def parse_version(value: object, label: str, errors: list[str]) -> tuple[int, int, int] | None:
    if not isinstance(value, str) or SEMVER.fullmatch(value) is None:
        errors.append(f"{label} must be a stable semantic version (X.Y.Z)")
        return None
    return tuple(int(part) for part in value.split("."))  # type: ignore[return-value]


def string_list(payload: dict[str, Any], key: str, errors: list[str]) -> list[str]:
    value = payload.get(key)
    if not isinstance(value, list) or not value or any(not isinstance(item, str) for item in value):
        errors.append(f"contract field {key!r} must be a non-empty string array")
        return []
    result = list(value)
    if len(result) != len(set(result)):
        errors.append(f"contract field {key!r} must not contain duplicates")
    return result


def load_contract(path: Path) -> tuple[dict[str, Any], list[str]]:
    contract, errors = read_json(path)
    if errors:
        return {}, errors
    if contract.get("schema") != 1:
        errors.append("release contract schema must be 1")
    for key in ("description", "package", "application_id", "service", "version"):
        if not isinstance(contract.get(key), str) or not contract[key]:
            errors.append(f"release contract field {key!r} must be a non-empty string")
    parse_version(contract.get("version"), "contract version", errors)

    crate_manifests = string_list(contract, "crate_manifests", errors)
    source_assets = string_list(contract, "required_source_assets", errors)
    bundle_paths = string_list(contract, "required_bundle_paths", errors)
    architectures = string_list(contract, "architectures", errors)

    for label, values in (
        ("crate manifest", crate_manifests),
        ("source asset", source_assets),
        ("bundle path", bundle_paths),
    ):
        for value in values:
            if not safe_relative_path(value):
                errors.append(f"invalid {label} path: {value!r}")
    for architecture in architectures:
        if ARCHITECTURE.fullmatch(architecture) is None:
            errors.append(f"invalid architecture in release contract: {architecture!r}")

    rollback = contract.get("rollback")
    if not isinstance(rollback, dict):
        errors.append("release contract field 'rollback' must be an object")
    else:
        retained = rollback.get("minimum_retained_releases")
        if not isinstance(retained, int) or isinstance(retained, bool) or retained < 1:
            errors.append("rollback minimum_retained_releases must be at least 1")
        if rollback.get("same_major_only") is not True:
            errors.append("rollback same_major_only must be true")
        if rollback.get("automatic") is not False:
            errors.append("rollback automatic must be false")

    return contract, errors


def desktop_values(path: Path) -> tuple[dict[str, str], list[str]]:
    values: dict[str, str] = {}
    errors: list[str] = []
    try:
        lines = path.read_text(encoding="utf-8").splitlines()
    except (OSError, UnicodeError) as error:
        return {}, [f"unable to read desktop file {path}: {error}"]
    in_section = False
    for line_number, raw_line in enumerate(lines, start=1):
        line = raw_line.strip()
        if not line or line.startswith("#"):
            continue
        if line.startswith("[") and line.endswith("]"):
            in_section = line == "[Desktop Entry]"
            continue
        if not in_section:
            continue
        key, separator, value = line.partition("=")
        if not separator or not key:
            errors.append(f"{path}:{line_number}: malformed desktop entry")
            continue
        values[key] = value
    if not values:
        errors.append(f"{path}: missing [Desktop Entry] values")
    return values, errors


def validate_source_contract(source_root: Path, contract_path: Path) -> list[str]:
    contract, errors = load_contract(contract_path)
    if errors:
        return errors

    cargo_path = source_root / "Cargo.toml"
    try:
        cargo = tomllib.loads(cargo_path.read_text(encoding="utf-8"))
    except FileNotFoundError:
        return [f"missing workspace manifest: {cargo_path}"]
    except (OSError, UnicodeError, tomllib.TOMLDecodeError) as error:
        return [f"unable to read workspace manifest {cargo_path}: {error}"]

    workspace_version = cargo.get("workspace", {}).get("package", {}).get("version")
    parse_version(workspace_version, "workspace version", errors)
    if workspace_version != contract.get("version"):
        errors.append("release contract version does not match workspace.package.version")

    for relative in contract["crate_manifests"]:
        path = source_root / relative
        try:
            crate = tomllib.loads(path.read_text(encoding="utf-8"))
        except FileNotFoundError:
            errors.append(f"missing crate manifest: {relative}")
            continue
        except (OSError, UnicodeError, tomllib.TOMLDecodeError) as error:
            errors.append(f"unable to read crate manifest {relative}: {error}")
            continue
        package = crate.get("package", {})
        if package.get("version", {}).get("workspace") is not True:
            errors.append(f"{relative}: package version must inherit workspace version")

    for relative in contract["required_source_assets"]:
        path = source_root / relative
        if not path.is_file():
            errors.append(f"missing required release source asset: {relative}")

    metainfo_path = source_root / "data/com.genixbit.SoftwareCenter.metainfo.xml"
    try:
        component = ET.parse(metainfo_path).getroot()
    except FileNotFoundError:
        errors.append("missing AppStream metainfo")
    except (OSError, ET.ParseError) as error:
        errors.append(f"unable to parse AppStream metainfo: {error}")
    else:
        component_id = component.findtext("id")
        if component_id != contract.get("application_id"):
            errors.append("AppStream component id does not match release contract")
        stock_icon = component.find("icon[@type='stock']")
        if stock_icon is None or (stock_icon.text or "").strip() != contract.get("application_id"):
            errors.append("AppStream stock icon does not match application id")
        release_versions = [
            release.get("version", "")
            for release in component.findall("./releases/release")
            if release.get("version")
        ]
        if contract.get("version") not in release_versions:
            errors.append("AppStream releases do not include the current contract version")
        parsed_releases = []
        for version in release_versions:
            parsed = parse_version(version, f"AppStream release version {version!r}", errors)
            if parsed is not None:
                parsed_releases.append((parsed, version))
        if parsed_releases and max(parsed_releases)[1] != contract.get("version"):
            errors.append("AppStream newest release version does not match the contract")

    desktop_path = source_root / "data/com.genixbit.SoftwareCenter.desktop"
    desktop, desktop_errors = desktop_values(desktop_path)
    errors.extend(desktop_errors)
    if desktop:
        if desktop.get("Icon") != contract.get("application_id"):
            errors.append("desktop icon does not match application id")
        if desktop.get("Exec") != contract.get("package"):
            errors.append("desktop executable does not match package identity")

    seed_path = source_root / "os-image/default-packages.list"
    try:
        package_names = [
            line.split("#", 1)[0].strip()
            for line in seed_path.read_text(encoding="utf-8").splitlines()
            if line.split("#", 1)[0].strip()
        ]
    except (OSError, UnicodeError) as error:
        errors.append(f"unable to read OS image package seed: {error}")
    else:
        if package_names.count(contract["package"]) != 1:
            errors.append("release package must appear exactly once in the OS image seed")

    return errors


def bundle_files(bundle_root: Path) -> tuple[list[str], list[str]]:
    files: list[str] = []
    errors: list[str] = []
    if not bundle_root.is_dir():
        return [], [f"bundle root is not a directory: {bundle_root}"]
    for path in sorted(bundle_root.rglob("*")):
        relative = path.relative_to(bundle_root).as_posix()
        metadata = path.lstat()
        if stat.S_ISLNK(metadata.st_mode):
            errors.append(f"bundle contains a symbolic link: {relative}")
        elif path.is_file():
            files.append(relative)
        elif not path.is_dir():
            errors.append(f"bundle contains a non-regular entry: {relative}")
    if not files:
        errors.append("bundle contains no files")
    return files, errors


def hash_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def build_manifest(
    bundle_root: Path,
    contract: dict[str, Any],
    version: str,
    architecture: str,
) -> tuple[dict[str, Any], list[str]]:
    errors: list[str] = []
    parse_version(version, "manifest version", errors)
    if architecture not in contract.get("architectures", []):
        errors.append(f"architecture is not allowed by the release contract: {architecture}")
    paths, path_errors = bundle_files(bundle_root)
    errors.extend(path_errors)
    if errors:
        return {}, errors
    required = set(contract["required_bundle_paths"])
    missing = sorted(required - set(paths))
    if missing:
        errors.append(f"bundle is missing required paths: {', '.join(missing)}")
        return {}, errors
    files = {relative: hash_file(bundle_root / relative) for relative in paths}
    return {
        "schema": 1,
        "package": contract["package"],
        "version": version,
        "architecture": architecture,
        "application_id": contract["application_id"],
        "service": contract["service"],
        "files": files,
    }, []


def load_manifest(path: Path) -> tuple[dict[str, Any], list[str]]:
    manifest, errors = read_json(path)
    if errors:
        return {}, errors
    if manifest.get("schema") != 1:
        errors.append(f"{path}: release manifest schema must be 1")
    for key in ("package", "version", "architecture", "application_id", "service"):
        if not isinstance(manifest.get(key), str) or not manifest[key]:
            errors.append(f"{path}: field {key!r} must be a non-empty string")
    parse_version(manifest.get("version"), f"{path}: version", errors)
    architecture = manifest.get("architecture")
    if not isinstance(architecture, str) or ARCHITECTURE.fullmatch(architecture) is None:
        errors.append(f"{path}: invalid architecture")
    files = manifest.get("files")
    if not isinstance(files, dict) or not files:
        errors.append(f"{path}: files must be a non-empty object")
    else:
        for relative, digest in files.items():
            if not isinstance(relative, str) or not safe_relative_path(relative):
                errors.append(f"{path}: invalid bundle path {relative!r}")
            if not isinstance(digest, str) or SHA256.fullmatch(digest) is None:
                errors.append(f"{path}: invalid SHA-256 for {relative!r}")
    return manifest, errors


def verify_bundle(
    bundle_root: Path,
    manifest: dict[str, Any],
    contract: dict[str, Any],
    label: str,
) -> list[str]:
    errors: list[str] = []
    for key in ("package", "application_id", "service"):
        if manifest.get(key) != contract.get(key):
            errors.append(f"{label}: manifest {key} does not match the release contract")
    if manifest.get("architecture") not in contract.get("architectures", []):
        errors.append(f"{label}: manifest architecture is not allowed")
    files = manifest.get("files")
    if not isinstance(files, dict):
        return errors + [f"{label}: manifest has no valid file inventory"]

    required = set(contract["required_bundle_paths"])
    missing_required = sorted(required - set(files))
    if missing_required:
        errors.append(f"{label}: manifest omits required paths: {', '.join(missing_required)}")

    actual_paths, path_errors = bundle_files(bundle_root)
    errors.extend(f"{label}: {error}" for error in path_errors)
    expected_paths = set(files)
    actual_set = set(actual_paths)
    missing_files = sorted(expected_paths - actual_set)
    extra_files = sorted(actual_set - expected_paths)
    if missing_files:
        errors.append(f"{label}: bundle is missing manifest files: {', '.join(missing_files)}")
    if extra_files:
        errors.append(f"{label}: bundle has unmanifested files: {', '.join(extra_files)}")

    for relative in sorted(expected_paths & actual_set):
        path = bundle_root / relative
        metadata = path.stat()
        if metadata.st_mode & (stat.S_IWGRP | stat.S_IWOTH):
            errors.append(f"{label}: bundle file is group- or world-writable: {relative}")
        actual_digest = hash_file(path)
        if actual_digest != files[relative]:
            errors.append(f"{label}: SHA-256 mismatch for {relative}")
    return errors


def validate_release_pair(
    contract: dict[str, Any],
    current_root: Path,
    current_manifest: dict[str, Any],
    rollback_root: Path,
    rollback_manifest: dict[str, Any],
) -> list[str]:
    errors = verify_bundle(current_root, current_manifest, contract, "current")
    errors.extend(verify_bundle(rollback_root, rollback_manifest, contract, "rollback"))

    current_version = parse_version(current_manifest.get("version"), "current version", errors)
    rollback_version = parse_version(rollback_manifest.get("version"), "rollback version", errors)
    if current_manifest.get("version") != contract.get("version"):
        errors.append("current manifest version does not match the release contract")
    if current_manifest.get("architecture") != rollback_manifest.get("architecture"):
        errors.append("current and rollback architectures must match")
    if current_version is not None and rollback_version is not None:
        if rollback_version >= current_version:
            errors.append("rollback version must be older than the current version")
        if contract["rollback"]["same_major_only"] and rollback_version[0] != current_version[0]:
            errors.append("rollback version must remain within the current major version")
    return errors


def write_manifest(path: Path, manifest: dict[str, Any]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(manifest, indent=2, sort_keys=True) + "\n", encoding="utf-8")


def print_errors(errors: Iterable[str]) -> int:
    collected = list(errors)
    for error in collected:
        print(f"error: {error}", file=sys.stderr)
    return 1 if collected else 0


def parser() -> argparse.ArgumentParser:
    result = argparse.ArgumentParser(description=__doc__)
    subparsers = result.add_subparsers(dest="command", required=True)

    source = subparsers.add_parser("source", help="validate source release metadata")
    source.add_argument("--source-root", type=Path, required=True)
    source.add_argument("--contract", type=Path, required=True)

    manifest = subparsers.add_parser("manifest", help="create a staged bundle manifest")
    manifest.add_argument("--contract", type=Path, required=True)
    manifest.add_argument("--bundle-root", type=Path, required=True)
    manifest.add_argument("--version", required=True)
    manifest.add_argument("--architecture", required=True)
    manifest.add_argument("--output", type=Path, required=True)

    pair = subparsers.add_parser("pair", help="validate current and rollback bundles")
    pair.add_argument("--contract", type=Path, required=True)
    pair.add_argument("--current-root", type=Path, required=True)
    pair.add_argument("--current-manifest", type=Path, required=True)
    pair.add_argument("--rollback-root", type=Path, required=True)
    pair.add_argument("--rollback-manifest", type=Path, required=True)
    return result


def main(argv: list[str] | None = None) -> int:
    arguments = parser().parse_args(argv)
    contract, contract_errors = load_contract(arguments.contract)
    if contract_errors:
        return print_errors(contract_errors)

    if arguments.command == "source":
        return print_errors(validate_source_contract(arguments.source_root, arguments.contract))

    if arguments.command == "manifest":
        manifest, errors = build_manifest(
            arguments.bundle_root,
            contract,
            arguments.version,
            arguments.architecture,
        )
        if errors:
            return print_errors(errors)
        write_manifest(arguments.output, manifest)
        print(f"wrote release manifest: {arguments.output}")
        return 0

    current, current_errors = load_manifest(arguments.current_manifest)
    rollback, rollback_errors = load_manifest(arguments.rollback_manifest)
    errors = current_errors + rollback_errors
    if not errors:
        errors.extend(
            validate_release_pair(
                contract,
                arguments.current_root,
                current,
                arguments.rollback_root,
                rollback,
            )
        )
    if errors:
        return print_errors(errors)
    print(
        f"release {current['version']} and rollback {rollback['version']} "
        f"are valid for {current['architecture']}."
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
