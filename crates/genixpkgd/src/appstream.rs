use std::collections::{BTreeSet, HashSet};

use anyhow::{Context, bail};
use genixbit_package_model::{AppRecord, CatalogPage, FeaturedCollection};
use tokio::process::Command;

const MAX_CATALOG_RESULTS: usize = 1_000;
const MAX_PAGE_SIZE: u64 = 100;

pub async fn is_available() -> bool {
    Command::new("appstreamcli")
        .arg("--version")
        .env("LC_ALL", "C")
        .kill_on_drop(true)
        .output()
        .await
        .is_ok_and(|output| output.status.success())
}

pub fn featured_collections() -> Vec<FeaturedCollection> {
    vec![
        collection("developer-tools", "Developer Tools", "IDEs, editors and software-development utilities.", "development", "applications-development-symbolic"),
        collection("creative-studio", "Creative Studio", "Graphics, photography, animation and design applications.", "graphics", "applications-graphics-symbolic"),
        collection("office-productivity", "Office & Productivity", "Writing, spreadsheets, planning and document tools.", "office", "x-office-document-symbolic"),
        collection("audio-video", "Audio & Video", "Media players, recording, editing and production software.", "multimedia", "applications-multimedia-symbolic"),
        collection("education-science", "Education & Science", "Learning, research, mathematics and scientific tools.", "science", "applications-science-symbolic"),
        collection("system-utilities", "System Utilities", "Monitoring, storage, networking and maintenance tools.", "system", "applications-system-symbolic"),
    ]
}

fn collection(id: &str, title: &str, description: &str, query: &str, icon: &str) -> FeaturedCollection {
    FeaturedCollection {
        id: id.to_owned(),
        title: title.to_owned(),
        description: description.to_owned(),
        query: query.to_owned(),
        icon: icon.to_owned(),
    }
}

pub async fn search(
    query: &str,
    installed_packages: &HashSet<String>,
) -> anyhow::Result<Vec<AppRecord>> {
    Ok(search_page(query, 0, MAX_PAGE_SIZE, installed_packages)
        .await?
        .items)
}

