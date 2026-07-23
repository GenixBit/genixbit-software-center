from __future__ import annotations

import importlib.util
from pathlib import Path
import tempfile
import unittest

SCRIPT = Path(__file__).resolve().parents[1] / "scripts" / "validate-accessibility.py"
SPEC = importlib.util.spec_from_file_location("accessibility_audit", SCRIPT)
assert SPEC is not None and SPEC.loader is not None
accessibility_audit = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(accessibility_audit)

PAGES = (
    "dashboard",
    "discover",
    "installed",
    "updates",
    "activity",
    "stacks",
    "security",
    "services",
    "profiles",
    "settings",
)
SEARCHABLE = (
    "discover",
    "installed",
    "activity",
    "stacks",
    "security",
    "services",
)


class AccessibilityAuditTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        (self.root / "crates/software-center/src").mkdir(parents=True)
        (self.root / "docs").mkdir()

    def write_valid_fixture(self) -> None:
        page_calls = "\n".join(
            f'add_widget_page(\n    &stack,\n    "{page}",\n    "Title",\n    "icon",\n    &widget,\n);'
            for page in PAGES
        )
        mappings = ",\n".join(
            f'("{page}", ui.{page}_entry.clone())' for page in SEARCHABLE
        )
        (self.root / "crates/software-center/src/main.rs").write_text(
            "mod accessibility;\n"
            'let refresh = gtk::Button::with_mnemonic("_Refresh");\n'
            f"{page_calls}\n"
            "accessibility::install_actions(\n"
            "    application,\n"
            "    &stack,\n"
            "    &refresh,\n"
            f"    vec![{mappings}],\n"
            ");\n",
            encoding="utf-8",
        )
        navigation = ",\n".join(
            f'    ("{page}", "<Alt>{index}")'
            for page, index in zip(PAGES, (1, 2, 3, 4, 5, 6, 7, 8, 9, 0))
        )
        (self.root / "crates/software-center/src/accessibility.rs").write_text(
            'pub const REFRESH_ACCELERATOR: &str = "<Primary>r";\n'
            'pub const SEARCH_ACCELERATOR: &str = "<Primary>f";\n'
            'pub const QUIT_ACCELERATOR: &str = "<Primary>q";\n'
            f"pub const NAVIGATION_PAGES: [(&str, &str); 10] = [\n{navigation}\n];\n",
            encoding="utf-8",
        )
        (self.root / "docs/ACCESSIBILITY.md").write_text(
            "Ctrl+R refreshes, Ctrl+F focuses search, and Ctrl+Q quits. "
            "Alt+1 through Alt+0 navigate pages. The screen reader audit is documented, "
            "and package-changing operations remain fail-closed.\n",
            encoding="utf-8",
        )

    def test_accepts_complete_keyboard_policy(self) -> None:
        self.write_valid_fixture()
        self.assertEqual(accessibility_audit.validate(self.root), [])

    def test_rejects_missing_page_navigation_and_search_mapping(self) -> None:
        self.write_valid_fixture()
        accessibility = self.root / "crates/software-center/src/accessibility.rs"
        accessibility.write_text(
            accessibility.read_text(encoding="utf-8").replace(
                '    ("services", "<Alt>8"),\n', ""
            ),
            encoding="utf-8",
        )
        main = self.root / "crates/software-center/src/main.rs"
        main.write_text(
            main.read_text(encoding="utf-8").replace(
                '("security", ui.security_entry.clone()),\n', ""
            ),
            encoding="utf-8",
        )

        errors = accessibility_audit.validate(self.root)
        self.assertTrue(any("services" in error for error in errors))
        self.assertTrue(any("security" in error for error in errors))

    def test_rejects_icon_only_refresh_and_missing_documentation(self) -> None:
        self.write_valid_fixture()
        main = self.root / "crates/software-center/src/main.rs"
        main.write_text(
            main.read_text(encoding="utf-8").replace(
                'gtk::Button::with_mnemonic("_Refresh")', "gtk::Button::new()"
            ),
            encoding="utf-8",
        )
        (self.root / "docs/ACCESSIBILITY.md").write_text(
            "Keyboard audit.\n", encoding="utf-8"
        )

        errors = accessibility_audit.validate(self.root)
        self.assertTrue(any("visible mnemonic" in error for error in errors))
        self.assertTrue(any("Ctrl+R" in error for error in errors))


if __name__ == "__main__":
    unittest.main()
