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
    "mod dashboard_view;\nmod security_view;\n",
    "mod dashboard_view;\nmod security_advisory;\nmod security_view;\n",
    path,
)
text = replace_once(
    text,
    "use gtk::glib;\nuse security_view::{\n",
    "use gtk::glib;\nuse security_advisory::{SecurityAdvisory, advisory_for_update};\nuse security_view::{\n",
    path,
)
old_loop = '''    for update in security {
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
new_loop = '''    for update in security {
        let Some(advisory) = advisory_for_update(update) else {
            continue;
        };
        let subtitle = format!(
            "{} · {} → {} · {} · {}",
            advisory.id,
            advisory.current_version,
            advisory.candidate_version,
            advisory.architecture,
            if advisory.source.trim().is_empty() {
                "source not reported"
            } else {
                &advisory.source
            }
        );
        let row = adw::ActionRow::builder()
            .title(&advisory.title)
            .subtitle(&subtitle)
            .activatable(true)
            .build();
        let badge = gtk::Label::new(Some("Advisory"));
        badge.add_css_class("error");
        row.add_suffix(&badge);
        let callback_ui = ui.clone();
        row.connect_activated(move |_| show_security_advisory(&callback_ui, &advisory));
        ui.security_list.append(&row);
    }
}

'''
text = replace_once(text, old_loop, new_loop, path)

details = '''fn show_security_advisory(ui: &UiState, advisory: &SecurityAdvisory) {
    let content = gtk::Box::new(gtk::Orientation::Vertical, 12);
    content.set_margin_top(18);
    content.set_margin_bottom(18);
    content.set_margin_start(18);
    content.set_margin_end(18);

    let status = gtk::Label::new(Some(
        "This advisory is derived from local APT metadata and is read only.",
    ));
    status.set_xalign(0.0);
    status.set_wrap(true);
    content.append(&status);

    let list = gtk::ListBox::new();
    list.set_selection_mode(gtk::SelectionMode::None);
    list.add_css_class("boxed-list");
    append_detail_row(&list, "Advisory ID", &advisory.id);
    append_detail_row(&list, "Package", &advisory.package);
    append_detail_row(&list, "Installed version", &advisory.current_version);
    append_detail_row(&list, "Security candidate", &advisory.candidate_version);
    append_detail_row(&list, "Architecture", &advisory.architecture);
    append_detail_row(&list, "Repository source", &advisory.source);
    append_detail_row(&list, "Coverage", &advisory.coverage_note);
    content.append(&list);

    let window = adw::Window::builder()
        .title(&advisory.title)
        .default_width(700)
        .default_height(560)
        .transient_for(&ui.window)
        .modal(true)
        .content(&content)
        .build();
    window.present();
}

'''
text = replace_once(text, "fn start_services_load(ui: &UiState) {\n", details + "fn start_services_load(ui: &UiState) {\n", path)
text = text.replace(
    '"{} Showing {} matching security updates.",',
    '"{} Showing {} matching local security advisories.",',
    1,
)
path.write_text(text)

roadmap_path = Path("docs/ROADMAP.md")
roadmap = roadmap_path.read_text()
roadmap = replace_once(roadmap, "- [ ] Security advisories\n", "- [x] Security advisories\n", roadmap_path)
roadmap_path.write_text(roadmap)
