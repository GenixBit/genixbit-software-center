use std::collections::{BTreeSet, HashSet};

use anyhow::{Context, bail};
use genixbit_package_model::AppRecord;
use tokio::process::Command;

pub async fn search(
    query: &str,
    installed_packages: &HashSet<String>,
) -> anyhow::Result<Vec<AppRecord>> {
    validate_query(query)?;

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

    Ok(parse_search(
        &String::from_utf8_lossy(&output.stdout),
        installed_packages,
    ))
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
        .take(100)
        .collect()
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

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::{parse_search, validate_query};

    #[test]
    fn parses_appstream_search_results() {
        let input = r#"Identifier: org.gnome.Builder.desktop [desktop-application]
Name: Builder
Summary: An IDE for GNOME
Package: gnome-builder
Homepage: https://apps.gnome.org/Builder/
Icon: org.gnome.Builder

Identifier: org.gnome.TextEditor.desktop [desktop-application]
Name: Text Editor
Summary: A simple text editor
Package: gnome-text-editor
Icon: org.gnome.TextEditor
"#;
        let installed = HashSet::from(["gnome-builder".to_owned()]);
        let records = parse_search(input, &installed);

        assert_eq!(records.len(), 2);
        assert_eq!(records[0].id, "org.gnome.Builder.desktop");
        assert_eq!(records[0].kind, "desktop-application");
        assert!(records[0].installed);
        assert!(!records[1].installed);
    }

    #[test]
    fn validates_queries_without_treating_them_as_shell_input() {
        assert!(validate_query("image editor").is_ok());
        assert!(validate_query("").is_err());
        assert!(validate_query("line\nbreak").is_err());
    }
}
