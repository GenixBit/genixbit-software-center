use std::collections::BTreeMap;

use genixbit_package_model::PackageRecord;

pub fn parse_status(input: &str) -> Vec<PackageRecord> {
    let mut packages = Vec::new();
    let mut fields = BTreeMap::<String, String>::new();
    let mut current_key: Option<String> = None;

    for line in input.lines().chain(std::iter::once("")) {
        if line.trim().is_empty() {
            if let Some(package) = package_from_fields(&fields) {
                packages.push(package);
            }
            fields.clear();
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

    packages.sort_by(|left, right| left.name.cmp(&right.name));
    packages
}

fn package_from_fields(fields: &BTreeMap<String, String>) -> Option<PackageRecord> {
    let status = fields.get("Status")?;
    if status != "install ok installed" {
        return None;
    }

    let name = fields.get("Package")?.to_owned();
    let summary = fields
        .get("Description")
        .and_then(|value| value.lines().next())
        .unwrap_or_default()
        .to_owned();

    Some(PackageRecord {
        name,
        version: fields.get("Version").cloned().unwrap_or_default(),
        architecture: fields.get("Architecture").cloned().unwrap_or_default(),
        summary,
        section: fields.get("Section").cloned().unwrap_or_default(),
        installed_size_kib: fields
            .get("Installed-Size")
            .and_then(|value| value.parse().ok())
            .unwrap_or_default(),
        essential: fields.get("Essential").is_some_and(|value| value == "yes"),
        priority: fields.get("Priority").cloned().unwrap_or_default(),
        source: fields
            .get("Source")
            .cloned()
            .unwrap_or_else(|| "dpkg".to_owned()),
    })
}

#[cfg(test)]
mod tests {
    use super::parse_status;

    const STATUS: &str = r#"Package: bash
Status: install ok installed
Priority: required
Section: shells
Installed-Size: 7336
Essential: yes
Architecture: amd64
Version: 5.2.21-2ubuntu4
Description: GNU Bourne Again SHell
 Bash is an sh-compatible command language interpreter.

Package: removed-package
Status: deinstall ok config-files
Architecture: amd64
Version: 1.0
Description: should not be returned

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
}
