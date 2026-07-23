from pathlib import Path


def replace_once(text: str, old: str, new: str, path: Path) -> str:
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"{path}: expected one marker, found {count}: {old[:120]!r}")
    return text.replace(old, new, 1)


path = Path("crates/software-center/src/main.rs")
text = path.read_text()
text = replace_once(text, "mod security_view;\n", "mod security_view;\nmod settings;\n", path)
text = replace_once(
    text,
    "use security_view::{\n    ALL_SECURITY_SOURCES, filter_security_updates, security_filters_active, summarize_security,\n};\n",
    "use security_view::{\n    ALL_SECURITY_SOURCES, filter_security_updates, security_filters_active, summarize_security,\n};\nuse settings::{AppSettings, load_settings, save_settings, settings_path};\n",
    path,
)
text = replace_once(
    text,
    "    discover_page_status: gtk::Label,\n    packages: Rc<RefCell<Vec<PackageRecord>>>,\n",
    "    discover_page_status: gtk::Label,\n    settings_offline: gtk::Switch,\n    settings_startup_refresh: gtk::Switch,\n    settings_status: gtk::Label,\n    settings: Rc<RefCell<AppSettings>>,\n    settings_path: Option<std::path::PathBuf>,\n    packages: Rc<RefCell<Vec<PackageRecord>>>,\n",
    path,
)
text = replace_once(
    text,
    "fn build_ui(application: &adw::Application) {\n    let header = adw::HeaderBar::new();\n",
    "fn build_ui(application: &adw::Application) {\n    let settings_path = settings_path();\n    let loaded_settings = load_settings(settings_path.as_deref());\n\n    let header = adw::HeaderBar::new();\n",
    path,
)
profiles_marker = '''    add_placeholder_page(
        &stack,
        "profiles",
        "System Profiles",
        "document-save-symbolic",
        "Portable system profiles",
        "Export, compare and restore software configurations across GenixBit OS devices.",
    );
'''
settings_page = profiles_marker + '''    let (settings_page, settings_offline, settings_startup_refresh, settings_status) =
        build_settings_page(&loaded_settings, settings_path.as_deref());
    add_widget_page(
        &stack,
        "settings",
        "Settings",
        "preferences-system-symbolic",
        &settings_page,
    );
'''
text = replace_once(text, profiles_marker, settings_page, path)
text = replace_once(
    text,
    "        discover_page_status,\n        packages: Rc::new(RefCell::new(Vec::new())),\n",
    "        discover_page_status,\n        settings_offline,\n        settings_startup_refresh,\n        settings_status,\n        settings: Rc::new(RefCell::new(loaded_settings)),\n        settings_path,\n        packages: Rc::new(RefCell::new(Vec::new())),\n",
    path,
)
text = replace_once(
    text,
    "            start_services_load(&ui);\n        });\n    }\n    {\n        let ui = ui.clone();\n        ui.stacks_entry\n",
    "            start_services_load(&ui);\n            start_curated_catalogue_load(&ui);\n        });\n    }\n    {\n        let ui = ui.clone();\n        ui.settings_offline.clone().connect_active_notify(move |control| {\n            ui.settings.borrow_mut().offline_mode = control.is_active();\n            persist_and_render_settings(&ui);\n        });\n    }\n    {\n        let ui = ui.clone();\n        ui.settings_startup_refresh\n            .clone()\n            .connect_active_notify(move |control| {\n                ui.settings.borrow_mut().refresh_on_startup = control.is_active();\n                persist_and_render_settings(&ui);\n            });\n    }\n    {\n        let ui = ui.clone();\n        ui.stacks_entry\n",
    path,
)
text = replace_once(
    text,
    "    start_snapshot_load(&ui);\n    start_activity_load(&ui);\n    start_services_load(&ui);\n    start_curated_catalogue_load(&ui);\n    window.present();\n",
    "    render_settings(&ui, None);\n    if ui.settings.borrow().refresh_on_startup {\n        start_snapshot_load(&ui);\n        start_activity_load(&ui);\n        start_services_load(&ui);\n        start_curated_catalogue_load(&ui);\n    } else {\n        render_startup_refresh_disabled(&ui);\n    }\n    window.present();\n",
    path,
)

