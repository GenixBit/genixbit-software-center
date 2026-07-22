mod client;

use std::{
    sync::mpsc::{self, TryRecvError},
    thread,
    time::Duration,
};

use adw::prelude::*;
use genixbit_package_model::{AppRecord, PackageRecord, SystemSnapshot, UpdateRecord};
use gtk::glib;

const APP_ID: &str = "com.genixbit.SoftwareCenter";
const MAX_VISIBLE_PACKAGES: usize = 750;

#[derive(Clone)]
struct UiState {
    dashboard: adw::StatusPage,
    installed_status: gtk::Label,
    installed_list: gtk::ListBox,
    updates_status: gtk::Label,
    updates_list: gtk::ListBox,
    discover_entry: gtk::SearchEntry,
    discover_status: gtk::Label,
    discover_list: gtk::ListBox,
}

fn main() -> glib::ExitCode {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "genixbit_software_center=info".into()),
        )
        .init();

    let application = adw::Application::builder().application_id(APP_ID).build();
    application.connect_activate(build_ui);
    application.run()
}

fn build_ui(application: &adw::Application) {
    let header = adw::HeaderBar::new();
    header.set_title_widget(Some(&adw::WindowTitle::new(
        "GenixBit Software Center",
        "Native software management for GenixBit OS",
    )));

    let stack = gtk::Stack::builder()
        .hexpand(true)
        .vexpand(true)
        .transition_type(gtk::StackTransitionType::Crossfade)
        .build();

    let dashboard = adw::StatusPage::builder()
        .icon_name("view-dashboard-symbolic")
        .title("Loading system metadata")
        .description("Connecting to the GenixBit package service…")
        .build();
    add_widget_page(
        &stack,
        "dashboard",
        "Dashboard",
        "view-dashboard-symbolic",
        &dashboard,
    );

    let (discover_page, discover_entry, discover_button, discover_status, discover_list) =
        build_discover_page();
    add_widget_page(
        &stack,
        "discover",
        "Discover",
        "system-software-install-symbolic",
        &discover_page,
    );

    let (installed_page, installed_status, installed_list) = build_list_page(
        "Installed software",
        "Applications, system packages, runtimes, drivers and GenixBit components installed on this device.",
    );
    add_widget_page(
        &stack,
        "installed",
        "Installed",
        "view-list-symbolic",
        &installed_page,
    );

    let (updates_page, updates_status, updates_list) = build_list_page(
        "Available updates",
        "Read-only update information from the configured APT repositories.",
    );
    add_widget_page(
        &stack,
        "updates",
        "Updates",
        "software-update-available-symbolic",
        &updates_page,
    );

    add_placeholder_page(
        &stack,
        "stacks",
        "Software Stacks",
        "view-grid-symbolic",
        "Software stacks",
        "Install capability-aware collections for AI, development, design and productivity.",
    );
    add_placeholder_page(
        &stack,
        "security",
        "Security",
        "security-high-symbolic",
        "Security status",
        "Package advisories, signature verification and repository trust will appear here.",
    );
    add_placeholder_page(
        &stack,
        "services",
        "Services",
        "system-run-symbolic",
        "System services",
        "Inspect and control approved background services through the GenixBit system service.",
    );
    add_placeholder_page(
        &stack,
        "profiles",
        "System Profiles",
        "document-save-symbolic",
        "Portable system profiles",
        "Export, compare and restore software configurations across GenixBit OS devices.",
    );

    let sidebar = gtk::StackSidebar::new();
    sidebar.set_stack(&stack);
    sidebar.set_width_request(230);

    let split = gtk::Paned::new(gtk::Orientation::Horizontal);
    split.set_start_child(Some(&sidebar));
    split.set_end_child(Some(&stack));
    split.set_position(230);
    split.set_shrink_start_child(false);
    split.set_resize_start_child(false);

    let root = gtk::Box::new(gtk::Orientation::Vertical, 0);
    root.append(&header);
    root.append(&split);

    let window = adw::ApplicationWindow::builder()
        .application(application)
        .title("GenixBit Software Center")
        .default_width(1180)
        .default_height(760)
        .content(&root)
        .build();

    let ui = UiState {
        dashboard,
        installed_status,
        installed_list,
        updates_status,
        updates_list,
        discover_entry,
        discover_status,
        discover_list,
    };

    {
        let ui = ui.clone();
        discover_button.connect_clicked(move |_| start_catalog_search(&ui));
    }
    {
        let ui = ui.clone();
        ui.discover_entry
            .clone()
            .connect_activate(move |_| start_catalog_search(&ui));
    }

    start_snapshot_load(&ui);
    window.present();
}

