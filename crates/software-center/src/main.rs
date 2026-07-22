mod client;

use std::{
    cell::{Cell, RefCell},
    collections::BTreeSet,
    rc::Rc,
    sync::mpsc::{self, TryRecvError},
    thread,
    time::Duration,
};

use adw::prelude::*;
use genixbit_package_model::{
    AppRecord, CatalogPage, FeaturedCollection, PackageDetailRecord, PackageRecord, SystemHealth,
    SystemSnapshot, UpdateRecord,
};
use gtk::glib;

const APP_ID: &str = "com.genixbit.SoftwareCenter";
const CATALOG_PAGE_SIZE: u64 = 25;
const INSTALLED_PAGE_SIZE: usize = 100;

#[derive(Clone)]
struct UiState {
    window: adw::ApplicationWindow,
    dashboard_status: gtk::Label,
    health_list: gtk::ListBox,
    installed_entry: gtk::SearchEntry,
    installed_section: gtk::DropDown,
    installed_status: gtk::Label,
    installed_list: gtk::ListBox,
    installed_previous: gtk::Button,
    installed_next: gtk::Button,
    installed_page_status: gtk::Label,
    updates_status: gtk::Label,
    updates_list: gtk::ListBox,
    discover_entry: gtk::SearchEntry,
    discover_category: gtk::DropDown,
    discover_status: gtk::Label,
    discover_list: gtk::ListBox,
    discover_collections: gtk::ListBox,
    discover_previous: gtk::Button,
    discover_next: gtk::Button,
    discover_page_status: gtk::Label,
    packages: Rc<RefCell<Vec<PackageRecord>>>,
    apps: Rc<RefCell<Vec<AppRecord>>>,
    installed_offset: Rc<Cell<usize>>,
    catalog_query: Rc<RefCell<String>>,
    catalog_offset: Rc<Cell<u64>>,
    catalog_total: Rc<Cell<u64>>,
    catalog_has_more: Rc<Cell<bool>>,
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

    let refresh = gtk::Button::builder()
        .icon_name("view-refresh-symbolic")
        .tooltip_text("Refresh package metadata")
        .build();
    header.pack_end(&refresh);

    let stack = gtk::Stack::builder()
        .hexpand(true)
        .vexpand(true)
        .transition_type(gtk::StackTransitionType::Crossfade)
        .build();

    let (dashboard_page, dashboard_status, health_list) = build_dashboard_page();
    add_widget_page(
        &stack,
        "dashboard",
        "Dashboard",
        "view-dashboard-symbolic",
        &dashboard_page,
    );

    let (
        discover_page,
        discover_entry,
        discover_button,
        discover_category,
        discover_status,
        discover_list,
        discover_collections,
        discover_previous,
        discover_next,
        discover_page_status,
    ) = build_discover_page();
    add_widget_page(
        &stack,
        "discover",
        "Discover",
        "system-software-install-symbolic",
        &discover_page,
    );

