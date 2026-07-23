#!/usr/bin/env python3
"""Validate the GenixBit OS default-installation contract.

This check is intentionally read-only. It verifies that the desktop image seed
selects the expected package and that source-controlled runtime assets required
by the package are present in the repository.
"""

from __future__ import annotations

from pathlib import Path
import sys

ROOT = Path(__file__).resolve().parents[1]
SEED = ROOT / "os-image" / "default-packages.list"
EXPECTED_PACKAGE = "genixbit-software-center"
REQUIRED_ASSETS = (
    "data/com.genixbit.SoftwareCenter.desktop",
    "data/com.genixbit.SoftwareCenter.metainfo.xml",
    "data/com.genixbit.SoftwareCenter.css",
    "data/icons/hicolor/scalable/apps/com.genixbit.SoftwareCenter.svg",
    "data/icons/hicolor/symbolic/apps/com.genixbit.SoftwareCenter-symbolic.svg",
    "dbus/com.genixbit.PackageManager1.xml",
    "systemd/genixpkgd.service",
)


def package_names(seed_text: str) -> list[str]:
    names: list[str] = []
    for raw_line in seed_text.splitlines():
        line = raw_line.split("#", 1)[0].strip()
        if line:
            names.append(line)
    return names


def validate() -> list[str]:
    errors: list[str] = []
    if not SEED.is_file():
        return [f"missing image seed: {SEED.relative_to(ROOT)}"]

    names = package_names(SEED.read_text(encoding="utf-8"))
    if names.count(EXPECTED_PACKAGE) != 1:
        errors.append(
            f"{EXPECTED_PACKAGE!r} must appear exactly once in "
            f"{SEED.relative_to(ROOT)}"
        )
    duplicates = sorted({name for name in names if names.count(name) > 1})
    if duplicates:
        errors.append(f"duplicate packages in image seed: {', '.join(duplicates)}")

    for relative_path in REQUIRED_ASSETS:
        if not (ROOT / relative_path).is_file():
            errors.append(f"missing required runtime asset: {relative_path}")

    return errors


def main() -> int:
    errors = validate()
    if errors:
        for error in errors:
            print(f"error: {error}", file=sys.stderr)
        return 1
    print("GenixBit OS default-installation contract is valid.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