fn build_discover_page() -> (
    gtk::Box,
    gtk::SearchEntry,
    gtk::Button,
    gtk::Label,
    gtk::ListBox,
) {
    let page = gtk::Box::new(gtk::Orientation::Vertical, 12);
    page.set_margin_top(24);
    page.set_margin_bottom(24);
    page.set_margin_start(24);
    page.set_margin_end(24);

    let title = gtk::Label::new(Some("Discover software"));
    title.set_xalign(0.0);
    title.add_css_class("title-1");
    page.append(&title);

    let description = gtk::Label::new(Some(
        "Search verified application metadata from the local AppStream catalogue.",
    ));
    description.set_xalign(0.0);
    description.set_wrap(true);
    page.append(&description);

    let search_row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    let entry = gtk::SearchEntry::builder()
        .placeholder_text("Search applications, editors, AI tools…")
        .hexpand(true)
        .build();
    let button = gtk::Button::with_label("Search");
    search_row.append(&entry);
    search_row.append(&button);
    page.append(&search_row);

    let status = gtk::Label::new(Some("Enter a search term to browse AppStream."));
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

    (page, entry, button, status, list)
}

fn build_list_page(
    title_text: &str,
    description_text: &str,
) -> (gtk::Box, gtk::Label, gtk::ListBox) {
    let page = gtk::Box::new(gtk::Orientation::Vertical, 12);
    page.set_margin_top(24);
    page.set_margin_bottom(24);
    page.set_margin_start(24);
    page.set_margin_end(24);

    let title = gtk::Label::new(Some(title_text));
    title.set_xalign(0.0);
    title.add_css_class("title-1");
    page.append(&title);

    let description = gtk::Label::new(Some(description_text));
    description.set_xalign(0.0);
    description.set_wrap(true);
    page.append(&description);

    let status = gtk::Label::new(Some("Loading…"));
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

    (page, status, list)
}

fn start_snapshot_load(ui: &UiState) {
    ui.dashboard.set_title("Loading system metadata");
    ui.dashboard
        .set_description(Some("Connecting to the GenixBit package service…"));
    ui.installed_status.set_text("Loading installed packages…");
    ui.updates_status
        .set_text("Checking for available updates…");

    let (sender, receiver) = mpsc::channel();
    thread::spawn(move || {
        let result = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(anyhow::Error::from)
            .and_then(|runtime| runtime.block_on(client::load_snapshot()));
        let _ = sender.send(result);
    });

    let ui = ui.clone();
    glib::timeout_add_local(Duration::from_millis(100), move || {
        match receiver.try_recv() {
            Ok(Ok(snapshot)) => {
                render_snapshot(&ui, snapshot);
                glib::ControlFlow::Break
            }
            Ok(Err(error)) => {
                render_backend_error(&ui, &error.to_string());
                glib::ControlFlow::Break
            }
            Err(TryRecvError::Empty) => glib::ControlFlow::Continue,
            Err(TryRecvError::Disconnected) => {
                render_backend_error(&ui, "The package metadata worker stopped unexpectedly.");
                glib::ControlFlow::Break
            }
        }
    });
}

fn render_snapshot(ui: &UiState, snapshot: SystemSnapshot) {
    let installed_count = snapshot.installed.len();
    let update_count = snapshot.updates.len();
    let security_count = snapshot
        .updates
        .iter()
        .filter(|update| update.security)
        .count();

    ui.dashboard.set_title("System metadata ready");
    ui.dashboard.set_description(Some(&format!(
        "{installed_count} installed packages · {update_count} available updates · {security_count} security updates"
    )));

    render_installed(ui, &snapshot.installed);
    render_updates(ui, &snapshot.updates);
}

fn render_installed(ui: &UiState, packages: &[PackageRecord]) {
    clear_list(&ui.installed_list);
    let visible = packages.len().min(MAX_VISIBLE_PACKAGES);
    ui.installed_status.set_text(&format!(
        "{} packages installed. Showing {}.",
        packages.len(),
        visible
    ));

    for package in packages.iter().take(MAX_VISIBLE_PACKAGES) {
        let details = format!(
            "{} · {} · {} KiB{}",
            package.version,
            package.architecture,
            package.installed_size_kib,
            if package.section.is_empty() {
                String::new()
            } else {
                format!(" · {}", package.section)
            }
        );
        let row = adw::ActionRow::builder()
            .title(&package.name)
            .subtitle(if package.summary.is_empty() {
                &details
            } else {
                &package.summary
            })
            .build();
        let metadata = gtk::Label::new(Some(&details));
        metadata.add_css_class("dim-label");
        row.add_suffix(&metadata);
        if package.essential {
            let badge = gtk::Label::new(Some("Essential"));
            badge.add_css_class("accent");
            row.add_suffix(&badge);
        }
        ui.installed_list.append(&row);
    }
}

