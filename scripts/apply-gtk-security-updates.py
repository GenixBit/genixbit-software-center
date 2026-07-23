from pathlib import Path

path = Path("crates/software-center/src/main.rs")
text = path.read_text()


def replace_once(old: str, new: str) -> None:
    global text
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"expected one marker, found {count}: {old[:100]!r}")
    text = text.replace(old, new, 1)


replace_once(
    "mod client;\n",
    "mod client;\nmod security_view;\n",
)
replace_once(
    "use gtk::glib;\n",
    "use gtk::glib;\nuse security_view::{security_updates, summarize_security};\n",
)
replace_once(
    "    updates_status: gtk::Label,\n    updates_list: gtk::ListBox,\n",
    "    updates_status: gtk::Label,\n    updates_list: gtk::ListBox,\n    security_status: gtk::Label,\n    security_list: gtk::ListBox,\n",
)
replace_once(
    '''    add_placeholder_page(
        &stack,
        "security",
        "Security",
        "security-high-symbolic",
        "Security status",
        "Package advisories, signature verification and repository trust will appear here.",
    );
''',
    '''    let (security_page, security_status, security_list) = build_list_page(
        "Package security updates",
        "Review security-classified package updates reported by the configured APT metadata.",
    );
    add_widget_page(
        &stack,
        "security",
        "Security",
        "security-high-symbolic",
        &security_page,
    );
''',
)
replace_once(
    "        updates_status,\n        updates_list,\n        activity_entry,\n",
    "        updates_status,\n        updates_list,\n        security_status,\n        security_list,\n        activity_entry,\n",
)
replace_once(
    '''    render_installed(ui);
    render_updates(ui, &snapshot.updates);
''',
    '''    render_installed(ui);
    render_updates(ui, &snapshot.updates);
    render_security(ui, &snapshot.updates);
''',
)

marker = "fn start_activity_load(ui: &UiState) {"
index = text.index(marker)
render_security = '''fn render_security(ui: &UiState, updates: &[UpdateRecord]) {
    clear_list(&ui.security_list);
    let summary = summarize_security(updates);
    ui.security_status.set_text(&summary.status_text());

    let security = security_updates(updates);
    if security.is_empty() {
        let row = adw::ActionRow::builder()
            .title("No security updates reported")
            .subtitle("APT metadata currently classifies no available package updates as security updates.")
            .build();
        let badge = gtk::Label::new(Some("Current"));
        badge.add_css_class("success");
        row.add_suffix(&badge);
        ui.security_list.append(&row);
        return;
    }

    for update in security {
        let subtitle = format!(
            "{} → {} · {} · {}",
            update.current_version, update.candidate_version, update.architecture, update.source
        );
        let row = adw::ActionRow::builder()
            .title(&update.name)
            .subtitle(&subtitle)
            .activatable(true)
            .build();
        let badge = gtk::Label::new(Some("Security"));
        badge.add_css_class("error");
        row.add_suffix(&badge);
        let callback_ui = ui.clone();
        let package_name = update.name.clone();
        row.connect_activated(move |_| start_package_details(&callback_ui, &package_name));
        ui.security_list.append(&row);
    }
}

'''
text = text[:index] + render_security + text[index:]

replace_once(
    '''    ui.installed_status.set_text(message);
    ui.updates_status.set_text(message);
''',
    '''    ui.installed_status.set_text(message);
    ui.updates_status.set_text(message);
    ui.security_status.set_text(message);
    clear_list(&ui.security_list);
''',
)

path.write_text(text)
