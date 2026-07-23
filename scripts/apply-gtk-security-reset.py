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
    "use security_view::{ALL_SECURITY_SOURCES, filter_security_updates, summarize_security};",
    "use security_view::{\n    ALL_SECURITY_SOURCES, filter_security_updates, security_filters_active, summarize_security,\n};",
)
replace_once(
    "    security_source: gtk::DropDown,\n    security_status: gtk::Label,\n",
    "    security_source: gtk::DropDown,\n    security_reset: gtk::Button,\n    security_status: gtk::Label,\n",
)
replace_once(
    '''    let (security_page, security_entry, security_source, security_status, security_list) =
        build_security_page();
''',
    '''    let (
        security_page,
        security_entry,
        security_source,
        security_reset,
        security_status,
        security_list,
    ) = build_security_page();
''',
)
replace_once(
    "        security_entry,\n        security_source,\n        security_status,\n",
    "        security_entry,\n        security_source,\n        security_reset,\n        security_status,\n",
)
replace_once(
    '''    {
        let ui = ui.clone();
        ui.security_source
            .clone()
            .connect_selected_notify(move |_| render_security(&ui));
    }
''',
    '''    {
        let ui = ui.clone();
        ui.security_source
            .clone()
            .connect_selected_notify(move |_| render_security(&ui));
    }
    {
        let ui = ui.clone();
        ui.security_reset.clone().connect_clicked(move |_| {
            ui.security_entry.set_text("");
            ui.security_source.set_selected(0);
            render_security(&ui);
        });
    }
''',
)

start = text.index("fn build_security_page() -> (")
end = text.index("fn build_list_page", start)
new_build = '''fn build_security_page() -> (
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
        "Package security updates",
        "Review security-classified package updates reported by the configured APT metadata.",
    );

    let filters = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    let entry = gtk::SearchEntry::builder()
        .placeholder_text("Search security packages, versions or sources…")
        .hexpand(true)
        .build();
    let source = gtk::DropDown::from_strings(&[ALL_SECURITY_SOURCES]);
    source.set_tooltip_text(Some("Filter by security-update source"));
    source.set_sensitive(false);
    let reset = gtk::Button::with_label("Clear filters");
    reset.set_tooltip_text(Some("Clear Security search and source filters"));
    reset.set_sensitive(false);
    filters.append(&entry);
    filters.append(&source);
    filters.append(&reset);
    page.append(&filters);

    let status = gtk::Label::new(Some("Loading package security metadata…"));
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

    (page, entry, source, reset, status, list)
}

'''
text = text[:start] + new_build + text[end:]

replace_once(
    '''    let query = ui.security_entry.text();
    let source = selected_text(&ui.security_source);
    let security = filter_security_updates(&updates, query.as_str(), &source);
''',
    '''    let query = ui.security_entry.text();
    let source = selected_text(&ui.security_source);
    ui.security_reset
        .set_sensitive(security_filters_active(query.as_str(), &source));
    let security = filter_security_updates(&updates, query.as_str(), &source);
''',
)

path.write_text(text)
