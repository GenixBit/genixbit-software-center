from pathlib import Path


def replace_once(text: str, old: str, new: str, path: Path) -> str:
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"{path}: expected one marker, found {count}: {old[:140]!r}")
    return text.replace(old, new, 1)


model_path = Path("crates/package-model/src/lib.rs")
model = model_path.read_text()
model = replace_once(
    model,
    "#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize, Type)]\npub struct CatalogPage {\n",
    "#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize, Type)]\npub struct CuratedCollection {\n    pub id: String,\n    pub title: String,\n    pub description: String,\n    pub query: String,\n    pub category: String,\n    pub icon: String,\n    pub applications: Vec<AppRecord>,\n}\n\n#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize, Type)]\npub struct CatalogPage {\n",
    model_path,
)
model_path.write_text(model)

appstream_path = Path("crates/genixpkgd/src/appstream.rs")
appstream = appstream_path.read_text()
appstream = replace_once(
    appstream,
    "use genixbit_package_model::{AppRecord, CatalogPage, FeaturedCollection};",
    "use genixbit_package_model::{AppRecord, CatalogPage, CuratedCollection, FeaturedCollection};",
    appstream_path,
)
appstream = replace_once(
    appstream,
    "const MAX_CATALOG_RESULTS: usize = 500;\nconst MAX_PAGE_SIZE: u64 = 100;\n",
    "const MAX_CATALOG_RESULTS: usize = 500;\nconst MAX_PAGE_SIZE: u64 = 100;\nconst CURATED_SEARCH_LIMIT: u64 = 40;\nconst CURATED_APPS_PER_COLLECTION: usize = 6;\n",
    appstream_path,
)
appstream = replace_once(
    appstream,
    "}\n\npub async fn search(\n",
    "}\n\npub async fn curated_catalog(installed_packages: &HashSet<String>) -> Vec<CuratedCollection> {\n    let mut seen = BTreeSet::new();\n    let mut catalogue = Vec::new();\n\n    for collection in featured_collections() {\n        let applications = match search_page(\n            &collection.query,\n            installed_packages,\n            0,\n            CURATED_SEARCH_LIMIT,\n        )\n        .await\n        {\n            Ok(page) => curate_applications(&collection, &page.applications, &mut seen),\n            Err(error) => {\n                tracing::warn!(\n                    collection = %collection.id,\n                    %error,\n                    \"failed to resolve curated AppStream collection\"\n                );\n                Vec::new()\n            }\n        };\n\n        catalogue.push(CuratedCollection {\n            id: collection.id,\n            title: collection.title,\n            description: collection.description,\n            query: collection.query,\n            category: collection.category,\n            icon: collection.icon,\n            applications,\n        });\n    }\n\n    catalogue\n}\n\nfn curate_applications(\n    collection: &FeaturedCollection,\n    candidates: &[AppRecord],\n    seen: &mut BTreeSet<String>,\n) -> Vec<AppRecord> {\n    let category = collection.category.to_ascii_lowercase();\n    let mut preferred = candidates\n        .iter()\n        .filter(|app| {\n            app.categories\n                .iter()\n                .any(|value| value.to_ascii_lowercase() == category)\n        })\n        .collect::<Vec<_>>();\n\n    if preferred.is_empty() {\n        preferred = candidates.iter().collect();\n    }\n\n    preferred\n        .into_iter()\n        .filter_map(|app| {\n            let key = format!(\"{}\\0{}\", app.id, app.package);\n            seen.insert(key).then(|| app.clone())\n        })\n        .take(CURATED_APPS_PER_COLLECTION)\n        .collect()\n}\n\npub async fn search(\n",
    appstream_path,
)
appstream = replace_once(
    appstream,
    "    use std::collections::HashSet;\n",
    "    use std::collections::{BTreeSet, HashSet};\n",
    appstream_path,
)
appstream = replace_once(
    appstream,
    "        AppRecord, featured_collections, paginate, parse_categories, parse_search, validate_page,\n        validate_query,\n",
    "        AppRecord, curate_applications, featured_collections, paginate, parse_categories,\n        parse_search, validate_page, validate_query,\n",
    appstream_path,
)
appstream = replace_once(
    appstream,
    "    #[test]\n    fn paginates_catalog_records_without_overlap() {\n",
    "    #[test]\n    fn curates_category_matches_with_stable_global_deduplication() {\n        let collection = featured_collections().remove(0);\n        let candidates = vec![\n            AppRecord {\n                id: \"editor.desktop\".into(),\n                name: \"Editor\".into(),\n                package: \"editor\".into(),\n                categories: vec![\"Development\".into()],\n                ..AppRecord::default()\n            },\n            AppRecord {\n                id: \"music.desktop\".into(),\n                name: \"Music\".into(),\n                package: \"music\".into(),\n                categories: vec![\"AudioVideo\".into()],\n                ..AppRecord::default()\n            },\n        ];\n        let mut seen = BTreeSet::new();\n        let curated = curate_applications(&collection, &candidates, &mut seen);\n        assert_eq!(curated.len(), 1);\n        assert_eq!(curated[0].package, \"editor\");\n        assert!(curate_applications(&collection, &candidates, &mut seen).is_empty());\n    }\n\n    #[test]\n    fn paginates_catalog_records_without_overlap() {\n",
    appstream_path,
)
appstream_path.write_text(appstream)