    let (
        installed_page,
        installed_entry,
        installed_section,
        installed_status,
        installed_list,
        installed_previous,
        installed_next,
        installed_page_status,
    ) = build_installed_page();
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
        window: window.clone(),
        dashboard_status,
        health_list,
        installed_entry,
        installed_section,
        installed_status,
        installed_list,
        installed_previous,
        installed_next,
        installed_page_status,
        updates_status,
        updates_list,
        discover_entry,
        discover_category,
        discover_status,
        discover_list,
        discover_collections,
        discover_previous,
        discover_next,
        discover_page_status,
        packages: Rc::new(RefCell::new(Vec::new())),
        apps: Rc::new(RefCell::new(Vec::new())),
        installed_offset: Rc::new(Cell::new(0)),
        catalog_query: Rc::new(RefCell::new(String::new())),
        catalog_offset: Rc::new(Cell::new(0)),
        catalog_total: Rc::new(Cell::new(0)),
        catalog_has_more: Rc::new(Cell::new(false)),
    };

    {
        let ui = ui.clone();
        refresh.connect_clicked(move |_| start_snapshot_load(&ui));
    }
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
    {
        let ui = ui.clone();
        ui.installed_entry.clone().connect_changed(move |_| {
            ui.installed_offset.set(0);
            render_installed(&ui);
        });
    }
    {
        let ui = ui.clone();
        ui.installed_section
            .clone()
            .connect_selected_notify(move |_| {
                ui.installed_offset.set(0);
                render_installed(&ui);
            });
    }
    {
        let ui = ui.clone();
        ui.installed_previous.clone().connect_clicked(move |_| {
            let offset = ui.installed_offset.get();
            ui.installed_offset
                .set(offset.saturating_sub(INSTALLED_PAGE_SIZE));
            render_installed(&ui);
        });
    }
    {
        let ui = ui.clone();
        ui.installed_next.clone().connect_clicked(move |_| {
            ui.installed_offset.set(
                ui.installed_offset
                    .get()
                    .saturating_add(INSTALLED_PAGE_SIZE),
            );
            render_installed(&ui);
        });
    }
    {
        let ui = ui.clone();
        ui.discover_category
            .clone()
            .connect_selected_notify(move |_| render_catalog(&ui));
    }
    {
        let ui = ui.clone();
        ui.discover_previous.clone().connect_clicked(move |_| {
            let query = ui.catalog_query.borrow().clone();
            if !query.is_empty() {
                let offset = ui.catalog_offset.get().saturating_sub(CATALOG_PAGE_SIZE);
                start_catalog_page(&ui, query, offset);
            }
        });
    }
    {
        let ui = ui.clone();
        ui.discover_next.clone().connect_clicked(move |_| {
            let query = ui.catalog_query.borrow().clone();
            if !query.is_empty() && ui.catalog_has_more.get() {
                let offset = ui.catalog_offset.get().saturating_add(CATALOG_PAGE_SIZE);
                start_catalog_page(&ui, query, offset);
            }
        });
    }

    start_snapshot_load(&ui);
    start_featured_collections_load(&ui);
    window.present();
}

fn build_dashboard_page() -> (gtk::Box, gtk::Label, gtk::ListBox) {
    let page = page_box();
    append_page_heading(
        &page,
        "System health",
        "A read-only overview of package integrity, updates, storage and metadata services.",
    );

    let status = gtk::Label::new(Some("Loading system metadata…"));
    status.set_xalign(0.0);
    status.set_wrap(true);
    page.append(&status);

    let list = gtk::ListBox::new();
    list.set_selection_mode(gtk::SelectionMode::None);
    list.add_css_class("boxed-list");
    page.append(&list);

    (page, status, list)
}

fn build_installed_page() -> (
    gtk::Box,
    gtk::SearchEntry,
    gtk::DropDown,
    gtk::Label,
    gtk::ListBox,
    gtk::Button,
    gtk::Button,
    gtk::Label,
) {
    let page = page_box();
    append_page_heading(
        &page,
        "Installed software",
        "Search installed applications, system packages, runtimes, drivers and GenixBit components.",
    );

    let filters = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    let entry = gtk::SearchEntry::builder()
        .placeholder_text("Filter by package name or description…")
        .hexpand(true)
        .build();
    let section = gtk::DropDown::from_strings(&["All sections"]);
    section.set_tooltip_text(Some("Filter by Debian package section"));
    filters.append(&entry);
    filters.append(&section);
    page.append(&filters);

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

    let navigation = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    let previous = gtk::Button::with_label("Previous");
    previous.set_sensitive(false);
    let page_status = gtk::Label::new(Some("Page 1"));
    page_status.set_hexpand(true);
    page_status.set_xalign(0.5);
    let next = gtk::Button::with_label("Next");
    next.set_sensitive(false);
    navigation.append(&previous);
    navigation.append(&page_status);
    navigation.append(&next);
    page.append(&navigation);

    (
        page,
        entry,
        section,
        status,
        list,
        previous,
        next,
        page_status,
    )
}

