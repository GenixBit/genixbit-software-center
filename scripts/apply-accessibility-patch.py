#!/usr/bin/env python3
from __future__ import annotations

from pathlib import Path

MAIN = Path("crates/software-center/src/main.rs")
WORKFLOW = Path(".github/workflows/apply-accessibility.yml")
SELF = Path(__file__)


def replace_once(text: str, old: str, new: str, label: str) -> str:
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"expected one {label} marker, found {count}")
    return text.replace(old, new, 1)


def main() -> None:
    text = MAIN.read_text(encoding="utf-8")

    if "mod accessibility;" not in text:
        text = replace_once(
            text,
            "mod activity_filter;\n",
            "mod accessibility;\nmod activity_filter;\n",
            "module",
        )

    old_refresh = '''    let refresh = gtk::Button::builder()
        .icon_name("view-refresh-symbolic")
        .tooltip_text("Refresh package metadata")
        .build();
'''
    new_refresh = '''    let refresh = gtk::Button::with_mnemonic("_Refresh");
    refresh.set_tooltip_text(Some("Refresh all local metadata (Ctrl+R)"));
'''
    if 'gtk::Button::with_mnemonic("_Refresh")' not in text:
        text = replace_once(text, old_refresh, new_refresh, "refresh button")

    action_install = '''    accessibility::install_actions(
        application,
        &stack,
        &refresh,
        vec![
            ("discover", ui.discover_entry.clone()),
            ("installed", ui.installed_entry.clone()),
            ("activity", ui.activity_entry.clone()),
            ("stacks", ui.stacks_entry.clone()),
            ("security", ui.security_entry.clone()),
            ("services", ui.services_entry.clone()),
        ],
    );

'''
    action_marker = '''    {
        let ui = ui.clone();
        refresh.connect_clicked(move |_| {
'''
    if "accessibility::install_actions(" not in text:
        text = replace_once(
            text,
            action_marker,
            action_install + action_marker,
            "action installation",
        )

    MAIN.write_text(text, encoding="utf-8")
    WORKFLOW.unlink(missing_ok=True)
    SELF.unlink(missing_ok=True)


if __name__ == "__main__":
    main()
