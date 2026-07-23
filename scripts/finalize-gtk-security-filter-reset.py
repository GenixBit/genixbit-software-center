from pathlib import Path

path = Path("crates/software-center/src/main.rs")
text = path.read_text()

replacements = [
    (
        "use security_view::{ALL_SECURITY_SOURCES, filter_security_updates, summarize_security};",
        "use security_view::{\n    ALL_SECURITY_SOURCES, filter_security_updates, security_filters_active, summarize_security,\n};",
    ),
    (
        "    security_entry: gtk::SearchEntry,\n    security_source: gtk::DropDown,\n    security_status: gtk::Label,",
        "    security_entry: gtk::SearchEntry,\n    security_source: gtk::DropDown,\n    security_reset: gtk::Button,\n    security_status: gtk::Label,",
    ),
    (
        "    let (security_page, security_entry, security_source, security_status, security_list) =\n        build_security_page();",
        "    let (\n        security_page,\n        security_entry,\n        security_source,\n        security_reset,\n        security_status,\n        security_list,\n    ) = build_security_page();",
    ),
    (
        "        security_entry,\n        security_source,\n        security_status,",
        "        security_entry,\n        security_source,\n        security_reset,\n        security_status,",
    ),
    (
        "    {\n        let ui = ui.clone();\n        ui.activity_entry",
        "    {\n        let ui = ui.clone();\n        ui.security_reset.clone().connect_clicked(move |_| {\n            ui.security_entry.set_text(\"\");\n            ui.security_source.set_selected(0);\n            render_security(&ui);\n        });\n    }\n    {\n        let ui = ui.clone();\n        ui.activity_entry",
    ),
]

for old, new in replacements:
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"expected one marker, found {count}: {old[:100]!r}")
    text = text.replace(old, new, 1)

path.write_text(text)
