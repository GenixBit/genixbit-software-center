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
    "use security_view::{security_updates, summarize_security};",
    "use security_view::{ALL_SECURITY_SOURCES, filter_security_updates, summarize_security};",
)
replace_once(
    "    security_status: gtk::Label,\n    security_list: gtk::ListBox,\n",
    "    security_entry: gtk::SearchEntry,\n    security_source: gtk::DropDown,\n    security_status: gtk::Label,\n    security_list: gtk::ListBox,\n",
)
replace_once(
    "    activity_records: Rc<RefCell<Vec<TransactionRecord>>>,\n",
    "    activity_records: Rc<RefCell<Vec<TransactionRecord>>>,\n    security_updates: Rc<RefCell<Vec<UpdateRecord>>>,\n",
)
replace_once(
    '''    let (security_page, security_status, security_list) = build_list_page(
        "Package security updates",
        "Review security-classified package updates reported by the configured APT metadata.",
    );
''',
    '''    let (
        security_page,
        security_entry,
        security_source,
        security_status,
        security_list,
    ) = build_security_page();
''',
)
replace_once(
    "        security_status,\n        security_list,\n        activity_entry,\n",
    "        security_entry,\n        security_source,\n        security_status,\n        security_list,\n        activity_entry,\n",
)
replace_once(
    "        activity_records: Rc::new(RefCell::new(Vec::new())),\n",
    "        activity_records: Rc::new(RefCell::new(Vec::new())),\n        security_updates: Rc::new(RefCell::new(Vec::new())),\n",
)
replace_once(
    '''    {
        let ui = ui.clone();
        ui.activity_entry
            .clone()
            .connect_changed(move |_| render_activity(&ui));
    }
''',
    '''    {
        let ui = ui.clone();
        ui.security_entry
            .clone()
            .connect_changed(move |_| render_security(&ui));
    }
    {
        let ui = ui.clone();
        ui.security_source
            .clone()
            .connect_selected_notify(move |_| render_security(&ui));
    }
    {
        let ui = ui.clone();
        ui.activity_entry
            .clone()
            .connect_changed(move |_| render_activity(&ui));
    }
''',
)

build_marker = "fn build_list_page(\n"
build_index = text.index(build_marker)
build_security = '''fn build_security_page() -> (
    gtk::Box,
    gtk::SearchEntry,
    gtk::DropDown,
    gtk::Label,
    gtk::ListBox,
) {
    let page = page_box();
    append_page_heading(
        &page,
        "Package security updates",
        "Review security-classified package updates reported by the configured APT metadata.",
    );

    let filters = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    let entry = gtk::SearchEntry::builder()
        .placeholder_text("Search security packages, versions or sources…")
        .hexpand(true)
        .build();
    let source = gtk::DropDown::from_strings(&[ALL_SECURITY_SOURCES]);
    source.set_tooltip_text(Some("Filter by security-update source"));
    source.set_sensitive(false);
    filters.append(&entry);
    filters.append(&source);
    page.append(&filters);

    let status = gtk::Label::new(Some("Loading package security metadata…"));
    status.set_xalign(0.0);
    status.set_wrap(true);
    page.append(&status);

    let list = gtk::ListBox::new();
    list.set_selection_mode(gtk::SelectionMode::None);
    list.add_css_class("boxed-list");
    let scrolled = gtk::ScrolledWindow::builder()
        .hexpand(true)
        .vexpand(true)
        .child(&list)
        .build();
    page.append(&scrolled);

    (page, entry, source, status, list)
}

'''
text = text[:build_index] + build_security + text[build_index:]

replace_once(
    '''    render_health(ui, &snapshot.health);
    render_installed(ui);
    render_updates(ui, &snapshot.updates);
    render_security(ui, &snapshot.updates);
''',
    '''    render_health(ui, &snapshot.health);
    render_installed(ui);
    *ui.security_updates.borrow_mut() = snapshot.updates.clone();
    populate_security_sources(ui);
    render_updates(ui, &snapshot.updates);
    render_security(ui);
''',
)

start = text.index("fn render_security(ui: &UiState, updates: &[UpdateRecord]) {")
end = text.index("fn start_activity_load", start)
new_security = '''fn populate_security_sources(ui: &UiState) {
    let updates = ui.security_updates.borrow();
    let summary = summarize_security(&updates);
    let mut values = vec![ALL_SECURITY_SOURCES];
    values.extend(summary.sources.iter().map(String::as_str));
    let model = gtk::StringList::new(&values);
    ui.security_source.set_model(Some(&model));
    ui.security_source.set_selected(0);
    ui.security_source.set_sensitive(values.len() > 1);
}

fn render_security(ui: &UiState) {
    clear_list(&ui.security_list);
    let updates = ui.security_updates.borrow();
    let summary = summarize_security(&updates);

    if summary.security_updates == 0 {
        ui.security_status.set_text(&summary.status_text());
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

    let query = ui.security_entry.text();
    let source = selected_text(&ui.security_source);
    let security = filter_security_updates(&updates, query.as_str(), &source);
    if security.is_empty() {
        ui.security_status
            .set_text("No security updates match the current search and source filter.");
        return;
    }

    ui.security_status.set_text(&format!(
        "{} Showing {} matching security updates.",
        summary.status_text(),
        security.len()
    ));
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
text = text[:start] + new_security + text[end:]

replace_once(
    '''    ui.security_status.set_text(message);
    clear_list(&ui.security_list);
''',
    '''    ui.security_status.set_text(message);
    ui.security_updates.borrow_mut().clear();
    clear_list(&ui.security_list);
''',
)

path.write_text(text)
