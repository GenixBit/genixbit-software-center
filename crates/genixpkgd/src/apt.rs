use anyhow::{Context, bail};
use genixbit_package_model::UpdateRecord;
use tokio::process::Command;

pub async fn check_updates() -> anyhow::Result<Vec<UpdateRecord>> {
    let output = Command::new("apt")
        .args(["list", "--upgradable"])
        .env("LC_ALL", "C")
        .env("DEBIAN_FRONTEND", "noninteractive")
        .kill_on_drop(true)
        .output()
        .await
        .context("failed to execute apt update discovery")?;

    if !output.status.success() {
        bail!(
            "apt update discovery failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }

    Ok(parse_upgradable(&String::from_utf8_lossy(&output.stdout)))
}

pub fn parse_upgradable(input: &str) -> Vec<UpdateRecord> {
    let mut updates = input
        .lines()
        .filter_map(parse_update_line)
        .collect::<Vec<_>>();
    updates.sort_by(|left, right| left.name.cmp(&right.name));
    updates
}

fn parse_update_line(line: &str) -> Option<UpdateRecord> {
    let line = line.trim();
    if line.is_empty() || line.starts_with("Listing...") {
        return None;
    }

    let mut parts = line.split_whitespace();
    let package_and_source = parts.next()?;
    let candidate_version = parts.next()?.to_owned();
    let architecture = parts.next()?.to_owned();
    let (name, source) = package_and_source.split_once('/')?;
    let current_version = line
        .split_once("[upgradable from: ")
        .and_then(|(_, value)| value.strip_suffix(']'))
        .unwrap_or_default()
        .to_owned();

    Some(UpdateRecord {
        name: name.to_owned(),
        current_version,
        candidate_version,
        architecture,
        source: source.to_owned(),
        security: source.to_ascii_lowercase().contains("security"),
    })
}

#[cfg(test)]
mod tests {
    use super::parse_upgradable;

    #[test]
    fn parses_apt_list_output() {
        let input = r#"Listing...
curl/noble-updates 8.5.0-2ubuntu10.6 amd64 [upgradable from: 8.5.0-2ubuntu10.5]
openssl/noble-security 3.0.13-0ubuntu3.5 amd64 [upgradable from: 3.0.13-0ubuntu3.4]
"#;

        let updates = parse_upgradable(input);
        assert_eq!(updates.len(), 2);
        assert_eq!(updates[0].name, "curl");
        assert_eq!(updates[0].current_version, "8.5.0-2ubuntu10.5");
        assert_eq!(updates[0].candidate_version, "8.5.0-2ubuntu10.6");
        assert!(!updates[0].security);
        assert!(updates[1].security);
    }

    #[test]
    fn ignores_headers_and_malformed_lines() {
        let updates = parse_upgradable("Listing...\nnot-a-package\n\n");
        assert!(updates.is_empty());
    }
}
