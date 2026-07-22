use anyhow::{Context, bail};
use genixbit_package_model::UpdateRecord;
use tokio::process::Command;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct PackagePolicy {
    pub installed_version: String,
    pub candidate_version: String,
    pub origin: String,
    pub upgradable: bool,
    pub security_update: bool,
}

pub async fn is_available() -> bool {
    Command::new("apt")
        .arg("--version")
        .env("LC_ALL", "C")
        .kill_on_drop(true)
        .output()
        .await
        .is_ok_and(|output| output.status.success())
}

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

pub async fn package_policy(package: &str) -> anyhow::Result<PackagePolicy> {
    let output = Command::new("apt-cache")
        .arg("policy")
        .arg(package)
        .env("LC_ALL", "C")
        .kill_on_drop(true)
        .output()
        .await
        .context("failed to execute apt-cache policy")?;

    if !output.status.success() {
        bail!(
            "apt-cache policy failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }

    Ok(parse_policy(&String::from_utf8_lossy(&output.stdout)))
}

pub fn parse_upgradable(input: &str) -> Vec<UpdateRecord> {
    let mut updates = input
        .lines()
        .filter_map(parse_update_line)
        .collect::<Vec<_>>();
    updates.sort_by(|left, right| left.name.cmp(&right.name));
    updates
}

pub fn parse_policy(input: &str) -> PackagePolicy {
    let mut policy = PackagePolicy::default();

    for line in input.lines().map(str::trim) {
        if let Some(value) = line.strip_prefix("Installed:") {
            policy.installed_version = value.trim().to_owned();
            continue;
        }
        if let Some(value) = line.strip_prefix("Candidate:") {
            policy.candidate_version = value.trim().to_owned();
            continue;
        }

        let parts = line.split_whitespace().collect::<Vec<_>>();
        if parts.len() >= 3
            && parts[0].parse::<u32>().is_ok()
            && (parts[1].starts_with("http://") || parts[1].starts_with("https://"))
        {
            policy.origin = format!("{} {}", parts[1], parts[2]);
            if parts
                .iter()
                .any(|part| part.to_ascii_lowercase().contains("security"))
            {
                policy.security_update = true;
            }
            break;
        }
    }

    policy.upgradable = !policy.installed_version.is_empty()
        && policy.installed_version != "(none)"
        && !policy.candidate_version.is_empty()
        && policy.candidate_version != "(none)"
        && policy.installed_version != policy.candidate_version;
    policy
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
    use super::{parse_policy, parse_upgradable};

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
    fn parses_package_policy_and_origin() {
        let input = r#"curl:
  Installed: 8.5.0-2ubuntu10.5
  Candidate: 8.5.0-2ubuntu10.6
  Version table:
     8.5.0-2ubuntu10.6 500
        500 http://archive.ubuntu.com/ubuntu noble-updates/main amd64 Packages
 *** 8.5.0-2ubuntu10.5 100
        100 /var/lib/dpkg/status
"#;
        let policy = parse_policy(input);
        assert!(policy.upgradable);
        assert_eq!(policy.candidate_version, "8.5.0-2ubuntu10.6");
        assert_eq!(
            policy.origin,
            "http://archive.ubuntu.com/ubuntu noble-updates/main"
        );
        assert!(!policy.security_update);
    }

    #[test]
    fn recognizes_security_repository_origins() {
        let input = r#"openssl:
  Installed: 3.0.13-0ubuntu3.4
  Candidate: 3.0.13-0ubuntu3.5
  Version table:
     3.0.13-0ubuntu3.5 500
        500 http://security.ubuntu.com/ubuntu noble-security/main amd64 Packages
"#;
        assert!(parse_policy(input).security_update);
    }

    #[test]
    fn ignores_headers_and_malformed_lines() {
        let updates = parse_upgradable("Listing...\nnot-a-package\n\n");
        assert!(updates.is_empty());
    }
}
