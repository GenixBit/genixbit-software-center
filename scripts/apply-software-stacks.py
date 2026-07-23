from pathlib import Path


def replace_once(text: str, old: str, new: str, path: Path) -> str:
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"{path}: expected one marker, found {count}: {old[:100]!r}")
    return text.replace(old, new, 1)

path = Path("crates/software-center/src/main.rs")
text = path.read_text()
text = replace_once(text, "mod service_view;\n", "mod service_view;\nmod stack_view;\n", path)
text = replace_once(
    text,
    "use service_view::{\n    ALL_SERVICE_STATES, filter_services, service_filters_active, service_state_css_class,\n    service_state_label, summarize_services,\n};\n",
    "use service_view::{\n    ALL_SERVICE_STATES, filter_services, service_filters_active, service_state_css_class,\n    service_state_label, summarize_services,\n};\nuse stack_view::{\n    ALL_STACK_CATEGORIES, filter_stacks, filters_active as stack_filters_active, installed_names,\n    software_stacks, stack_status, SoftwareStack,\n};\n",
    path,
)
text = replace_once(
    text,
    "    services_list: gtk::ListBox,\n    activity_entry: gtk::SearchEntry,\n",
    "    services_list: gtk::ListBox,\n    stacks_entry: gtk::SearchEntry,\n    stacks_category: gtk::DropDown,\n    stacks_reset: gtk::Button,\n    stacks_status: gtk::Label,\n    stacks_list: gtk::ListBox,\n    activity_entry: gtk::SearchEntry,\n",
    path,
)
old_placeholder = '''    add_placeholder_page(\n        &stack,\n        "stacks",\n        "Software Stacks",\n        "view-grid-symbolic",\n        "Software stacks",\n        "Install capability-aware collections for AI, development, design and productivity.",\n    );\n'''
new_page = '''    let (\n        stacks_page,\n        stacks_entry,\n        stacks_category,\n        stacks_reset,\n        stacks_status,\n        stacks_list,\n    ) = build_stacks_page();\n    add_widget_page(\n        &stack,\n        "stacks",\n        "Software Stacks",\n        "view-grid-symbolic",\n        &stacks_page,\n    );\n'''
text = replace_once(text, old_placeholder, new_page, path)
text = replace_once(
    text,
    "        services_status,\n        services_list,\n        activity_entry,\n",
    "        services_status,\n        services_list,\n        stacks_entry,\n        stacks_category,\n        stacks_reset,\n        stacks_status,\n        stacks_list,\n        activity_entry,\n",
    path,
)
marker = '''    {\n        let ui = ui.clone();\n        ui.services_entry\n'''
handlers = '''    {\n        let ui = ui.clone();\n        ui.stacks_entry\n            .clone()\n            .connect_changed(move |_| render_stacks(&ui));\n    }\n    {\n        let ui = ui.clone();\n        ui.stacks_category\n            .clone()\n            .connect_selected_notify(move |_| render_stacks(&ui));\n    }\n    {\n        let ui = ui.clone();\n        ui.stacks_reset.clone().connect_clicked(move |_| {\n            ui.stacks_entry.set_text("");\n            ui.stacks_category.set_selected(0);\n            render_stacks(&ui);\n        });\n    }\n'''
text = replace_once(text, marker, handlers + marker, path)
text = replace_once(
    text,
    "    render_installed(ui);\n    *ui.security_updates.borrow_mut() = snapshot.updates.clone();\n",
    "    render_installed(ui);\n    render_stacks(ui);\n    *ui.security_updates.borrow_mut() = snapshot.updates.clone();\n",
    path,
)
insert_before = "fn build_list_page(\n"
build_fn = '''fn build_stacks_page() -> (\n    gtk::Box,\n    gtk::SearchEntry,\n    gtk::DropDown,\n    gtk::Button,\n    gtk::Label,\n    gtk::ListBox,\n) {\n    let page = page_box();\n    append_page_heading(\n        &page,\n        "Software stacks",\n        "Inspect curated capability bundles and see which packages are already installed. Installation remains disabled.",\n    );\n    let filters = gtk::Box::new(gtk::Orientation::Horizontal, 8);\n    let entry = gtk::SearchEntry::builder()\n        .placeholder_text("Search stacks, packages or roles…")\n        .hexpand(true)\n        .build();\n    let category = gtk::DropDown::from_strings(&[\n        ALL_STACK_CATEGORIES,\n        "AI",\n        "Development",\n        "Design",\n        "Productivity",\n    ]);\n    let reset = gtk::Button::with_label("Clear filters");\n    reset.set_sensitive(false);\n    filters.append(&entry);\n    filters.append(&category);\n    filters.append(&reset);\n    page.append(&filters);\n    let status = gtk::Label::new(Some("Loading installed package state…"));\n    status.set_xalign(0.0);\n    status.set_wrap(true);\n    page.append(&status);\n    let list = gtk::ListBox::new();\n    list.set_selection_mode(gtk::SelectionMode::None);\n    list.add_css_class("boxed-list");\n    let scrolled = gtk::ScrolledWindow::builder()\n        .hexpand(true)\n        .vexpand(true)\n        .child(&list)\n        .build();\n    page.append(&scrolled);\n    (page, entry, category, reset, status, list)\n}\n\n'''
text = replace_once(text, insert_before, build_fn + insert_before, path)
insert_render_before = "fn start_activity_load(ui: &UiState) {\n"
render_fns = '''fn render_stacks(ui: &UiState) {\n    clear_list(&ui.stacks_list);\n    let query = ui.stacks_entry.text();\n    let category = selected_text(&ui.stacks_category);\n    ui.stacks_reset\n        .set_sensitive(stack_filters_active(query.as_str(), &category));\n    let filtered = filter_stacks(query.as_str(), &category);\n    let packages = ui.packages.borrow();\n    let installed = installed_names(&packages);\n    if filtered.is_empty() {\n        ui.stacks_status\n            .set_text("No software stacks match the current filters.");\n        return;\n    }\n    let complete = filtered\n        .iter()\n        .filter(|stack| {\n            let status = stack_status(stack, &installed);\n            status.total > 0 && status.installed == status.total\n        })\n        .count();\n    ui.stacks_status.set_text(&format!(\n        "Showing {} curated stacks; {} complete. Read-only package status only.",\n        filtered.len(), complete\n    ));\n    for stack in filtered {\n        let status = stack_status(stack, &installed);\n        let row = adw::ActionRow::builder()\n            .title(stack.title)\n            .subtitle(format!("{} · {}", stack.category, stack.description))\n            .activatable(true)\n            .build();\n        row.add_prefix(&gtk::Image::from_icon_name(stack.icon));\n        let badge = gtk::Label::new(Some(&status.status_text()));\n        if status.total > 0 && status.installed == status.total {\n            badge.add_css_class("success");\n        } else {\n            badge.add_css_class("dim-label");\n        }\n        row.add_suffix(&badge);\n        let callback_ui = ui.clone();\n        let stack = stack.clone();\n        row.connect_activated(move |_| show_stack_details(&callback_ui, &stack));\n        ui.stacks_list.append(&row);\n    }\n}\n\nfn show_stack_details(ui: &UiState, stack: &SoftwareStack) {\n    let list = gtk::ListBox::new();\n    list.set_selection_mode(gtk::SelectionMode::None);\n    list.add_css_class("boxed-list");\n    let packages = ui.packages.borrow();\n    let installed = installed_names(&packages);\n    for package in stack.packages {\n        let is_installed = installed.contains(package.name);\n        let row = adw::ActionRow::builder()\n            .title(package.name)\n            .subtitle(package.role)\n            .activatable(is_installed)\n            .build();\n        let badge = gtk::Label::new(Some(if is_installed { "Installed" } else { "Missing" }));\n        badge.add_css_class(if is_installed { "success" } else { "dim-label" });\n        row.add_suffix(&badge);\n        if is_installed {\n            let callback_ui = ui.clone();\n            let package_name = package.name.to_owned();\n            row.connect_activated(move |_| start_package_details(&callback_ui, &package_name));\n        }\n        list.append(&row);\n    }\n    let content = gtk::Box::new(gtk::Orientation::Vertical, 12);\n    content.set_margin_top(18);\n    content.set_margin_bottom(18);\n    content.set_margin_start(18);\n    content.set_margin_end(18);\n    let description = gtk::Label::new(Some(stack.description));\n    description.set_xalign(0.0);\n    description.set_wrap(true);\n    content.append(&description);\n    content.append(&list);\n    let window = adw::Window::builder()\n        .title(stack.title)\n        .default_width(640)\n        .default_height(520)\n        .transient_for(&ui.window)\n        .modal(true)\n        .content(&content)\n        .build();\n    window.present();\n}\n\n'''
text = replace_once(text, insert_render_before, render_fns + insert_render_before, path)
text = replace_once(
    text,
    "    ui.services_status.set_text(message);\n    ui.services.borrow_mut().clear();\n",
    "    ui.services_status.set_text(message);\n    ui.services.borrow_mut().clear();\n    ui.stacks_status.set_text(message);\n    clear_list(&ui.stacks_list);\n",
    path,
)
path.write_text(text)

roadmap = Path("docs/ROADMAP.md")
roadmap_text = roadmap.read_text()
roadmap_text = replace_once(roadmap_text, "- [ ] Software Stacks\n", "- [x] Software Stacks\n", roadmap)
roadmap.write_text(roadmap_text)