backend_path = Path("crates/genixpkgd/src/main.rs")
backend = backend_path.read_text()
backend = replace_once(
    backend,
    "    AppRecord, CatalogPage, FeaturedCollection, PackageDetailRecord, PackageRecord, ServiceRecord,\n",
    "    AppRecord, CatalogPage, CuratedCollection, FeaturedCollection, PackageDetailRecord,\n    PackageRecord, ServiceRecord,\n",
    backend_path,
)
backend = replace_once(
    backend,
    "    async fn featured_collections(&self) -> Vec<FeaturedCollection> {\n        appstream::featured_collections()\n    }\n\n",
    "    async fn featured_collections(&self) -> Vec<FeaturedCollection> {\n        appstream::featured_collections()\n    }\n\n    async fn curated_catalogue(&self) -> zbus::fdo::Result<Vec<CuratedCollection>> {\n        let installed_names = self.installed_names().await.map_err(dbus_failed)?;\n        Ok(appstream::curated_catalog(&installed_names).await)\n    }\n\n",
    backend_path,
)
backend_path.write_text(backend)

client_path = Path("crates/software-center/src/client.rs")
client = client_path.read_text()
client = replace_once(
    client,
    "    CatalogPage, FeaturedCollection, PackageDetailRecord, PackageRecord, ServiceRecord,\n",
    "    CatalogPage, CuratedCollection, FeaturedCollection, PackageDetailRecord, PackageRecord,\n    ServiceRecord,\n",
    client_path,
)
client = replace_once(
    client,
    "    async fn featured_collections(&self) -> zbus::Result<Vec<FeaturedCollection>>;\n",
    "    async fn featured_collections(&self) -> zbus::Result<Vec<FeaturedCollection>>;\n    async fn curated_catalogue(&self) -> zbus::Result<Vec<CuratedCollection>>;\n",
    client_path,
)
client = replace_once(
    client,
    "pub async fn search_catalog_page(\n",
    "pub async fn curated_catalogue() -> anyhow::Result<Vec<CuratedCollection>> {\n    let connection = connect().await?;\n    let proxy = PackageManagerProxy::new(&connection)\n        .await\n        .context(\"failed to create package-manager proxy\")?;\n    proxy\n        .curated_catalogue()\n        .await\n        .context(\"failed to load the curated AppStream catalogue\")\n}\n\npub async fn search_catalog_page(\n",
    client_path,
)
client_path.write_text(client)

xml_path = Path("dbus/com.genixbit.PackageManager1.xml")
xml = xml_path.read_text()
xml = replace_once(
    xml,
    "    <method name=\"FeaturedCollections\">\n      <arg name=\"collections\" type=\"a(ssssss)\" direction=\"out\"/>\n    </method>\n",
    "    <method name=\"FeaturedCollections\">\n      <arg name=\"collections\" type=\"a(ssssss)\" direction=\"out\"/>\n    </method>\n    <method name=\"CuratedCatalogue\">\n      <arg name=\"collections\" type=\"a(ssssssa(sssssssasb))\" direction=\"out\"/>\n    </method>\n",
    xml_path,
)
xml_path.write_text(xml)

