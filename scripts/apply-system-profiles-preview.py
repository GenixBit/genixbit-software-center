from pathlib import Path


def replace_once(text: str, old: str, new: str, path: Path) -> str:
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"{path}: expected one marker, found {count}: {old[:120]!r}")
    return text.replace(old, new, 1)


path = Path("crates/software-center/src/main.rs")
text = path.read_text()
text = replace_once(text, "mod stack_view;\n", "mod stack_view;\nmod system_profile;\n", path)
text = replace_once(
    text,
    "use stack_view::{\n    ALL_STACK_CATEGORIES, SoftwareStack, filter_stacks, filters_active as stack_filters_active,\n    installed_names, stack_status,\n};\n",
    "use stack_view::{\n    ALL_STACK_CATEGORIES, SoftwareStack, filter_stacks, filters_active as stack_filters_active,\n    installed_names, stack_status,\n};\nuse system_profile::{ProfileComparison, SystemProfile, compare_profile};\n",
    path,
)
text = replace_once(
    text,
    "    settings_status: gtk::Label,\n    settings: Rc<RefCell<AppSettings>>,\n",
    "    settings_status: gtk::Label,\n    profiles_status: gtk::Label,\n    settings: Rc<RefCell<AppSettings>>,\n",
    path,
)
placeholder = '''    add_placeholder_page(
        &stack,
        "profiles",
        "System Profiles",
        "document-save-symbolic",
        "Portable system profiles",
        "Export, compare and restore software configurations across GenixBit OS devices.",
    );
'''
replacement = '''    let (profiles_page, profiles_export, profiles_compare, profiles_status) =
        build_profiles_page();
    add_widget_page(
        &stack,
        "profiles",
        "System Profiles",
        "document-save-symbolic",
        &profiles_page,
    );
'''
text = replace_once(text, placeholder, replacement, path)
text = replace_once(
    text,
    "        settings_status,\n        settings: Rc::new(RefCell::new(loaded_settings)),\n",
    "        settings_status,\n        profiles_status,\n        settings: Rc::new(RefCell::new(loaded_settings)),\n",
    path,
)
text = replace_once(
    text,
    "    {\n        let ui = ui.clone();\n        ui.settings_offline\n",
    "    {\n        let ui = ui.clone();\n        profiles_export.connect_clicked(move |_| show_profile_export(&ui));\n    }\n    {\n        let ui = ui.clone();\n        profiles_compare.connect_clicked(move |_| show_profile_compare(&ui));\n    }\n    {\n        let ui = ui.clone();\n        ui.settings_offline\n",
    path,
)
text = replace_once(
    text,
    "    render_installed(ui);\n    render_stacks(ui);\n",
    "    render_installed(ui);\n    render_stacks(ui);\n    render_profiles_status(ui);\n",
    path,
)

build_profiles = '''fn build_profiles_page() -> (gtk::Box, gtk::Button, gtk::Button, gtk::Label) {
    let page = page_box();
    append_page_heading(
        &page,
        "Portable system profiles",
        "Export installed-package state, compare another profile and review a fail-closed restore plan.",
    );

    let status = gtk::Label::new(Some("Load installed-package metadata to create a profile."));
    status.set_xalign(0.0);
    status.set_wrap(true);
    page.append(&status);

    let actions = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    let export = gtk::Button::with_label("Export current profile");
    export.set_tooltip_text(Some("Open a copyable deterministic profile for this system"));
    let compare = gtk::Button::with_label("Compare or preview restore");
    compare.set_tooltip_text(Some("Paste a GenixBit profile and review differences without executing changes"));
    actions.append(&export);
    actions.append(&compare);
    page.append(&actions);

    let list = gtk::ListBox::new();
    list.set_selection_mode(gtk::SelectionMode::None);
    list.add_css_class("boxed-list");
    let export_row = adw::ActionRow::builder()
        .title("Deterministic export")
        .subtitle("Package name, installed version, architecture and section are encoded in a bounded text format.")
        .build();
    list.append(&export_row);
    let compare_row = adw::ActionRow::builder()
        .title("Comparison")
        .subtitle("Reports missing, extra and version-different packages in stable package-name order.")
        .build();
    list.append(&compare_row);
    let restore_row = adw::ActionRow::builder()
        .title("Restore preview")
        .subtitle("Essential packages are protected and all proposed changes remain informational.")
        .build();
    let badge = gtk::Label::new(Some("Execution disabled"));
    badge.add_css_class("success");
    restore_row.add_suffix(&badge);
    list.append(&restore_row);
    page.append(&list);

    (page, export, compare, status)
}

'''
text = replace_once(text, "fn build_settings_page(\n", build_profiles + "fn build_settings_page(\n", path)