fn build_discover_page() -> (
    gtk::Box,
    gtk::SearchEntry,
    gtk::Button,
    gtk::DropDown,
    gtk::Label,
    gtk::ListBox,
    gtk::ListBox,
    gtk::Button,
    gtk::Button,
    gtk::Label,
) {
    let page = page_box();
    append_page_heading(
        &page,
        "Discover software",
        "Browse curated collections or search verified application metadata from the local AppStream catalogue.",
    );

    let collections_title = gtk::Label::new(Some("Featured collections"));
    collections_title.set_xalign(0.0);
    collections_title.add_css_class("title-3");
    page.append(&collections_title);

    let collections = gtk::ListBox::new();
    collections.set_selection_mode(gtk::SelectionMode::None);
    collections.add_css_class("boxed-list");
    let loading = adw::ActionRow::builder()
        .title("Loading featured collections…")
        .subtitle("Reading curated AppStream collection metadata")
        .build();
    collections.append(&loading);
    page.append(&collections);

    let search_row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    let entry = gtk::SearchEntry::builder()
        .placeholder_text("Search applications, editors, AI tools…")
        .hexpand(true)
        .build();
    let button = gtk::Button::with_label("Search");
    search_row.append(&entry);
    search_row.append(&button);
    page.append(&search_row);

    let category = gtk::DropDown::from_strings(&["All categories"]);
    category.set_tooltip_text(Some("Filter the current page by AppStream category"));
    category.set_sensitive(false);
    page.append(&category);

    let status = gtk::Label::new(Some("Choose a collection or enter a search term."));
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

    let navigation = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    let previous = gtk::Button::with_label("Previous");
    previous.set_sensitive(false);
    let page_status = gtk::Label::new(Some("No catalogue page loaded"));
    page_status.set_hexpand(true);
    page_status.set_xalign(0.5);
    let next = gtk::Button::with_label("Next");
    next.set_sensitive(false);
    navigation.append(&previous);
    navigation.append(&page_status);
    navigation.append(&next);
    page.append(&navigation);

    (
        page,
        entry,
        button,
        category,
        status,
        list,
        collections,
        previous,
        next,
        page_status,
    )
}