fn render_updates(ui: &UiState, updates: &[UpdateRecord]) {
    clear_list(&ui.updates_list);
    if updates.is_empty() {
        ui.updates_status
            .set_text("No package updates are currently reported by APT.");
        return;
    }

    let security_count = updates.iter().filter(|update| update.security).count();
    ui.updates_status.set_text(&format!(
        "{} updates available, including {} security updates.",
        updates.len(),
        security_count
    ));

    for update in updates {
        let subtitle = format!(
            "{} → {} · {} · {}",
            update.current_version, update.candidate_version, update.architecture, update.source
        );
        let row = adw::ActionRow::builder()
            .title(&update.name)
            .subtitle(&subtitle)
            .build();
        if update.security {
            let badge = gtk::Label::new(Some("Security"));
            badge.add_css_class("error");
            row.add_suffix(&badge);
        }
        ui.updates_list.append(&row);
    }
}

fn start_catalog_search(ui: &UiState) {
    let query = ui.discover_entry.text().trim().to_owned();
    if query.is_empty() {
        ui.discover_status
            .set_text("Enter a search term before searching.");
        return;
    }

    clear_list(&ui.discover_list);
    ui.discover_status
        .set_text(&format!("Searching AppStream for “{query}”…"));

    let (sender, receiver) = mpsc::channel();
    thread::spawn(move || {
        let result = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(anyhow::Error::from)
            .and_then(|runtime| runtime.block_on(client::search_catalog(&query)));
        let _ = sender.send(result);
    });

    let ui = ui.clone();
    glib::timeout_add_local(Duration::from_millis(100), move || {
        match receiver.try_recv() {
            Ok(Ok(apps)) => {
                render_catalog(&ui, &apps);
                glib::ControlFlow::Break
            }
            Ok(Err(error)) => {
                ui.discover_status
                    .set_text(&format!("AppStream search failed: {error}"));
                glib::ControlFlow::Break
            }
            Err(TryRecvError::Empty) => glib::ControlFlow::Continue,
            Err(TryRecvError::Disconnected) => {
                ui.discover_status
                    .set_text("The AppStream search worker stopped unexpectedly.");
                glib::ControlFlow::Break
            }
        }
    });
}

fn render_catalog(ui: &UiState, apps: &[AppRecord]) {
    clear_list(&ui.discover_list);
    ui.discover_status
        .set_text(&format!("{} AppStream results.", apps.len()));

    for app in apps {
        let subtitle = if app.summary.is_empty() {
            format!("{} · {}", app.package, app.kind)
        } else {
            app.summary.clone()
        };
        let row = adw::ActionRow::builder()
            .title(&app.name)
            .subtitle(&subtitle)
            .build();
        let package = gtk::Label::new(Some(&app.package));
        package.add_css_class("dim-label");
        row.add_suffix(&package);
        if app.installed {
            let badge = gtk::Label::new(Some("Installed"));
            badge.add_css_class("success");
            row.add_suffix(&badge);
        }
        ui.discover_list.append(&row);
    }
}

fn render_backend_error(ui: &UiState, message: &str) {
    ui.dashboard.set_title("Package service unavailable");
    ui.dashboard.set_description(Some(message));
    ui.installed_status.set_text(message);
    ui.updates_status.set_text(message);
}

fn clear_list(list: &gtk::ListBox) {
    while let Some(child) = list.first_child() {
        list.remove(&child);
    }
}

fn add_placeholder_page(
    stack: &gtk::Stack,
    name: &str,
    sidebar_title: &str,
    icon_name: &str,
    title: &str,
    description: &str,
) {
    let page = adw::StatusPage::builder()
        .icon_name(icon_name)
        .title(title)
        .description(description)
        .build();
    add_widget_page(stack, name, sidebar_title, icon_name, &page);
}

fn add_widget_page<W: IsA<gtk::Widget>>(
    stack: &gtk::Stack,
    name: &str,
    sidebar_title: &str,
    icon_name: &str,
    widget: &W,
) {
    let stack_page = stack.add_titled(widget, Some(name), sidebar_title);
    stack_page.set_icon_name(icon_name);
}