ui_path = Path("crates/software-center/src/main.rs")
ui = ui_path.read_text()
ui = replace_once(
    ui,
    "    AppRecord, CatalogPage, FeaturedCollection, PackageDetailRecord, PackageRecord, ServiceRecord,\n",
    "    AppRecord, CatalogPage, CuratedCollection, PackageDetailRecord, PackageRecord, ServiceRecord,\n",
    ui_path,
)
ui = replace_once(
    ui,
    "    start_featured_collections_load(&ui);\n",
    "    start_curated_catalogue_load(&ui);\n",
    ui_path,
)
start = ui.index("fn start_featured_collections_load(ui: &UiState) {")
end = ui.index("fn start_catalog_search(ui: &UiState) {", start)
replacement = '''fn start_curated_catalogue_load(ui: &UiState) {
    let (sender, receiver) = mpsc::channel();
    thread::spawn(move || {
        let result = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(anyhow::Error::from)
            .and_then(|runtime| runtime.block_on(client::curated_catalogue()));
        let _ = sender.send(result);
    });

    let ui = ui.clone();
    glib::timeout_add_local(Duration::from_millis(100), move || {
        match receiver.try_recv() {
            Ok(Ok(collections)) => {
                render_curated_catalogue(&ui, &collections);
                glib::ControlFlow::Break
            }
            Ok(Err(error)) => {
                clear_list(&ui.discover_collections);
                let row = adw::ActionRow::builder()
                    .title("Curated catalogue unavailable")
                    .subtitle(error.to_string())
                    .build();
                ui.discover_collections.append(&row);
                glib::ControlFlow::Break
            }
            Err(TryRecvError::Empty) => glib::ControlFlow::Continue,
            Err(TryRecvError::Disconnected) => {
                clear_list(&ui.discover_collections);
                let row = adw::ActionRow::builder()
                    .title("Curated catalogue worker stopped")
                    .subtitle("Restart the software center and try again")
                    .build();
                ui.discover_collections.append(&row);
                glib::ControlFlow::Break
            }
        }
    });
}

fn render_curated_catalogue(ui: &UiState, collections: &[CuratedCollection]) {
    clear_list(&ui.discover_collections);
    let application_count = collections
        .iter()
        .map(|collection| collection.applications.len())
        .sum::<usize>();
    ui.discover_status.set_text(&format!(
        "{} editorial collections with {} locally indexed application picks. Choose a shelf or search the full catalogue.",
        collections.len(), application_count
    ));

    if collections.is_empty() {
        let row = adw::ActionRow::builder()
            .title("No curated collections available")
            .subtitle("The local AppStream catalogue did not return editorial shelves.")
            .build();
        ui.discover_collections.append(&row);
        return;
    }

    for collection in collections {
        let header = adw::ActionRow::builder()
            .title(&collection.title)
            .subtitle(&collection.description)
            .activatable(true)
            .build();
        header.add_prefix(&gtk::Image::from_icon_name(&collection.icon));
        let count = gtk::Label::new(Some(&format!(
            "{} picks",
            collection.applications.len()
        )));
        count.add_css_class("dim-label");
        header.add_suffix(&count);
        let callback_ui = ui.clone();
        let query = collection.query.clone();
        header.connect_activated(move |_| {
            callback_ui.discover_entry.set_text(&query);
            start_catalog_page(&callback_ui, query.clone(), 0);
        });
        ui.discover_collections.append(&header);

        if collection.applications.is_empty() {
            let row = adw::ActionRow::builder()
                .title("No matching applications indexed locally")
                .subtitle(format!("Use the {} shelf search to inspect the full local catalogue.", collection.title))
                .build();
            ui.discover_collections.append(&row);
            continue;
        }

        for app in &collection.applications {
            let subtitle = if app.summary.trim().is_empty() {
                app.package.clone()
            } else {
                format!("{} · {}", app.summary, app.package)
            };
            let row = adw::ActionRow::builder()
                .title(format!("↳ {}", app.name))
                .subtitle(&subtitle)
                .activatable(!app.package.is_empty())
                .build();
            if !app.icon.is_empty() {
                row.add_prefix(&gtk::Image::from_icon_name(&app.icon));
            }
            if app.installed {
                let badge = gtk::Label::new(Some("Installed"));
                badge.add_css_class("success");
                row.add_suffix(&badge);
            }
            if !app.package.is_empty() {
                let callback_ui = ui.clone();
                let package = app.package.clone();
                row.connect_activated(move |_| start_package_details(&callback_ui, &package));
            }
            ui.discover_collections.append(&row);
        }
    }
}

'''
ui = ui[:start] + replacement + ui[end:]
ui_path.write_text(ui)

roadmap_path = Path("docs/ROADMAP.md")
roadmap = roadmap_path.read_text()
roadmap = replace_once(
    roadmap,
    "- [ ] Curated Discover catalogue\n",
    "- [x] Curated Discover catalogue\n",
    roadmap_path,
)
roadmap_path.write_text(roadmap)
