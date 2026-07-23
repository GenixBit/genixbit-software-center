from pathlib import Path


def replace_once(text: str, old: str, new: str, path: Path) -> str:
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"{path}: expected one marker, found {count}: {old[:120]!r}")
    return text.replace(old, new, 1)


main_path = Path("crates/software-center/src/main.rs")
main = main_path.read_text()
main = replace_once(main, "mod client;\n", "mod client;\nmod design;\n", main_path)
main = replace_once(
    main,
    "    let application = adw::Application::builder().application_id(APP_ID).build();\n    application.connect_activate(build_ui);\n",
    "    let application = adw::Application::builder().application_id(APP_ID).build();\n    application.connect_startup(|_| design::install());\n    application.connect_activate(build_ui);\n",
    main_path,
)
main_path.write_text(main)

meta_path = Path("data/com.genixbit.SoftwareCenter.metainfo.xml")
meta = meta_path.read_text()
meta = replace_once(
    meta,
    "  <launchable type=\"desktop-id\">com.genixbit.SoftwareCenter.desktop</launchable>\n",
    "  <launchable type=\"desktop-id\">com.genixbit.SoftwareCenter.desktop</launchable>\n  <icon type=\"stock\">com.genixbit.SoftwareCenter</icon>\n",
    meta_path,
)
meta_path.write_text(meta)

roadmap_path = Path("docs/ROADMAP.md")
roadmap = roadmap_path.read_text()
roadmap = replace_once(
    roadmap,
    "- [ ] Branded icon and design tokens\n",
    "- [x] Branded icon and design tokens\n",
    roadmap_path,
)
roadmap_path.write_text(roadmap)
