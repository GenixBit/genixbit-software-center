from pathlib import Path


def replace_once(text: str, old: str, new: str, path: Path) -> str:
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"{path}: expected one marker, found {count}: {old[:120]!r}")
    return text.replace(old, new, 1)


path = Path("crates/software-center/src/main.rs")
text = path.read_text()
text = replace_once(
    text,
    "use service_view::{service_state_css_class, service_state_label, summarize_services};\n",
    "use service_view::{\n    ALL_SERVICE_STATES, filter_services, service_filters_active, service_state_css_class,\n    service_state_label, summarize_services,\n};\n",
    path,
)
text = replace_once(
    text,
    "    services_status: gtk::Label,\n    services_list: gtk::ListBox,\n",
    "    services_entry: gtk::SearchEntry,\n    services_state: gtk::DropDown,\n    services_reset: gtk::Button,\n    services_status: gtk::Label,\n    services_list: gtk::ListBox,\n",
    path,
)
text = replace_once(
    text,
    "    security_updates: Rc<RefCell<Vec<UpdateRecord>>>,\n",
    "    security_updates: Rc<RefCell<Vec<UpdateRecord>>>,\n    services: Rc<RefCell<Vec<ServiceRecord>>>,\n",
    path,
)
text = replace_once(
    text,
    '''    let (services_page, services_status, services_list) = build_list_page(
        "Approved system services",
        "Inspect read-only state for explicitly approved systemd services.",
    );
''',
    '''    let (
        services_page,
        services_entry,
        services_state,
        services_reset,
        services_status,
        services_list,
    ) = build_services_page();
''',
    path,
)
text = replace_once(
    text,
    "        security_list,\n        services_status,\n        services_list,\n",
    "        security_list,\n        services_entry,\n        services_state,\n        services_reset,\n        services_status,\n        services_list,\n",
    path,
)
text = replace_once(
    text,
    "        security_updates: Rc::new(RefCell::new(Vec::new())),\n",
    "        security_updates: Rc::new(RefCell::new(Vec::new())),\n        services: Rc::new(RefCell::new(Vec::new())),\n",
    path,
)
text = replace_once(
    text,
    '''    {
        let ui = ui.clone();
        ui.security_entry
''',
    '''    {
        let ui = ui.clone();
        ui.services_entry
            .clone()
            .connect_changed(move |_| render_services(&ui));
    }
    {
        let ui = ui.clone();
        ui.services_state
            .clone()
            .connect_selected_notify(move |_| render_services(&ui));
    }
    {
        let ui = ui.clone();
        ui.services_reset.clone().connect_clicked(move |_| {
            ui.services_entry.set_text("");
            ui.services_state.set_selected(0);
            render_services(&ui);
        });
    }
    {
        let ui = ui.clone();
        ui.security_entry
''',
    path,
)

build_marker = "fn build_list_page(\n"
build_index = text.index(build_marker)
build_services = '''fn build_services_page() -> (
    gtk::Box,
    gtk::SearchEntry,
    gtk::DropDown,
    gtk::Button,
    gtk::Label,
    gtk::ListBox,
) {
    let page = page_box();
    append_page_heading(
        &page,
        "Approved system services",
        "Inspect read-only state for explicitly approved systemd services.",
    );

    let filters = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    let entry = gtk::SearchEntry::builder()
        .placeholder_text("Search approved services or metadata…")
        .hexpand(true)
        .build();
    let state = gtk::DropDown::from_strings(&[
        ALL_SERVICE_STATES,
        "Active",
        "Failed",
        "Inactive",
        "Unavailable",
        "Transitional",
    ]);
    state.set_tooltip_text(Some("Filter by service state"));
    let reset = gtk::Button::with_label("Clear filters");
    reset.set_tooltip_text(Some("Clear Services search and state filters"));
    reset.set_sensitive(false);
    filters.append(&entry);
    filters.append(&state);
    filters.append(&reset);
    page.append(&filters);

    let status = gtk::Label::new(Some("Loading approved system services…"));
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

    (page, entry, state, reset, status, list)
}

'''
text = text[:build_index] + build_services + text[build_index:]
text = replace_once(
    text,
    '''fn start_services_load(ui: &UiState) {
    ui.services_status
        .set_text("Loading approved system services…");
    clear_list(&ui.services_list);
''',
    '''fn start_services_load(ui: &UiState) {
    ui.services_status
        .set_text("Loading approved system services…");
    ui.services.borrow_mut().clear();
    clear_list(&ui.services_list);
''',
    path,
)
text = replace_once(
    text,
    '''            Ok(Ok(services)) => {
                render_services(&ui, &services);
                glib::ControlFlow::Break
            }
''',
    '''            Ok(Ok(services)) => {
                *ui.services.borrow_mut() = services;
                render_services(&ui);
                glib::ControlFlow::Break
            }
''',
    path,
)
start = text.index("fn render_services(ui: &UiState, services: &[ServiceRecord]) {")
end = text.index("fn start_activity_load", start)
render = '''fn render_services(ui: &UiState) {
    clear_list(&ui.services_list);
    let query = ui.services_entry.text();
    let state = selected_text(&ui.services_state);
    let services = ui.services.borrow();
    let summary = summarize_services(&services);
    ui.services_reset
        .set_sensitive(service_filters_active(query.as_str(), &state));

    if services.is_empty() {
        ui.services_status.set_text(&summary.status_text());
        let row = adw::ActionRow::builder()
            .title("No approved services configured")
            .subtitle("Configure GENIXPKGD_APPROVED_SERVICES for read-only service inspection.")
            .build();
        ui.services_list.append(&row);
        return;
    }

    let filtered = filter_services(&services, query.as_str(), &state);
    if filtered.is_empty() {
        ui.services_status
            .set_text("No approved services match the current search and state filter.");
        return;
    }

    ui.services_status.set_text(&format!(
        "{} Showing {} matching services.",
        summary.status_text(),
        filtered.len()
    ));
    for service in filtered {
        let description = if service.description.trim().is_empty() {
            "No description reported"
        } else {
            &service.description
        };
        let subtitle = format!(
            "{} · load {} · {} · unit file {}",
            description, service.load_state, service.sub_state, service.unit_file_state
        );
        let row = adw::ActionRow::builder()
            .title(&service.name)
            .subtitle(&subtitle)
            .build();
        let badge = gtk::Label::new(Some(service_state_label(service)));
        badge.add_css_class(service_state_css_class(service));
        row.add_suffix(&badge);
        ui.services_list.append(&row);
    }
}

'''
text = text[:start] + render + text[end:]
text = replace_once(
    text,
    "    ui.services_status.set_text(message);\n    clear_list(&ui.services_list);\n",
    "    ui.services_status.set_text(message);\n    ui.services.borrow_mut().clear();\n    clear_list(&ui.services_list);\n",
    path,
)
path.write_text(text)