pub async fn search_page(
    query: &str,
    offset: u64,
    limit: u64,
    installed_packages: &HashSet<String>,
) -> anyhow::Result<CatalogPage> {
    validate_query(query)?;
    validate_page(offset, limit)?;

    let output = Command::new("appstreamcli")
        .arg("search")
        .arg(query)
        .args(["--details", "--no-color"])
        .env("LC_ALL", "C")
        .kill_on_drop(true)
        .output()
        .await
        .context("failed to execute AppStream search")?;

    if !output.status.success() && output.stdout.is_empty() {
        bail!(
            "AppStream search failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }

    let records = parse_search(&String::from_utf8_lossy(&output.stdout), installed_packages);
    Ok(paginate(records, offset, limit))
}

pub fn paginate(records: Vec<AppRecord>, offset: u64, limit: u64) -> CatalogPage {
    let total = records.len() as u64;
    let start = usize::try_from(offset).unwrap_or(usize::MAX).min(records.len());
    let end = start
        .saturating_add(usize::try_from(limit).unwrap_or(usize::MAX))
        .min(records.len());
    let items = records[start..end].to_vec();
    CatalogPage {
        items,
        offset,
        limit,
        total,
        has_more: end < records.len(),
    }
}

pub fn parse_search(input: &str, installed_packages: &HashSet<String>) -> Vec<AppRecord> {
    let mut records = Vec::new();
    let mut current: Option<AppRecord> = None;

    for line in input.lines() {
        let line = line.trim();
        if line.is_empty() || line == "---" {
            continue;
        }

        let Some((key, value)) = line.split_once(':') else {
            continue;
        };
        let key = key.trim();
        let value = value.trim();

        if key == "Identifier" {
            if let Some(record) = current.take() {
                records.push(record);
            }
            let (id, kind) = parse_identifier(value);
            current = Some(AppRecord {
                id,
                kind,
                ..AppRecord::default()
            });
            continue;
        }

        let Some(record) = current.as_mut() else {
            continue;
        };
        match key {
            "Name" => record.name = value.to_owned(),
            "Summary" => record.summary = value.to_owned(),
            "Package" => record.package = value.to_owned(),
            "Icon" => record.icon = value.to_owned(),
            "Homepage" => record.homepage = value.to_owned(),
            "Categories" => record.categories = parse_categories(value),
            _ => {}
        }
    }

    if let Some(record) = current {
        records.push(record);
    }

    let mut seen = BTreeSet::new();
    records
        .into_iter()
        .filter_map(|mut record| {
            if record.id.is_empty() || record.name.is_empty() {
                return None;
            }
            record.installed =
                !record.package.is_empty() && installed_packages.contains(record.package.as_str());
            let key = format!("{}\0{}", record.id, record.package);
            seen.insert(key).then_some(record)
        })
        .take(MAX_CATALOG_RESULTS)
        .collect()
}

fn parse_categories(value: &str) -> Vec<String> {
    let mut categories = value
        .split([';', ','])
        .map(str::trim)
        .filter(|category| !category.is_empty())
        .map(ToOwned::to_owned)
        .collect::<Vec<_>>();
    categories.sort();
    categories.dedup();
    categories
}

fn parse_identifier(value: &str) -> (String, String) {
    if let Some((id, kind)) = value.rsplit_once(" [")
        && let Some(kind) = kind.strip_suffix(']')
    {
        return (id.to_owned(), kind.to_owned());
    }
    (value.to_owned(), String::new())
}

fn validate_query(query: &str) -> anyhow::Result<()> {
    let valid =
        !query.trim().is_empty() && query.len() <= 100 && !query.chars().any(char::is_control);
    if valid {
        Ok(())
    } else {
        bail!("invalid AppStream search query")
    }
}

fn validate_page(offset: u64, limit: u64) -> anyhow::Result<()> {
    if limit == 0 || limit > MAX_PAGE_SIZE || offset > 100_000 {
        bail!("invalid catalogue page request")
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use genixbit_package_model::AppRecord;

    use super::{featured_collections, paginate, parse_categories, parse_search, validate_page, validate_query};

    #[test]
    fn parses_appstream_search_results() {
        let input = r#"Identifier: org.gnome.Builder.desktop [desktop-application]
Name: Builder
Summary: An IDE for GNOME
Package: gnome-builder
Homepage: https://apps.gnome.org/Builder/
Icon: org.gnome.Builder
Categories: Development;IDE;

Identifier: org.gnome.TextEditor.desktop [desktop-application]
Name: Text Editor
Summary: A simple text editor
Package: gnome-text-editor
Icon: org.gnome.TextEditor
Categories: Utility;TextEditor;
"#;
        let installed = HashSet::from(["gnome-builder".to_owned()]);
        let records = parse_search(input, &installed);

        assert_eq!(records.len(), 2);
        assert_eq!(records[0].id, "org.gnome.Builder.desktop");
        assert_eq!(records[0].kind, "desktop-application");
        assert_eq!(records[0].categories, ["Development", "IDE"]);
        assert!(records[0].installed);
        assert!(!records[1].installed);
    }

    #[test]
    fn paginates_catalogue_results() {
        let records = (0..5)
            .map(|index| AppRecord { name: format!("App {index}"), ..AppRecord::default() })
            .collect();
        let page = paginate(records, 2, 2);
        assert_eq!(page.total, 5);
        assert_eq!(page.items.len(), 2);
        assert_eq!(page.items[0].name, "App 2");
        assert!(page.has_more);
    }

    #[test]
    fn exposes_stable_featured_collections() {
        let collections = featured_collections();
        assert!(collections.len() >= 6);
        assert!(collections.iter().all(|item| !item.id.is_empty() && !item.query.is_empty()));
    }

    #[test]
    fn normalizes_category_lists() {
        assert_eq!(
            parse_categories("Utility; Development, Utility"),
            ["Development", "Utility"]
        );
    }

    #[test]
    fn validates_queries_and_pages() {
        assert!(validate_query("image editor").is_ok());
        assert!(validate_query("").is_err());
        assert!(validate_query("line\nbreak").is_err());
        assert!(validate_page(0, 50).is_ok());
        assert!(validate_page(0, 0).is_err());
        assert!(validate_page(0, 101).is_err());
    }
}
