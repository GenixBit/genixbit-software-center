#!/usr/bin/env python3
"""Validate the source-controlled release and rollback compatibility contract.

This tool is read-only. It does not install, downgrade, remove, refresh, or
execute packaged software. It verifies that release metadata is internally
consistent and that every declared rollback target can read the persistent
formats produced by the current release.
"""

from __future__ import annotations

import argparse
import json
import re
import sys
import tomllib
from pathlib import Path
from typing import Any

VERSION_RE = re.compile(r"^(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)$")
FORMAT_KEYS = (
    "dbus_api",
    "transaction_journal",
    "event_journal",
    "settings_format",
    "system_profile_format",
)


def parse_version(value: Any, label: str, errors: list[str]) -> tuple[int, int, int] | None:
    if not isinstance(value, str) or VERSION_RE.fullmatch(value) is None:
        errors.append(f"{label} must be a semantic version in MAJOR.MINOR.PATCH form")
        return None
    return tuple(int(part) for part in value.split("."))  # type: ignore[return-value]


def read_json(path: Path, errors: list[str]) -> dict[str, Any]:
    try:
        payload = json.loads(path.read_text(encoding="utf-8"))
    except FileNotFoundError:
        errors.append(f"missing release policy: {path}")
        return {}
    except (OSError, UnicodeError, json.JSONDecodeError) as error:
        errors.append(f"unable to read release policy {path}: {error}")
        return {}
    if not isinstance(payload, dict):
        errors.append("release policy root must be a JSON object")
        return {}
    return payload


def workspace_version(root: Path, errors: list[str]) -> str | None:
    path = root / "Cargo.toml"
    try:
        payload = tomllib.loads(path.read_text(encoding="utf-8"))
        value = payload["workspace"]["package"]["version"]
    except (FileNotFoundError, OSError, UnicodeError, tomllib.TOMLDecodeError, KeyError, TypeError) as error:
        errors.append(f"unable to read workspace version from {path}: {error}")
        return None
    if not isinstance(value, str):
        errors.append("workspace package version must be a string")
        return None
    return value


def validate_format_record(record: Any, label: str, errors: list[str]) -> dict[str, int] | None:
    if not isinstance(record, dict):
        errors.append(f"{label} must be an object")
        return None
    normalized: dict[str, int] = {}
    for key in FORMAT_KEYS:
        value = record.get(key)
        if not isinstance(value, int) or isinstance(value, bool) or value < 1:
            errors.append(f"{label}.{key} must be a positive integer")
            continue
        normalized[key] = value
    return normalized if len(normalized) == len(FORMAT_KEYS) else None


def validate(root: Path, policy_path: Path) -> list[str]:
    errors: list[str] = []
    policy = read_json(policy_path, errors)
    if not policy:
        return errors

    if policy.get("schema_version") != 1:
        errors.append("release policy schema_version must be 1")

    application_id = policy.get("application_id")
    if application_id != "com.genixbit.SoftwareCenter":
        errors.append("release policy application_id must be com.genixbit.SoftwareCenter")

    current = policy.get("current")
    if not isinstance(current, dict):
        errors.append("release policy current entry must be an object")
        return errors

    current_text = current.get("version")
    current_version = parse_version(current_text, "current.version", errors)
    current_formats = validate_format_record(current, "current", errors)

    cargo_version = workspace_version(root, errors)
    if isinstance(current_text, str) and cargo_version is not None and current_text != cargo_version:
        errors.append(
            f"release policy current.version {current_text} does not match workspace version {cargo_version}"
        )

    metainfo = root / "data" / "com.genixbit.SoftwareCenter.metainfo.xml"
    try:
        metainfo_text = metainfo.read_text(encoding="utf-8")
    except (FileNotFoundError, OSError, UnicodeError) as error:
        errors.append(f"unable to read AppStream metadata {metainfo}: {error}")
    else:
        if isinstance(current_text, str) and f'<release version="{current_text}"' not in metainfo_text:
            errors.append(f"AppStream metadata has no release entry for {current_text}")

    assets = policy.get("required_assets")
    if not isinstance(assets, list) or not assets:
        errors.append("required_assets must be a non-empty array")
    else:
        seen_assets: set[str] = set()
        for item in assets:
            if not isinstance(item, str) or not item or item.startswith("/") or ".." in Path(item).parts:
                errors.append(f"invalid required asset path: {item!r}")
                continue
            if item in seen_assets:
                errors.append(f"duplicate required asset: {item}")
                continue
            seen_assets.add(item)
            if not (root / item).is_file():
                errors.append(f"missing required release asset: {item}")

    targets = policy.get("rollback_targets")
    if not isinstance(targets, list) or not targets:
        errors.append("rollback_targets must be a non-empty array")
        return errors

    seen_versions: set[str] = set()
    for index, target in enumerate(targets):
        label = f"rollback_targets[{index}]"
        if not isinstance(target, dict):
            errors.append(f"{label} must be an object")
            continue
        target_text = target.get("version")
        target_version = parse_version(target_text, f"{label}.version", errors)
        target_formats = validate_format_record(target, label, errors)
        if isinstance(target_text, str):
            if target_text in seen_versions:
                errors.append(f"duplicate rollback target version: {target_text}")
            seen_versions.add(target_text)
        if current_version is not None and target_version is not None and target_version >= current_version:
            errors.append(f"{label}.version must be older than current.version")
        if current_formats is not None and target_formats is not None:
            if target_formats["dbus_api"] != current_formats["dbus_api"]:
                errors.append(f"{label}.dbus_api must equal current.dbus_api")
            for key in FORMAT_KEYS[1:]:
                if target_formats[key] < current_formats[key]:
                    errors.append(
                        f"{label}.{key} cannot read current format {current_formats[key]}"
                    )

    return errors


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", type=Path, default=Path(__file__).resolve().parents[1])
    parser.add_argument(
        "--policy",
        type=Path,
        default=Path(__file__).resolve().parents[1] / "release" / "release-policy.json",
    )
    args = parser.parse_args()
    errors = validate(args.root.resolve(), args.policy.resolve())
    if errors:
        for error in errors:
            print(f"error: {error}", file=sys.stderr)
        return 1
    print("Release and rollback compatibility contract is valid.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
