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
    "use activity_filter::{ALL_OPERATIONS, ALL_STATES, filter_records};",
    "use activity_filter::{ALL_OPERATIONS, ALL_STATES, filter_records, summarize_records};",
)
replace_once(
    "    activity_state: gtk::DropDown,\n    activity_status: gtk::Label,\n",
    "    activity_state: gtk::DropDown,\n    activity_reset: gtk::Button,\n    activity_summary: gtk::Label,\n    activity_status: gtk::Label,\n",
)
replace_once(
    '''        activity_operation,
        activity_state,
        activity_status,
        activity_list,
    ) = build_activity_page();
''',
    '''        activity_operation,
        activity_state,
        activity_reset,
        activity_summary,
        activity_status,
        activity_list,
    ) = build_activity_page();
''',
)
replace_once(
    '''        activity_entry,
        activity_operation,
        activity_state,
        activity_status,
        activity_list,
        discover_entry,
''',
    '''        activity_entry,
        activity_operation,
        activity_state,
        activity_reset,
        activity_summary,
        activity_status,
        activity_list,
        discover_entry,
''',
)
replace_once(
    '''    {
        let ui = ui.clone();
        ui.activity_state
            .clone()
            .connect_selected_notify(move |_| render_activity(&ui));
    }
''',
    '''    {
        let ui = ui.clone();
        ui.activity_state
            .clone()
            .connect_selected_notify(move |_| render_activity(&ui));
    }
    {
        let ui = ui.clone();
        ui.activity_reset.clone().connect_clicked(move |_| {
            ui.activity_entry.set_text("");
            ui.activity_operation.set_selected(0);
            ui.activity_state.set_selected(0);
            render_activity(&ui);
        });
    }
''',
)

start = text.index("fn build_activity_page() -> (")
end = text.index("fn build_list_page", start)
new_build = '''fn build_activity_page() -> (
    gtk::Box,
    gtk::SearchEntry,
    gtk::DropDown,
    gtk::DropDown,
    gtk::Button,
    gtk::Label,
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
    let operation = gtk::DropDown::from_strings(&[ALL_OPERATIONS, "Install", "Remove", "Upgrade"]);
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
    let reset = gtk::Button::with_label("Clear filters");
    reset.set_tooltip_text(Some("Clear all Activity filters"));
    reset.set_sensitive(false);
    filters.append(&entry);
    filters.append(&operation);
    filters.append(&state);
    filters.append(&reset);
    page.append(&filters);

    let summary = gtk::Label::new(Some("Loading activity summary…"));
    summary.set_xalign(0.0);
    summary.set_wrap(true);
    summary.add_css_class("dim-label");
    page.append(&summary);

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

    (page, entry, operation, state, reset, summary, status, list)
}

'''
text = text[:start] + new_build + text[end:]

replace_once(
    '''fn start_activity_load(ui: &UiState) {
    ui.activity_status
        .set_text("Loading recent package transaction activity…");
''',
    '''fn start_activity_load(ui: &UiState) {
    ui.activity_summary.set_text("Loading activity summary…");
    ui.activity_status
        .set_text("Loading recent package transaction activity…");
''',
)
replace_once(
    '''            Ok(Err(error)) => {
                ui.activity_status
                    .set_text(&format!("Unable to load transaction activity: {error}"));
                glib::ControlFlow::Break
            }
''',
    '''            Ok(Err(error)) => {
                ui.activity_summary.set_text("Activity summary unavailable.");
                ui.activity_status
                    .set_text(&format!("Unable to load transaction activity: {error}"));
                glib::ControlFlow::Break
            }
''',
)
replace_once(
    '''            Err(TryRecvError::Disconnected) => {
                ui.activity_status
                    .set_text("The transaction activity worker stopped unexpectedly.");
                glib::ControlFlow::Break
            }
''',
    '''            Err(TryRecvError::Disconnected) => {
                ui.activity_summary.set_text("Activity summary unavailable.");
                ui.activity_status
                    .set_text("The transaction activity worker stopped unexpectedly.");
                glib::ControlFlow::Break
            }
''',
)
replace_once(
    '''    let state = selected_text(&ui.activity_state);
    let records = ui.activity_records.borrow();

    if records.is_empty() {
''',
    '''    let state = selected_text(&ui.activity_state);
    let records = ui.activity_records.borrow();
    ui.activity_reset.set_sensitive(
        !query.trim().is_empty()
            || ui.activity_operation.selected() != 0
            || ui.activity_state.selected() != 0,
    );

    if records.is_empty() {
        ui.activity_summary.set_text("No recorded transactions.");
''',
)
replace_once(
    '''    let filtered = filter_records(&records, query.as_str(), &operation, &state);
''',
    '''    ui.activity_summary
        .set_text(&summarize_records(&records).status_text());
    let filtered = filter_records(&records, query.as_str(), &operation, &state);
''',
)
replace_once(
    '''    ui.activity_status.set_text(message);
    ui.activity_records.borrow_mut().clear();
''',
    '''    ui.activity_summary.set_text("Activity summary unavailable.");
    ui.activity_status.set_text(message);
    ui.activity_records.borrow_mut().clear();
''',
)

path.write_text(text)