build_settings = '''fn build_settings_page(
    settings: &AppSettings,
    path: Option<&std::path::Path>,
) -> (gtk::Box, gtk::Switch, gtk::Switch, gtk::Label) {
    let page = page_box();
    append_page_heading(
        &page,
        "Settings and offline controls",
        "Control user-level network policy and local metadata loading. Package and service changes remain disabled.",
    );

    let status = gtk::Label::new(None);
    status.set_xalign(0.0);
    status.set_wrap(true);
    page.append(&status);

    let list = gtk::ListBox::new();
    list.set_selection_mode(gtk::SelectionMode::None);
    list.add_css_class("boxed-list");

    let offline = gtk::Switch::builder()
        .active(settings.offline_mode)
        .valign(gtk::Align::Center)
        .build();
    let offline_row = adw::ActionRow::builder()
        .title("Offline mode")
        .subtitle("Block future external providers while allowing local dpkg, APT, AppStream, journal and systemd reads.")
        .build();
    offline_row.add_suffix(&offline);
    list.append(&offline_row);

    let startup_refresh = gtk::Switch::builder()
        .active(settings.refresh_on_startup)
        .valign(gtk::Align::Center)
        .build();
    let refresh_row = adw::ActionRow::builder()
        .title("Refresh local metadata on startup")
        .subtitle("Load package, update, service, Activity and AppStream metadata when the application opens.")
        .build();
    refresh_row.add_suffix(&startup_refresh);
    list.append(&refresh_row);

    let storage = adw::ActionRow::builder()
        .title("Settings storage")
        .subtitle(
            path.map(|value| value.display().to_string())
                .unwrap_or_else(|| "Session only: no HOME or XDG_CONFIG_HOME is available".to_owned()),
        )
        .build();
    list.append(&storage);

    let safety = adw::ActionRow::builder()
        .title("Mutation safety")
        .subtitle("Install, remove, upgrade, repository refresh and service-control operations remain fail-closed.")
        .build();
    let badge = gtk::Label::new(Some("Protected"));
    badge.add_css_class("success");
    safety.add_suffix(&badge);
    list.append(&safety);

    page.append(&list);
    (page, offline, startup_refresh, status)
}

'''
text = replace_once(text, "fn build_list_page(\n", build_settings + "fn build_list_page(\n", path)

helpers = '''fn persist_and_render_settings(ui: &UiState) {
    let settings = ui.settings.borrow().clone();
    let error = ui
        .settings_path
        .as_deref()
        .and_then(|path| save_settings(path, &settings).err())
        .map(|error| error.to_string());
    render_settings(ui, error.as_deref());
}

fn render_settings(ui: &UiState, error: Option<&str>) {
    let settings = ui.settings.borrow();
    let mut text = settings.policy_text();
    if let Some(path) = ui.settings_path.as_deref() {
        text.push_str(&format!(" Saved in {}.", path.display()));
    } else {
        text.push_str(" Changes apply to this session only.");
    }
    if let Some(error) = error {
        text.push_str(&format!(" Unable to save settings: {error}"));
    }
    ui.settings_status.set_text(&text);
}

fn render_startup_refresh_disabled(ui: &UiState) {
    let message = "Automatic startup refresh is disabled. Use the header refresh button to load local metadata.";
    ui.dashboard_status.set_text(message);
    ui.installed_status.set_text(message);
    ui.updates_status.set_text(message);
    ui.security_status.set_text(message);
    ui.services_status.set_text(message);
    ui.stacks_status.set_text(message);
    ui.activity_summary.set_text("Activity summary not loaded.");
    ui.activity_status.set_text(message);
    ui.discover_status.set_text(message);
    ui.discover_page_status.set_text("Refresh required");
    clear_list(&ui.discover_collections);
    ui.discover_collections.append(
        &adw::ActionRow::builder()
            .title("Curated catalogue not loaded")
            .subtitle(message)
            .build(),
    );
}

'''
text = replace_once(text, "fn render_backend_error(ui: &UiState, message: &str) {\n", helpers + "fn render_backend_error(ui: &UiState, message: &str) {\n", path)
path.write_text(text)

roadmap_path = Path("docs/ROADMAP.md")
roadmap = roadmap_path.read_text()
roadmap = replace_once(roadmap, "- [ ] Settings and offline controls\n", "- [x] Settings and offline controls\n", roadmap_path)
roadmap_path.write_text(roadmap)
