from pathlib import Path


def replace_once(text: str, old: str, new: str, path: Path) -> str:
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"{path}: expected one marker, found {count}: {old[:100]!r}")
    return text.replace(old, new, 1)


path = Path("crates/software-center/src/main.rs")
text = path.read_text()
text = replace_once(text, "mod dashboard_view;\n", "mod dashboard_view;\nmod design_tokens;\n", path)
text = replace_once(
    text,
    "use dashboard_view::summarize_dashboard;\n",
    "use dashboard_view::summarize_dashboard;\nuse design_tokens::{\n    CONTROL_SPACING, PAGE_MARGIN, PAGE_SPACING, SIDEBAR_WIDTH, WINDOW_HEIGHT, WINDOW_WIDTH,\n};\n",
    path,
)
install = '''fn install_design_tokens() {
    let Some(display) = gtk::gdk::Display::default() else {
        return;
    };
    let provider = gtk::CssProvider::new();
    provider.load_from_data(design_tokens::CSS);
    gtk::style_context_add_provider_for_display(
        &display,
        &provider,
        gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );
}

'''
text = replace_once(text, "fn main() -> glib::ExitCode {\n", install + "fn main() -> glib::ExitCode {\n", path)
text = replace_once(text, "fn build_ui(application: &adw::Application) {\n", "fn build_ui(application: &adw::Application) {\n    install_design_tokens();\n", path)
old_header = '''    let header = adw::HeaderBar::new();
    header.set_title_widget(Some(&adw::WindowTitle::new(
        "GenixBit Software Center",
        "Native software management for GenixBit OS",
    )));
'''
new_header = '''    let header = adw::HeaderBar::new();
    let window_title = adw::WindowTitle::new(
        "GenixBit Software Center",
        "Native software management for GenixBit OS",
    );
    window_title.add_css_class("genixbit-brand-title");
    header.set_title_widget(Some(&window_title));
'''
text = replace_once(text, old_header, new_header, path)
text = text.replace("sidebar.set_width_request(230);", "sidebar.set_width_request(SIDEBAR_WIDTH);")
text = text.replace("split.set_position(230);", "split.set_position(SIDEBAR_WIDTH);")
text = text.replace(".default_width(1180)", ".default_width(WINDOW_WIDTH)")
text = text.replace(".default_height(760)", ".default_height(WINDOW_HEIGHT)")
old_page = '''fn page_box() -> gtk::Box {
    let page = gtk::Box::new(gtk::Orientation::Vertical, 12);
    page.set_margin_top(24);
    page.set_margin_bottom(24);
    page.set_margin_start(24);
    page.set_margin_end(24);
    page
}
'''
new_page = '''fn page_box() -> gtk::Box {
    let page = gtk::Box::new(gtk::Orientation::Vertical, PAGE_SPACING);
    page.set_margin_top(PAGE_MARGIN);
    page.set_margin_bottom(PAGE_MARGIN);
    page.set_margin_start(PAGE_MARGIN);
    page.set_margin_end(PAGE_MARGIN);
    page
}
'''
text = replace_once(text, old_page, new_page, path)
text = text.replace("gtk::Box::new(gtk::Orientation::Horizontal, 8)", "gtk::Box::new(gtk::Orientation::Horizontal, CONTROL_SPACING)")
path.write_text(text)

roadmap_path = Path("docs/ROADMAP.md")
roadmap = roadmap_path.read_text()
roadmap = replace_once(
    roadmap,
    "- [ ] Branded icon and design tokens\n",
    "- [x] Branded icon and design tokens\n",
    roadmap_path,
)
roadmap_path.write_text(roadmap)
