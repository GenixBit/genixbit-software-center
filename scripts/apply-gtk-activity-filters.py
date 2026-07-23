from pathlib import Path

path = Path("crates/software-center/src/main.rs")
text = path.read_text()


def replace_once(old: str, new: str) -> None:
    global text
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"expected one marker, found {count}: {old[:80]!r}")
    text = text.replace(old, new, 1)


replace_once("mod client;\n", "mod activity_filter;\nmod client;\n")
replace_once(
    "use adw::prelude::*;\n",
    "use activity_filter::{ALL_OPERATIONS, ALL_STATES, filter_records};\nuse adw::prelude::*;\n",
)
replace_once(
    "    activity_status: gtk::Label,\n    activity_list: gtk::ListBox,\n",
    "    activity_entry: gtk::SearchEntry,\n    activity_operation: gtk::DropDown,\n    activity_state: gtk::DropDown,\n    activity_status: gtk::Label,\n    activity_list: gtk::ListBox,\n",
)
replace_once(
    "    apps: Rc<RefCell<Vec<AppRecord>>>,\n",
    "    apps: Rc<RefCell<Vec<AppRecord>>>,\n    activity_records: Rc<RefCell<Vec<TransactionRecord>>>,\n",
)
replace_once(
    '''    let (activity_page, activity_status, activity_list) = build_list_page(
        "Transaction activity",
        "Review recent package previews, simulations, cancellations, failures and interrupted work.",
    );
''',
    '''    let (
        activity_page,
        activity_entry,
        activity_operation,
        activity_state,
        activity_status,
        activity_list,
    ) = build_activity_page();
''',
)
replace_once(
    "        updates_status,\n        updates_list,\n        activity_status,\n        activity_list,\n        discover_entry,\n",
    "        updates_status,\n        updates_list,\n        activity_entry,\n        activity_operation,\n        activity_state,\n        activity_status,\n        activity_list,\n        discover_entry,\n",
)
replace_once(
    "        apps: Rc::new(RefCell::new(Vec::new())),\n",
    "        apps: Rc::new(RefCell::new(Vec::new())),\n        activity_records: Rc::new(RefCell::new(Vec::new())),\n",
)
replace_once(
    '''    {
        let ui = ui.clone();
        discover_button.connect_clicked(move |_| start_catalog_search(&ui));
    }
''',
    '''    {
        let ui = ui.clone();
        ui.activity_entry
            .clone()
            .connect_changed(move |_| render_activity(&ui));
    }
    {
        let ui = ui.clone();
        ui.activity_operation
            .clone()
            .connect_selected_notify(move |_| render_activity(&ui));
    }
    {
        let ui = ui.clone();
        ui.activity_state
            .clone()
            .connect_selected_notify(move |_| render_activity(&ui));
    }
    {
        let ui = ui.clone();
        discover_button.connect_clicked(move |_| start_catalog_search(&ui));
    }
''',
)

build_marker = "fn build_list_page(\n"
build_index = text.index(build_marker)
build_activity = '''fn build_activity_page() -> (
    gtk::Box,
    gtk::SearchEntry,
    gtk::DropDown,
    gtk::DropDown,
    gtk::Label,
    gtk::ListBox,
) {
    let page = page_box();
    append_page_heading(
        &page,
        "Transaction activity",
        "Review recent package previews, simulations, cancellations, failures and interrupted work.",
    );

    let filters = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    let entry = gtk::SearchEntry::builder()
        .placeholder_text("Search package, message or transaction ID…")
        .hexpand(true)
        .build();
    let operation = gtk::DropDown::from_strings(&[
        ALL_OPERATIONS,
        "Install",
        "Remove",
        "Upgrade",
    ]);
    operation.set_tooltip_text(Some("Filter by transaction operation"));
    let state = gtk::DropDown::from_strings(&[
        ALL_STATES,
        "Queued",
        "Running",
        "Completed",
        "Failed",
        "Cancelled",
        "Interrupted",
    ]);
    state.set_tooltip_text(Some("Filter by latest transaction state"));
    filters.append(&entry);
    filters.append(&operation);
    filters.append(&state);
    page.append(&filters);

    let status = gtk::Label::new(Some("Loading recent transaction activity…"));
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

    (page, entry, operation, state, status, list)
}

'''
text = text[:build_index] + build_activity + text[build_index:]

replace_once(
    '''fn start_activity_load(ui: &UiState) {
    ui.activity_status
        .set_text("Loading recent package transaction activity…");
    clear_list(&ui.activity_list);
''',
    '''fn start_activity_load(ui: &UiState) {
    ui.activity_status
        .set_text("Loading recent package transaction activity…");
    ui.activity_records.borrow_mut().clear();
    clear_list(&ui.activity_list);
''',
)
replace_once(
    '''            Ok(Ok(records)) => {
                render_activity(&ui, &records);
                glib::ControlFlow::Break
            }
''',
    '''            Ok(Ok(records)) => {
                *ui.activity_records.borrow_mut() = records;
                render_activity(&ui);
                glib::ControlFlow::Break
            }
''',
)

start = text.index("fn render_activity(ui: &UiState, records: &[TransactionRecord]) {")
end = text.index("fn start_transaction_details", start)
new_render = '''fn render_activity(ui: &UiState) {
    clear_list(&ui.activity_list);
    let query = ui.activity_entry.text();
    let operation = selected_text(&ui.activity_operation);
    let state = selected_text(&ui.activity_state);
    let records = ui.activity_records.borrow();

    if records.is_empty() {
        ui.activity_status
            .set_text("No package transaction activity has been recorded.");
        return;
    }

    let filtered = filter_records(&records, query.as_str(), &operation, &state);
    if filtered.is_empty() {
        ui.activity_status
            .set_text("No transactions match the current Activity filters.");
        return;
    }

    ui.activity_status.set_text(&format!(
        "Showing {} of {} recent transactions. Package execution remains simulation-only.",
        filtered.len(),
        records.len()
    ));
    for record in filtered {
        let row = adw::ActionRow::builder()
            .title(activity_title(record))
            .subtitle(format!("Transaction #{} · {}", record.id, record.message))
            .activatable(true)
            .build();

        let badge = gtk::Label::new(Some(activity_state_label(&record.state)));
        badge.add_css_class(activity_state_css_class(&record.state));
        row.add_suffix(&badge);

        if let Some(fraction) = activity_progress_fraction(record.progress_basis_points) {
            let progress = gtk::ProgressBar::new();
            progress.set_fraction(fraction);
            progress.set_width_request(120);
            progress.set_tooltip_text(Some(&format!(
                "{}% complete",
                record.progress_basis_points / 100
            )));
            row.add_suffix(&progress);
        }
        let callback_ui = ui.clone();
        let callback_record = record.clone();
        row.connect_activated(move |_| start_transaction_details(&callback_ui, &callback_record));
        ui.activity_list.append(&row);
    }
}

'''
text = text[:start] + new_render + text[end:]

replace_once(
    '''    ui.installed_status.set_text(message);
    ui.updates_status.set_text(message);
    ui.activity_status.set_text(message);
''',
    '''    ui.installed_status.set_text(message);
    ui.updates_status.set_text(message);
    ui.activity_status.set_text(message);
    ui.activity_records.borrow_mut().clear();
    clear_list(&ui.activity_list);
''',
)

path.write_text(text)