fn build_list_page(
    title_text: &str,
    description_text: &str,
) -> (gtk::Box, gtk::Label, gtk::ListBox) {
    let page = page_box();
    append_page_heading(&page, title_text, description_text);

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

fn page_box() -> gtk::Box {
    let page = gtk::Box::new(gtk::Orientation::Vertical, 12);
    page.set_margin_top(24);
    page.set_margin_bottom(24);
    page.set_margin_start(24);
    page.set_margin_end(24);
    page
}

fn append_page_heading(page: &gtk::Box, title_text: &str, description_text: &str) {
    let title = gtk::Label::new(Some(title_text));
    title.set_xalign(0.0);
    title.add_css_class("title-1");
    page.append(&title);

    let description = gtk::Label::new(Some(description_text));
    description.set_xalign(0.0);
    description.set_wrap(true);
    page.append(&description);
}

fn start_snapshot_load(ui: &UiState) {
    ui.dashboard_status
        .set_text("Connecting to the GenixBit package service…");
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
    *ui.packages.borrow_mut() = snapshot.installed;
    populate_installed_sections(ui);
    render_health(ui, &snapshot.health);
    render_installed(ui);
    render_updates(ui, &snapshot.updates);
}

fn render_health(ui: &UiState, health: &SystemHealth) {
    clear_list(&ui.health_list);
    let state = if health.broken_package_count == 0 {
        "Package database reports no interrupted states"
    } else {
        "Package database requires attention"
    };
    ui.dashboard_status.set_text(&format!(
        "{state}. {} installed packages, {} available updates and {} security updates.",
        health.installed_count, health.update_count, health.security_update_count
    ));

    append_health_row(
        &ui.health_list,
        "Package database",
        if health.dpkg_status_readable {
            "Readable"
        } else {
            "Unavailable"
        },
        if health.broken_package_count == 0 {
            "Healthy"
        } else {
            "Needs repair"
        },
    );
    append_health_row(
        &ui.health_list,
        "Installed footprint",
        &format!(
            "{} across {} packages",
            format_size(health.installed_size_kib),
            health.installed_count
        ),
        &format!("{} essential", health.essential_count),
    );
    append_health_row(
        &ui.health_list,
        "APT metadata",
        if health.apt_available {
            "Available"
        } else {
            "Unavailable or stale"
        },
        &format!("{} updates", health.update_count),
    );
    append_health_row(
        &ui.health_list,
        "AppStream catalogue",
        if health.appstream_available {
            "Available"
        } else {
            "Unavailable"
        },
        "Read only",
    );
    append_health_row(
        &ui.health_list,
        "Restart status",
        if health.reboot_required {
            "A system restart is required"
        } else {
            "No restart marker is present"
        },
        if health.reboot_required {
            "Restart"
        } else {
            "Ready"
        },
    );
    let update_origins = if health.update_sources.is_empty() {
        "No update repositories currently represented".to_owned()
    } else {
        health.update_sources.join(", ")
    };
    append_health_row(
        &ui.health_list,
        "Update origins",
        &update_origins,
        &format!("{} sources", health.update_sources.len()),
    );
}

fn append_health_row(list: &gtk::ListBox, title: &str, subtitle: &str, badge_text: &str) {
    let row = adw::ActionRow::builder()
        .title(title)
        .subtitle(subtitle)
        .build();
    let badge = gtk::Label::new(Some(badge_text));
    badge.add_css_class("dim-label");
    row.add_suffix(&badge);
    list.append(&row);
}

fn populate_installed_sections(ui: &UiState) {
    let mut sections = ui
        .packages
        .borrow()
        .iter()
        .map(|package| package.section.trim())
        .filter(|section| !section.is_empty())
        .map(ToOwned::to_owned)
        .collect::<BTreeSet<_>>();
    sections.insert("All sections".to_owned());
    let values = sections.iter().map(String::as_str).collect::<Vec<_>>();
    let model = gtk::StringList::new(&values);
    ui.installed_section.set_model(Some(&model));
    ui.installed_section.set_selected(0);
}

fn render_installed(ui: &UiState) {
    clear_list(&ui.installed_list);
    let query = ui.installed_entry.text().trim().to_ascii_lowercase();
    let section = selected_text(&ui.installed_section);
    let packages = ui.packages.borrow();
    let filtered = packages
        .iter()
        .filter(|package| {
            let query_matches = query.is_empty()
                || package.name.to_ascii_lowercase().contains(&query)
                || package.summary.to_ascii_lowercase().contains(&query);
            let section_matches =
                section.is_empty() || section == "All sections" || package.section == section;
            query_matches && section_matches
        })
        .collect::<Vec<_>>();

    let total = filtered.len();
    let mut offset = ui.installed_offset.get();
    if total == 0 {
        offset = 0;
    } else if offset >= total {
        offset = ((total - 1) / INSTALLED_PAGE_SIZE) * INSTALLED_PAGE_SIZE;
    }
    ui.installed_offset.set(offset);
    let end = offset.saturating_add(INSTALLED_PAGE_SIZE).min(total);

    if total == 0 {
        ui.installed_status
            .set_text("No installed packages match the current filters.");
        ui.installed_page_status.set_text("No results");
    } else {
        ui.installed_status.set_text(&format!(
            "{} matching packages. Showing {}–{} of {} installed.",
            total,
            offset + 1,
            end,
            packages.len()
        ));
        ui.installed_page_status.set_text(&format!(
            "Page {} of {}",
            offset / INSTALLED_PAGE_SIZE + 1,
            total.div_ceil(INSTALLED_PAGE_SIZE)
        ));
    }
    ui.installed_previous.set_sensitive(offset > 0);
    ui.installed_next.set_sensitive(end < total);

    for package in filtered[offset..end].iter().copied() {
        let details = format!(
            "{} · {} · {}{}",
            package.version,
            package.architecture,
            format_size(package.installed_size_kib),
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
            .activatable(true)
            .build();
        let metadata = gtk::Label::new(Some(&details));
        metadata.add_css_class("dim-label");
        row.add_suffix(&metadata);
        if package.essential {
            let badge = gtk::Label::new(Some("Essential"));
            badge.add_css_class("accent");
            row.add_suffix(&badge);
        }
        let callback_ui = ui.clone();
        let package_name = package.name.clone();
        row.connect_activated(move |_| start_package_details(&callback_ui, &package_name));
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
            .activatable(true)
            .build();
        if update.security {
            let badge = gtk::Label::new(Some("Security"));
            badge.add_css_class("error");
            row.add_suffix(&badge);
        }
        let callback_ui = ui.clone();
        let package_name = update.name.clone();
        row.connect_activated(move |_| start_package_details(&callback_ui, &package_name));
        ui.updates_list.append(&row);
    }
}

fn start_featured_collections_load(ui: &UiState) {
    let (sender, receiver) = mpsc::channel();
    thread::spawn(move || {
        let result = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(anyhow::Error::from)
            .and_then(|runtime| runtime.block_on(client::featured_collections()));
        let _ = sender.send(result);
    });

    let ui = ui.clone();
    glib::timeout_add_local(Duration::from_millis(100), move || {
        match receiver.try_recv() {
            Ok(Ok(collections)) => {
                render_featured_collections(&ui, &collections);
                glib::ControlFlow::Break
            }
            Ok(Err(error)) => {
                clear_list(&ui.discover_collections);
                let row = adw::ActionRow::builder()
                    .title("Featured collections unavailable")
                    .subtitle(error.to_string())
                    .build();
                ui.discover_collections.append(&row);
                glib::ControlFlow::Break
            }
            Err(TryRecvError::Empty) => glib::ControlFlow::Continue,
            Err(TryRecvError::Disconnected) => {
                clear_list(&ui.discover_collections);
                let row = adw::ActionRow::builder()
                    .title("Featured collection worker stopped")
                    .subtitle("Restart the software center and try again")
                    .build();
                ui.discover_collections.append(&row);
                glib::ControlFlow::Break
            }
        }
    });
}

fn render_featured_collections(ui: &UiState, collections: &[FeaturedCollection]) {
    clear_list(&ui.discover_collections);
    for collection in collections {
        let row = adw::ActionRow::builder()
            .title(&collection.title)
            .subtitle(&collection.description)
            .activatable(true)
            .build();
        let image = gtk::Image::from_icon_name(&collection.icon);
        row.add_prefix(&image);
        if !collection.category.is_empty() {
            let category = gtk::Label::new(Some(&collection.category));
            category.add_css_class("dim-label");
            row.add_suffix(&category);
        }
        let callback_ui = ui.clone();
        let query = collection.query.clone();
        row.connect_activated(move |_| {
            callback_ui.discover_entry.set_text(&query);
            start_catalog_page(&callback_ui, query.clone(), 0);
        });
        ui.discover_collections.append(&row);
    }
}

fn start_featured_collections_load(ui: &UiState) {
    let (sender, receiver) = mpsc::channel();
    thread::spawn(move || {
        let result = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(anyhow::Error::from)
            .and_then(|runtime| runtime.block_on(client::featured_collections()));
        let _ = sender.send(result);
    });

    let ui = ui.clone();
    glib::timeout_add_local(Duration::from_millis(100), move || {
        match receiver.try_recv() {
            Ok(Ok(collections)) => {
                render_featured_collections(&ui, &collections);
                glib::ControlFlow::Break
            }
            Ok(Err(error)) => {
                clear_list(&ui.discover_collections);
                let row = adw::ActionRow::builder()
                    .title("Featured collections unavailable")
                    .subtitle(error.to_string())
                    .build();
                ui.discover_collections.append(&row);
                glib::ControlFlow::Break
            }
            Err(TryRecvError::Empty) => glib::ControlFlow::Continue,
            Err(TryRecvError::Disconnected) => {
                clear_list(&ui.discover_collections);
                let row = adw::ActionRow::builder()
                    .title("Featured collection worker stopped")
                    .subtitle("Restart the software center and try again")
                    .build();
                ui.discover_collections.append(&row);
                glib::ControlFlow::Break
            }
        }
    });
}

fn render_featured_collections(ui: &UiState, collections: &[FeaturedCollection]) {
    clear_list(&ui.discover_collections);
    for collection in collections {
        let row = adw::ActionRow::builder()
            .title(&collection.title)
            .subtitle(&collection.description)
            .activatable(true)
            .build();
        let image = gtk::Image::from_icon_name(&collection.icon);
        row.add_prefix(&image);
        if !collection.category.is_empty() {
            let category = gtk::Label::new(Some(&collection.category));
            category.add_css_class("dim-label");
            row.add_suffix(&category);
        }
        let callback_ui = ui.clone();
        let query = collection.query.clone();
        row.connect_activated(move |_| {
            callback_ui.discover_entry.set_text(&query);
            start_catalog_page(&callback_ui, query.clone(), 0);
        });
        ui.discover_collections.append(&row);
    }
}

fn start_featured_collections_load(ui: &UiState) {
    let (sender, receiver) = mpsc::channel();
    thread::spawn(move || {
        let result = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(anyhow::Error::from)
            .and_then(|runtime| runtime.block_on(client::featured_collections()));
        let _ = sender.send(result);
    });

    let ui = ui.clone();
    glib::timeout_add_local(Duration::from_millis(100), move || {
        match receiver.try_recv() {
            Ok(Ok(collections)) => {
                render_featured_collections(&ui, &collections);
                glib::ControlFlow::Break
            }
            Ok(Err(error)) => {
                clear_list(&ui.discover_collections);
                let row = adw::ActionRow::builder()
                    .title("Featured collections unavailable")
                    .subtitle(error.to_string())
                    .build();
                ui.discover_collections.append(&row);
                glib::ControlFlow::Break
            }
            Err(TryRecvError::Empty) => glib::ControlFlow::Continue,
            Err(TryRecvError::Disconnected) => {
                clear_list(&ui.discover_collections);
                let row = adw::ActionRow::builder()
                    .title("Featured collection worker stopped")
                    .subtitle("Restart the software center and try again")
                    .build();
                ui.discover_collections.append(&row);
                glib::ControlFlow::Break
            }
        }
    });
}

fn render_featured_collections(ui: &UiState, collections: &[FeaturedCollection]) {
    clear_list(&ui.discover_collections);
    for collection in collections {
        let row = adw::ActionRow::builder()
            .title(&collection.title)
            .subtitle(&collection.description)
            .activatable(true)
            .build();
        let image = gtk::Image::from_icon_name(&collection.icon);
        row.add_prefix(&image);
        if !collection.category.is_empty() {
            let category = gtk::Label::new(Some(&collection.category));
            category.add_css_class("dim-label");
            row.add_suffix(&category);
        }
        let callback_ui = ui.clone();
        let query = collection.query.clone();
        row.connect_activated(move |_| {
            callback_ui.discover_entry.set_text(&query);
            start_catalog_page(&callback_ui, query.clone(), 0);
        });
        ui.discover_collections.append(&row);
    }
}

fn start_catalog_search(ui: &UiState) {
    let query = ui.discover_entry.text().trim().to_owned();
    if query.is_empty() {
        ui.discover_status
            .set_text("Enter a search term before searching.");
        return;
    }
    start_catalog_page(ui, query, 0);
}

fn start_catalog_page(ui: &UiState, query: String, offset: u64) {
    clear_list(&ui.discover_list);
    ui.discover_category.set_sensitive(false);
    ui.discover_previous.set_sensitive(false);
    ui.discover_next.set_sensitive(false);
    ui.discover_status.set_text(&format!(
        "Searching AppStream for “{query}” from result {}…",
        offset + 1
    ));

    let worker_query = query.clone();
    let (sender, receiver) = mpsc::channel();
    thread::spawn(move || {
        let result = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(anyhow::Error::from)
            .and_then(|runtime| {
                runtime.block_on(client::search_catalog_page(
                    &worker_query,
                    offset,
                    CATALOG_PAGE_SIZE,
                ))
            });
        let _ = sender.send(result);
    });

    let ui = ui.clone();
    glib::timeout_add_local(Duration::from_millis(100), move || {
        match receiver.try_recv() {
            Ok(Ok(page)) => {
                *ui.catalog_query.borrow_mut() = query.clone();
                render_catalog_page(&ui, page);
                glib::ControlFlow::Break
            }
            Ok(Err(error)) => {
                ui.discover_status
                    .set_text(&format!("AppStream search failed: {error}"));
                ui.discover_page_status.set_text("Search failed");
                glib::ControlFlow::Break
            }
            Err(TryRecvError::Empty) => glib::ControlFlow::Continue,
            Err(TryRecvError::Disconnected) => {
                ui.discover_status
                    .set_text("The AppStream search worker stopped unexpectedly.");
                ui.discover_page_status.set_text("Search stopped");
                glib::ControlFlow::Break
            }
        }
    });
}

fn render_catalog_page(ui: &UiState, page: CatalogPage) {
    ui.catalog_offset.set(page.offset);
    ui.catalog_total.set(page.total);
    ui.catalog_has_more.set(page.has_more);
    *ui.apps.borrow_mut() = page.applications;
    populate_catalog_categories(ui);
    render_catalog(ui);
}

fn populate_catalog_categories(ui: &UiState) {
    let mut categories = ui
        .apps
        .borrow()
        .iter()
        .flat_map(|app| app.categories.iter().cloned())
        .collect::<BTreeSet<_>>();
    categories.insert("All categories".to_owned());
    let values = categories.iter().map(String::as_str).collect::<Vec<_>>();
    let model = gtk::StringList::new(&values);
    ui.discover_category.set_model(Some(&model));
    ui.discover_category.set_selected(0);
    ui.discover_category.set_sensitive(values.len() > 1);
}

fn render_catalog(ui: &UiState) {
    clear_list(&ui.discover_list);
    let category = selected_text(&ui.discover_category);
    let apps = ui.apps.borrow();
    let filtered = apps
        .iter()
        .filter(|app| {
            category.is_empty()
                || category == "All categories"
                || app.categories.iter().any(|value| value == &category)
        })
        .collect::<Vec<_>>();
    let offset = ui.catalog_offset.get();
    let total = ui.catalog_total.get();
    let page_end = offset.saturating_add(apps.len() as u64).min(total);

    if apps.is_empty() {
        ui.discover_status
            .set_text("No AppStream applications matched this query.");
        ui.discover_page_status.set_text("No results");
    } else {
        ui.discover_status.set_text(&format!(
            "{} applications on this page match the current category filter.",
            filtered.len()
        ));
        ui.discover_page_status.set_text(&format!(
            "Showing {}–{} of {}",
            offset + 1,
            page_end,
            total
        ));
    }
    ui.discover_previous.set_sensitive(offset > 0);
    ui.discover_next.set_sensitive(ui.catalog_has_more.get());

    for app in filtered {
        let category_text = app.categories.join(", ");
        let subtitle = if app.summary.is_empty() {
            format!("{} · {}", app.package, app.kind)
        } else {
            app.summary.clone()
        };
        let row = adw::ActionRow::builder()
            .title(&app.name)
            .subtitle(&subtitle)
            .activatable(!app.package.is_empty())
            .build();
        if !app.icon.is_empty() {
            let image = gtk::Image::from_icon_name(&app.icon);
            row.add_prefix(&image);
        }
        let metadata = if category_text.is_empty() {
            app.package.clone()
        } else {
            format!("{} · {}", app.package, category_text)
        };
        let package = gtk::Label::new(Some(&metadata));
        package.add_css_class("dim-label");
        row.add_suffix(&package);
        if app.installed {
            let badge = gtk::Label::new(Some("Installed"));
            badge.add_css_class("success");
            row.add_suffix(&badge);
        }
        if !app.package.is_empty() {
            let callback_ui = ui.clone();
            let package_name = app.package.clone();
            row.connect_activated(move |_| start_package_details(&callback_ui, &package_name));
        }
        ui.discover_list.append(&row);
    }
}

fn start_package_details(ui: &UiState, package: &str) {
    let status = gtk::Label::new(Some("Loading package details…"));
    status.set_xalign(0.0);
    status.set_wrap(true);

    let list = gtk::ListBox::new();
    list.set_selection_mode(gtk::SelectionMode::None);
    list.add_css_class("boxed-list");

    let content = gtk::Box::new(gtk::Orientation::Vertical, 12);
    content.set_margin_top(18);
    content.set_margin_bottom(18);
    content.set_margin_start(18);
    content.set_margin_end(18);
    content.append(&status);
    let scrolled = gtk::ScrolledWindow::builder()
        .hexpand(true)
        .vexpand(true)
        .child(&list)
        .build();
    content.append(&scrolled);

    let window = gtk::Window::builder()
        .title(format!("Package details — {package}"))
        .transient_for(&ui.window)
        .modal(true)
        .default_width(720)
        .default_height(640)
        .child(&content)
        .build();
    window.present();

    let package = package.to_owned();
    let (sender, receiver) = mpsc::channel();
    thread::spawn(move || {
        let result = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(anyhow::Error::from)
            .and_then(|runtime| runtime.block_on(client::package_details(&package)));
        let _ = sender.send(result);
    });

    glib::timeout_add_local(Duration::from_millis(100), move || {
        match receiver.try_recv() {
            Ok(Ok(details)) => {
                render_package_details(&status, &list, &details);
                glib::ControlFlow::Break
            }
            Ok(Err(error)) => {
                status.set_text(&format!("Unable to load package details: {error}"));
                glib::ControlFlow::Break
            }
            Err(TryRecvError::Empty) => glib::ControlFlow::Continue,
            Err(TryRecvError::Disconnected) => {
                status.set_text("The package detail worker stopped unexpectedly.");
                glib::ControlFlow::Break
            }
        }
    });
}

fn render_package_details(status: &gtk::Label, list: &gtk::ListBox, details: &PackageDetailRecord) {
    clear_list(list);
    if !details.found {
        status.set_text("This package is not installed or no dpkg record was found.");
        return;
    }

    status.set_text(&format!(
        "{} {} · {} · {}",
        details.name,
        details.version,
        details.architecture,
        format_size(details.installed_size_kib)
    ));
    append_detail_row(list, "Summary", &details.summary);
    append_detail_row(list, "Section", &details.section);
    append_detail_row(list, "Priority", &details.priority);
    append_detail_row(list, "Source package", &details.source);
    append_detail_row(list, "Maintainer", &details.maintainer);
    append_detail_row(list, "Homepage", &details.homepage);
    append_detail_row(list, "Repository origin", &details.origin);
    append_detail_row(
        list,
        "Candidate version",
        if details.candidate_version.is_empty() {
            "Not reported"
        } else {
            &details.candidate_version
        },
    );
    append_detail_row(
        list,
        "Update status",
        if details.security_update {
            "Security update available"
        } else if details.upgradable {
            "Update available"
        } else {
            "Installed version is current"
        },
    );
    append_detail_row(list, "Depends", &join_or_none(&details.depends));
    append_detail_row(list, "Recommends", &join_or_none(&details.recommends));
    append_detail_row(list, "Suggests", &join_or_none(&details.suggests));
    append_detail_row(list, "Description", &details.description);
}

fn append_detail_row(list: &gtk::ListBox, title: &str, value: &str) {
    let row = adw::ActionRow::builder()
        .title(title)
        .subtitle(if value.trim().is_empty() {
            "Not reported"
        } else {
            value
        })
        .build();
    list.append(&row);
}

fn join_or_none(values: &[String]) -> String {
    if values.is_empty() {
        "None reported".to_owned()
    } else {
        values.join(", ")
    }
}

fn selected_text(dropdown: &gtk::DropDown) -> String {
    dropdown
        .selected_item()
        .and_downcast::<gtk::StringObject>()
        .map(|item| item.string().to_string())
        .unwrap_or_default()
}

fn format_size(size_kib: u64) -> String {
    if size_kib >= 1024 * 1024 {
        format!("{:.1} GiB", size_kib as f64 / (1024.0 * 1024.0))
    } else if size_kib >= 1024 {
        format!("{:.1} MiB", size_kib as f64 / 1024.0)
    } else {
        format!("{size_kib} KiB")
    }
}

fn render_backend_error(ui: &UiState, message: &str) {
    clear_list(&ui.health_list);
    ui.dashboard_status
        .set_text(&format!("Package service unavailable: {message}"));
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
