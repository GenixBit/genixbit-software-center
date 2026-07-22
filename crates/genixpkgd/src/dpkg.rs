use std::collections::BTreeMap;

use genixbit_package_model::{PackageDetailRecord, PackageRecord};

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct StatusMetrics {
    pub broken_package_count: u64,
}

pub fn parse_status(input: &str) -> Vec<PackageRecord> {
    let mut packages = parse_paragraphs(input)
        .into_iter()
        .filter_map(|fields| package_from_fields(&fields))
        .collect::<Vec<_>>();
    packages.sort_by(|left, right| left.name.cmp(&right.name));
    packages
}

pub fn package_details(input: &str, package_name: &str) -> PackageDetailRecord {
    parse_paragraphs(input)
        .into_iter()
        .find(|fields| {
            fields.get("Package").is_some_and(|name| name == package_name)
                && fields
                    .get("Status")
                    .is_some_and(|status| status == "install ok installed")
        })
        .map(|fields| details_from_fields(&fields))
        .unwrap_or_else(|| PackageDetailRecord {
            name: package_name.to_owned(),
            ..PackageDetailRecord::default()
        })
}

pub fn status_metrics(input: &str) -> StatusMetrics {
    let broken_package_count = parse_paragraphs(input)
        .into_iter()
        .filter(|fields| {
            let Some(status) = fields.get("Status") else {
                return false;
            };
            status != "install ok installed"
                && (status.contains("half-configured")
                    || status.contains("half-installed")
                    || status.contains("unpacked")
                    || status.contains("triggers-pending"))
        })
        .count() as u64;

    StatusMetrics {
        broken_package_count,
    }
}

fn parse_paragraphs(input: &str) -> Vec<BTreeMap<String, String>> {
    let mut paragraphs = Vec::new();
    let mut fields = BTreeMap::<String, String>::new();
    let mut current_key: Option<String> = None;

    for line in input.lines().chain(std::iter::once("")) {
        if line.trim().is_empty() {
            if !fields.is_empty() {
                paragraphs.push(std::mem::take(&mut fields));
            }
            current_key = None;
            continue;
        }

        if line.starts_with([' ', '\t']) {
            if let Some(key) = current_key.as_ref()
                && let Some(value) = fields.get_mut(key)
            {
                value.push('\n');
                value.push_str(line.trim());
            }
            continue;
        }

        if let Some((key, value)) = line.split_once(':') {
            let key = key.trim().to_owned();
            fields.insert(key.clone(), value.trim().to_owned());
            current_key = Some(key);
        }
    }

    paragraphs
}

fn package_from_fields(fields: &BTreeMap<String, String>) -> Option<PackageRecord> {
    let status = fields.get("Status")?;
    if status != "install ok installed" {
        return None;
    }

    let name = fields.get("Package")?.to_owned();
    let description = fields.get("Description").cloned().unwrap_or_default();

    Some(PackageRecord {
        name,
        version: field(fields, "Version"),
        architecture: field(fields, "Architecture"),
        summary: first_line(&description),
        section: field(fields, "Section"),
        installed_size_kib: installed_size(fields),
        essential: fields.get("Essential").is_some_and(|value| value == "yes"),
        priority: field(fields, "Priority"),
        source: fields
            .get("Source")
            .cloned()
            .unwrap_or_else(|| "dpkg".to_owned()),
    })
}

fn details_from_fields(fields: &BTreeMap<String, String>) -> PackageDetailRecord {
    let description = field(fields, "Description");
    PackageDetailRecord {
        found: true,
        name: field(fields, "Package"),
        version: field(fields, "Version"),
        architecture: field(fields, "Architecture"),
        summary: first_line(&description),
        description,
        section: field(fields, "Section"),
        installed_size_kib: installed_size(fields),
        essential: fields.get("Essential").is_some_and(|value| value == "yes"),
        priority: field(fields, "Priority"),
        source: fields
            .get("Source")
            .cloned()
            .unwrap_or_else(|| "dpkg".to_owned()),
        maintainer: field(fields, "Maintainer"),
        homepage: field(fields, "Homepage"),
        depends: parse_relationships(fields.get("Depends")),
        recommends: parse_relationships(fields.get("Recommends")),
        suggests: parse_relationships(fields.get("Suggests")),
        ..PackageDetailRecord::default()
    }
}

fn field(fields: &BTreeMap<String, String>, key: &str) -> String {
    fields.get(key).cloned().unwrap_or_default()
}

fn first_line(value: &str) -> String {
    value.lines().next().unwrap_or_default().to_owned()
}

fn installed_size(fields: &BTreeMap<String, String>) -> u64 {
    fields
        .get("Installed-Size")
        .and_then(|value| value.parse().ok())
        .unwrap_or_default()
}

fn parse_relationships(value: Option<&String>) -> Vec<String> {
    value
        .into_iter()
        .flat_map(|value| value.split(','))
        .map(str::trim)
        .filter(|dependency| !dependency.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{package_details, parse_status, status_metrics};

    const STATUS: &str = r#"Package: bash
Status: install ok installed
Priority: required
Section: shells
Installed-Size: 7336
Essential: yes
Architecture: amd64
Version: 5.2.21-2ubuntu4
Maintainer: Ubuntu Developers <ubuntu-devel-discuss@lists.ubuntu.com>
Homepage: https://www.gnu.org/software/bash/
Depends: base-files (>= 2.1.12), libc6 (>= 2.38)
Recommends: bash-completion
Description: GNU Bourne Again SHell
 Bash is an sh-compatible command language interpreter.

Package: removed-package
Status: deinstall ok config-files
Architecture: amd64
Version: 1.0
Description: should not be returned

Package: interrupted-package
Status: install ok half-configured
Architecture: amd64
Version: 2.0
Description: requires repair

Package: curl
Status: install ok installed
Priority: optional
Section: web
Installed-Size: 501
Architecture: amd64
Version: 8.5.0-2ubuntu10
Source: curl
Description: command line tool for transferring data with URL syntax
"#;

    #[test]
    fn returns_only_installed_packages() {
        let packages = parse_status(STATUS);
        assert_eq!(packages.len(), 2);
        assert_eq!(packages[0].name, "bash");
        assert_eq!(packages[1].name, "curl");
    }

    #[test]
    fn maps_package_metadata() {
        let packages = parse_status(STATUS);
        let bash = &packages[0];
        assert_eq!(bash.version, "5.2.21-2ubuntu4");
        assert_eq!(bash.installed_size_kib, 7336);
        assert!(bash.essential);
        assert_eq!(bash.summary, "GNU Bourne Again SHell");
    }

    #[test]
    fn returns_detailed_package_metadata() {
        let details = package_details(STATUS, "bash");
        assert!(details.found);
        assert_eq!(details.maintainer, "Ubuntu Developers <ubuntu-devel-discuss@lists.ubuntu.com>");
        assert_eq!(details.depends.len(), 2);
        assert_eq!(details.recommends, ["bash-completion"]);
        assert!(details.description.contains("sh-compatible"));
    }

    #[test]
    fn reports_missing_packages_without_guessing() {
        let details = package_details(STATUS, "does-not-exist");
        assert!(!details.found);
        assert_eq!(details.name, "does-not-exist");
    }

    #[test]
    fn counts_interrupted_package_states() {
        assert_eq!(status_metrics(STATUS).broken_package_count, 1);
    }
}
