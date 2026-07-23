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
    "mod security_view;\n",
    "mod security_view;\nmod service_view;\n",
    path,
)
text = replace_once(
    text,
    "    AppRecord, CatalogPage, FeaturedCollection, PackageDetailRecord, PackageRecord, SystemHealth,\n    SystemSnapshot, TransactionEvent, TransactionRecord, UpdateRecord,\n",
    "    AppRecord, CatalogPage, FeaturedCollection, PackageDetailRecord, PackageRecord, ServiceRecord,\n    SystemHealth, SystemSnapshot, TransactionEvent, TransactionRecord, UpdateRecord,\n",
    path,
)
text = replace_once(
    text,
    "use security_view::{\n    ALL_SECURITY_SOURCES, filter_security_updates, security_filters_active, summarize_security,\n};\n",
    "use security_view::{\n    ALL_SECURITY_SOURCES, filter_security_updates, security_filters_active, summarize_security,\n};\nuse service_view::{service_state_css_class, service_state_label, summarize_services};\n",
    path,
)
text = replace_once(
    text,
    "    security_status: gtk::Label,\n    security_list: gtk::ListBox,\n",
    "    security_status: gtk::Label,\n    security_list: gtk::ListBox,\n    services_status: gtk::Label,\n    services_list: gtk::ListBox,\n",
    path,
)
text = replace_once(
    text,
    '''    add_placeholder_page(
        &stack,
        "services",
        "Services",
        "system-run-symbolic",
        "System services",
        "Inspect and control approved background services through the GenixBit system service.",
    );
''',
    '''    let (services_page, services_status, services_list) = build_list_page(
        "Approved system services",
        "Inspect read-only state for explicitly approved systemd services.",
    );
    add_widget_page(
        &stack,
        "services",
        "Services",
        "system-run-symbolic",
        &services_page,
    );
''',
    path,
)
text = replace_once(
    text,
    "        security_status,\n        security_list,\n        activity_entry,\n",
    "        security_status,\n        security_list,\n        services_status,\n        services_list,\n        activity_entry,\n",
    path,
)
text = replace_once(
    text,
    '''        refresh.connect_clicked(move |_| {
            start_snapshot_load(&ui);
            start_activity_load(&ui);
        });
''',
    '''        refresh.connect_clicked(move |_| {
            start_snapshot_load(&ui);
            start_activity_load(&ui);
            start_services_load(&ui);
        });
''',
    path,
)
text = replace_once(
    text,
    '''    start_snapshot_load(&ui);
    start_activity_load(&ui);
    start_featured_collections_load(&ui);
''',
    '''    start_snapshot_load(&ui);
    start_activity_load(&ui);
    start_services_load(&ui);
    start_featured_collections_load(&ui);
''',
    path,
)

marker = "fn start_activity_load(ui: &UiState) {\n"
index = text.index(marker)
service_functions = '''fn start_services_load(ui: &UiState) {
    ui.services_status
        .set_text("Loading approved system services…");
    clear_list(&ui.services_list);

    let (sender, receiver) = mpsc::channel();
    thread::spawn(move || {
        let result = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(anyhow::Error::from)
            .and_then(|runtime| runtime.block_on(client::approved_services()));
        let _ = sender.send(result);
    });

    let ui = ui.clone();
    glib::timeout_add_local(Duration::from_millis(100), move || {
        match receiver.try_recv() {
            Ok(Ok(services)) => {
                render_services(&ui, &services);
                glib::ControlFlow::Break
            }
            Ok(Err(error)) => {
                ui.services_status
                    .set_text(&format!("Unable to load approved services: {error}"));
                glib::ControlFlow::Break
            }
            Err(TryRecvError::Empty) => glib::ControlFlow::Continue,
            Err(TryRecvError::Disconnected) => {
                ui.services_status
                    .set_text("The approved-service worker stopped unexpectedly.");
                glib::ControlFlow::Break
            }
        }
    });
}

fn render_services(ui: &UiState, services: &[ServiceRecord]) {
    clear_list(&ui.services_list);
    ui.services_status
        .set_text(&summarize_services(services).status_text());

    if services.is_empty() {
        let row = adw::ActionRow::builder()
            .title("No approved services configured")
            .subtitle("Configure GENIXPKGD_APPROVED_SERVICES for read-only service inspection.")
            .build();
        ui.services_list.append(&row);
        return;
    }

    for service in services {
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
text = text[:index] + service_functions + text[index:]
text = replace_once(
    text,
    "    ui.security_updates.borrow_mut().clear();\n    clear_list(&ui.security_list);\n    ui.activity_summary\n",
    "    ui.security_updates.borrow_mut().clear();\n    clear_list(&ui.security_list);\n    ui.services_status.set_text(message);\n    clear_list(&ui.services_list);\n    ui.activity_summary\n",
    path,
)
path.write_text(text)

roadmap_path = Path("docs/ROADMAP.md")
roadmap = roadmap_path.read_text()
roadmap = replace_once(
    roadmap,
    "- [ ] Approved systemd services\n",
    "- [x] Approved systemd services\n",
    roadmap_path,
)
roadmap_path.write_text(roadmap)