profile_functions = '''fn render_profiles_status(ui: &UiState) {
    let count = ui.packages.borrow().len();
    if count == 0 {
        ui.profiles_status
            .set_text("No installed-package metadata is currently available for profile export.");
    } else {
        ui.profiles_status.set_text(&format!(
            "{} installed packages are available for deterministic profile export and comparison.",
            count
        ));
    }
}

fn show_profile_export(ui: &UiState) {
    let profile = SystemProfile::from_packages(&ui.packages.borrow());
    let text = gtk::TextView::new();
    text.set_editable(false);
    text.set_cursor_visible(false);
    text.set_monospace(true);
    text.set_wrap_mode(gtk::WrapMode::None);
    text.buffer().set_text(&profile.serialize());

    let content = gtk::Box::new(gtk::Orientation::Vertical, 12);
    content.set_margin_top(18);
    content.set_margin_bottom(18);
    content.set_margin_start(18);
    content.set_margin_end(18);
    let status = gtk::Label::new(Some(&format!(
        "Copy this bounded profile text to move {} package records between GenixBit OS devices.",
        profile.packages.len()
    )));
    status.set_xalign(0.0);
    status.set_wrap(true);
    content.append(&status);
    content.append(
        &gtk::ScrolledWindow::builder()
            .hexpand(true)
            .vexpand(true)
            .child(&text)
            .build(),
    );

    adw::Window::builder()
        .title("Export system profile")
        .transient_for(&ui.window)
        .modal(true)
        .default_width(820)
        .default_height(680)
        .content(&content)
        .build()
        .present();
}

fn show_profile_compare(ui: &UiState) {
    let editor = gtk::TextView::new();
    editor.set_monospace(true);
    editor.set_wrap_mode(gtk::WrapMode::None);
    editor.set_tooltip_text(Some("Paste a GenixBit system profile here"));

    let status = gtk::Label::new(Some(
        "Paste a GenixBit profile, then generate a read-only comparison and restore preview.",
    ));
    status.set_xalign(0.0);
    status.set_wrap(true);
    let results = gtk::ListBox::new();
    results.set_selection_mode(gtk::SelectionMode::None);
    results.add_css_class("boxed-list");
    let compare = gtk::Button::with_label("Generate restore preview");

    let content = gtk::Box::new(gtk::Orientation::Vertical, 12);
    content.set_margin_top(18);
    content.set_margin_bottom(18);
    content.set_margin_start(18);
    content.set_margin_end(18);
    content.append(&status);
    content.append(
        &gtk::ScrolledWindow::builder()
            .hexpand(true)
            .min_content_height(220)
            .child(&editor)
            .build(),
    );
    content.append(&compare);
    content.append(
        &gtk::ScrolledWindow::builder()
            .hexpand(true)
            .vexpand(true)
            .child(&results)
            .build(),
    );

    let current_packages = ui.packages.clone();
    let editor_for_compare = editor.clone();
    let status_for_compare = status.clone();
    let results_for_compare = results.clone();
    compare.connect_clicked(move |_| {
        let buffer = editor_for_compare.buffer();
        let (start, end) = buffer.bounds();
        let input = buffer.text(&start, &end, true);
        match SystemProfile::parse(input.as_str()) {
            Ok(profile) => {
                let comparison = compare_profile(&current_packages.borrow(), &profile);
                render_profile_comparison(
                    &status_for_compare,
                    &results_for_compare,
                    &comparison,
                );
            }
            Err(error) => {
                clear_list(&results_for_compare);
                status_for_compare.set_text(&format!("Unable to parse profile: {error}"));
            }
        }
    });

    adw::Window::builder()
        .title("Compare system profile")
        .transient_for(&ui.window)
        .modal(true)
        .default_width(860)
        .default_height(760)
        .content(&content)
        .build()
        .present();
}

fn render_profile_comparison(
    status: &gtk::Label,
    list: &gtk::ListBox,
    comparison: &ProfileComparison,
) {
    clear_list(list);
    status.set_text(&comparison.status_text());
    if comparison.is_identical() {
        let row = adw::ActionRow::builder()
            .title("Profile matches this system")
            .subtitle("No restore actions are required.")
            .build();
        let badge = gtk::Label::new(Some("Identical"));
        badge.add_css_class("success");
        row.add_suffix(&badge);
        list.append(&row);
        return;
    }

    for package in &comparison.install_missing {
        append_profile_action_row(list, package, "Missing from this system", "Install preview", "accent");
    }
    for change in &comparison.version_changes {
        append_profile_action_row(
            list,
            &change.name,
            &format!("{} → {}", change.current_version, change.profile_version),
            "Version preview",
            "accent",
        );
    }
    for package in &comparison.remove_extra {
        append_profile_action_row(list, package, "Not present in imported profile", "Remove preview", "error");
    }
    for package in &comparison.protected_extra {
        append_profile_action_row(
            list,
            package,
            "Essential package omitted by imported profile; removal is blocked",
            "Protected",
            "success",
        );
    }
}

fn append_profile_action_row(
    list: &gtk::ListBox,
    package: &str,
    subtitle: &str,
    badge_text: &str,
    badge_class: &str,
) {
    let row = adw::ActionRow::builder()
        .title(package)
        .subtitle(subtitle)
        .build();
    let badge = gtk::Label::new(Some(badge_text));
    badge.add_css_class(badge_class);
    row.add_suffix(&badge);
    list.append(&row);
}

'''
text = replace_once(text, "fn persist_and_render_settings(ui: &UiState) {\n", profile_functions + "fn persist_and_render_settings(ui: &UiState) {\n", path)
text = replace_once(
    text,
    "    ui.stacks_status.set_text(message);\n",
    "    ui.stacks_status.set_text(message);\n    ui.profiles_status.set_text(message);\n",
    path,
)
text = replace_once(
    text,
    "    ui.services_status.set_text(message);\n",
    "    ui.profiles_status.set_text(message);\n    ui.services_status.set_text(message);\n",
    path,
)
path.write_text(text)

roadmap_path = Path("docs/ROADMAP.md")
roadmap = roadmap_path.read_text()
roadmap = replace_once(
    roadmap,
    "- [ ] System Profiles export, comparison and restore\n",
    "- [x] System Profiles export, comparison and restore preview\n",
    roadmap_path,
)
roadmap += "\nSystem Profiles restore remains a fail-closed preview until the real transaction runner and safe APT operations are complete.\n"
roadmap_path.write_text(roadmap)
