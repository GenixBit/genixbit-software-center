#!/usr/bin/env python3
"""Validate keyboard-navigation and accessible-control policy offline.

The validator is intentionally source-only. It verifies that every GTK page has
an explicit keyboard destination, every searchable page participates in the
page-aware search action, and icon-only refresh control has been replaced by a
visible mnemonic label.
"""

from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path

MAIN_PATH = Path("crates/software-center/src/main.rs")
ACCESSIBILITY_PATH = Path("crates/software-center/src/accessibility.rs")
DOC_PATH = Path("docs/ACCESSIBILITY.md")

PAGE_PATTERN = re.compile(
    r"add_widget_page\(\s*&stack,\s*\"(?P<page>[a-z0-9-]+)\"",
    re.MULTILINE,
)
NAVIGATION_PATTERN = re.compile(
    r"\(\"(?P<page>[a-z0-9-]+)\",\s*\"(?P<accelerator><Alt>[0-9])\"\)"
)
SEARCH_MAPPING_PATTERN = re.compile(
    r"\(\"(?P<page>[a-z0-9-]+)\",\s*ui\.[a-z0-9_]+\.clone\(\)\)"
)

REQUIRED_GLOBAL_ACCELERATORS = {
    "REFRESH_ACCELERATOR": "<Primary>r",
    "SEARCH_ACCELERATOR": "<Primary>f",
    "QUIT_ACCELERATOR": "<Primary>q",
}
REQUIRED_SEARCHABLE_PAGES = {
    "discover",
    "installed",
    "activity",
    "stacks",
    "security",
    "services",
}


def read_text(root: Path, relative: Path, errors: list[str]) -> str:
    path = root / relative
    try:
        return path.read_text(encoding="utf-8")
    except FileNotFoundError:
        errors.append(f"missing accessibility asset: {relative}")
    except (OSError, UnicodeError) as error:
        errors.append(f"unable to read {relative}: {error}")
    return ""


def validate(root: Path) -> list[str]:
    errors: list[str] = []
    main = read_text(root, MAIN_PATH, errors)
    accessibility = read_text(root, ACCESSIBILITY_PATH, errors)
    documentation = read_text(root, DOC_PATH, errors)
    if errors:
        return errors

    if "mod accessibility;" not in main:
        errors.append("GTK application must include the accessibility module")
    if "accessibility::install_actions(" not in main:
        errors.append("GTK application must install accessibility actions")
    if 'gtk::Button::with_mnemonic("_Refresh")' not in main:
        errors.append("refresh control must use a visible mnemonic label")

    pages = PAGE_PATTERN.findall(main)
    if not pages:
        errors.append("no GTK stack pages were discovered")
    elif len(pages) != len(set(pages)):
        errors.append("GTK stack page names must be unique")

    navigation_matches = NAVIGATION_PATTERN.findall(accessibility)
    navigation_pages = [page for page, _ in navigation_matches]
    accelerators = [accelerator for _, accelerator in navigation_matches]
    if set(navigation_pages) != set(pages):
        missing = sorted(set(pages) - set(navigation_pages))
        extra = sorted(set(navigation_pages) - set(pages))
        if missing:
            errors.append(f"pages missing keyboard navigation: {', '.join(missing)}")
        if extra:
            errors.append(f"keyboard navigation references unknown pages: {', '.join(extra)}")
    if len(accelerators) != len(set(accelerators)):
        errors.append("page-navigation accelerators must be unique")

    mapped_search_pages = set(SEARCH_MAPPING_PATTERN.findall(main))
    missing_search = sorted(REQUIRED_SEARCHABLE_PAGES - mapped_search_pages)
    if missing_search:
        errors.append(
            "searchable pages missing Ctrl+F focus mapping: " + ", ".join(missing_search)
        )
    unexpected_search = sorted(mapped_search_pages - REQUIRED_SEARCHABLE_PAGES)
    if unexpected_search:
        errors.append(
            "unreviewed page-aware search mappings: " + ", ".join(unexpected_search)
        )

    for constant, accelerator in REQUIRED_GLOBAL_ACCELERATORS.items():
        declaration = f'pub const {constant}: &str = "{accelerator}";'
        if declaration not in accessibility:
            errors.append(f"missing global accelerator declaration: {declaration}")

    required_doc_phrases = (
        "Ctrl+R",
        "Ctrl+F",
        "Ctrl+Q",
        "Alt+1",
        "screen reader",
        "package-changing operations remain fail-closed",
    )
    for phrase in required_doc_phrases:
        if phrase not in documentation:
            errors.append(f"accessibility documentation is missing {phrase!r}")

    return errors


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", type=Path, default=Path.cwd())
    arguments = parser.parse_args()

    errors = validate(arguments.root.resolve())
    if errors:
        for error in errors:
            print(f"error: {error}", file=sys.stderr)
        return 1
    print("Accessibility and keyboard-navigation policy is valid.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
