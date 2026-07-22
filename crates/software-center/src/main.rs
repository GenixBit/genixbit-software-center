use adw::prelude::*;
use gtk::glib;

const APP_ID: &str = "com.genixbit.SoftwareCenter";

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

    add_page(
        &stack,
        "dashboard",
        "Dashboard",
        "view-dashboard-symbolic",
        "System overview",
        "Package health, updates, security notices and storage information will appear here.",
    );
    add_page(
        &stack,
        "discover",
        "Discover",
        "system-software-install-symbolic",
        "Discover software",
        "Browse verified applications and software stacks from GenixBit repositories.",
    );
    add_page(
        &stack,
        "installed",
        "Installed",
        "view-list-symbolic",
        "Installed software",
        "Review applications, system packages, runtimes, drivers and GenixBit components.",
    );
    add_page(
        &stack,
        "updates",
        "Updates",
        "software-update-available-symbolic",
        "System updates",
        "Review and apply verified package and operating-system updates.",
    );
    add_page(
        &stack,
        "stacks",
        "Software Stacks",
        "view-grid-symbolic",
        "Software stacks",
        "Install capability-aware collections for AI, development, design and productivity.",
    );
    add_page(
        &stack,
        "security",
        "Security",
        "security-high-symbolic",
        "Security status",
        "Package advisories, signature verification and repository trust will appear here.",
    );
    add_page(
        &stack,
        "services",
        "Services",
        "system-run-symbolic",
        "System services",
        "Inspect and control approved background services through the GenixBit system service.",
    );
    add_page(
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

    window.present();
}

fn add_page(
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

    let stack_page = stack.add_titled(&page, Some(name), sidebar_title);
    stack_page.set_icon_name(icon_name);
}
