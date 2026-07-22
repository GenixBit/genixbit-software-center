use anyhow::{Context, bail};
use genixbit_package_model::TransactionChange;
use tokio::process::Command;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct AptSimulation {
    pub changes: Vec<TransactionChange>,
    pub download_size_bytes: u64,
    pub installed_size_delta_bytes: i64,
    pub summary: String,
}

pub async fn simulate(kind: &str, package: &str) -> anyhow::Result<AptSimulation> {
    let mut command = Command::new("apt-get");
    command
        .arg("--simulate")
        .env("LC_ALL", "C")
        .env("DEBIAN_FRONTEND", "noninteractive")
        .kill_on_drop(true);

    match kind {
        "install" => {
            command.arg("install").arg(package);
        }
        "remove" => {
            command.arg("remove").arg(package);
        }
        "upgrade" => {
            command.args(["install", "--only-upgrade", package]);
        }
        _ => bail!("unsupported APT simulation kind {kind}"),
    }

    let output = command
        .output()
        .await
        .with_context(|| format!("failed to run APT {kind} simulation for {package}"))?;
    if !output.status.success() {
        bail!(
            "APT {kind} simulation failed for {package}: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }

    Ok(parse_simulation(&String::from_utf8_lossy(&output.stdout)))
}

pub fn parse_simulation(input: &str) -> AptSimulation {
    let mut result = AptSimulation::default();

    for line in input.lines().map(str::trim) {
        if let Some(change) = parse_change(line) {
            result.changes.push(change);
            continue;
        }
        if let Some(value) = line
            .strip_prefix("Need to get ")
            .and_then(|value| value.split_once(" of archives.").map(|(size, _)| size))
        {
            result.download_size_bytes = parse_size_bytes(value).unwrap_or_default();
            continue;
        }
        if let Some(value) = line
            .strip_prefix("After this operation, ")
            .and_then(|value| value.strip_suffix(" of additional disk space will be used."))
        {
            result.installed_size_delta_bytes = parse_size_bytes(value)
                .and_then(|bytes| i64::try_from(bytes).ok())
                .unwrap_or_default();
            continue;
        }
        if let Some(value) = line
            .strip_prefix("After this operation, ")
            .and_then(|value| value.strip_suffix(" disk space will be freed."))
        {
            result.installed_size_delta_bytes = parse_size_bytes(value)
                .and_then(|bytes| i64::try_from(bytes).ok())
                .map(|bytes| -bytes)
                .unwrap_or_default();
            continue;
        }
        if looks_like_summary(line) {
            result.summary = line.to_owned();
        }
    }

    if result.summary.is_empty() {
        result.summary = format!("APT simulation reports {} package changes", result.changes.len());
    }
    result
}

fn parse_change(line: &str) -> Option<TransactionChange> {
    let mut parts = line.split_whitespace();
    let action = parts.next()?;
    if !matches!(action, "Inst" | "Remv") {
        return None;
    }
    let package = parts.next()?.to_owned();
    let current_version = bracket_value(line).unwrap_or_default();
    let candidate_version = if action == "Inst" {
        parenthesized_value(line).unwrap_or_default()
    } else {
        String::new()
    };

    Some(TransactionChange {
        package,
        action: if action == "Inst" {
            if current_version.is_empty() {
                "install"
            } else {
                "upgrade"
            }
        } else {
            "remove"
        }
        .to_owned(),
        current_version,
        candidate_version,
    })
}

fn bracket_value(line: &str) -> Option<String> {
    let start = line.find('[')? + 1;
    let end = line[start..].find(']')? + start;
    Some(line[start..end].trim().to_owned())
}

fn parenthesized_value(line: &str) -> Option<String> {
    let start = line.find('(')? + 1;
    let end = line[start..].find(')')? + start;
    line[start..end]
        .split_whitespace()
        .next()
        .map(ToOwned::to_owned)
}

fn looks_like_summary(line: &str) -> bool {
    line.contains(" upgraded,")
        && line.contains(" newly installed,")
        && line.contains(" to remove")
}

fn parse_size_bytes(value: &str) -> Option<u64> {
    let mut parts = value.trim().split_whitespace();
    let amount = parts.next()?.replace(',', "").parse::<f64>().ok()?;
    let unit = parts.next().unwrap_or("B");
    let multiplier = match unit {
        "B" => 1.0,
        "kB" | "KB" => 1_000.0,
        "MB" => 1_000_000.0,
        "GB" => 1_000_000_000.0,
        "KiB" => 1_024.0,
        "MiB" => 1_048_576.0,
        "GiB" => 1_073_741_824.0,
        _ => return None,
    };
    let bytes = amount * multiplier;
    if bytes.is_finite() && bytes >= 0.0 && bytes <= u64::MAX as f64 {
        Some(bytes.round() as u64)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::{parse_simulation, parse_size_bytes};

    #[test]
    fn parses_install_and_upgrade_simulation() {
        let input = r#"The following NEW packages will be installed:
  ripgrep
The following packages will be upgraded:
  curl
Need to get 12.5 MB of archives.
After this operation, 3,200 kB of additional disk space will be used.
Inst ripgrep (14.1.0-1 noble [amd64])
Inst curl [8.5.0-2ubuntu10.5] (8.5.0-2ubuntu10.6 noble-updates [amd64])
Conf ripgrep (14.1.0-1 noble [amd64])
1 upgraded, 1 newly installed, 0 to remove and 4 not upgraded.
"#;
        let result = parse_simulation(input);
        assert_eq!(result.changes.len(), 2);
        assert_eq!(result.changes[0].action, "install");
        assert_eq!(result.changes[1].action, "upgrade");
        assert_eq!(result.changes[1].current_version, "8.5.0-2ubuntu10.5");
        assert_eq!(result.changes[1].candidate_version, "8.5.0-2ubuntu10.6");
        assert_eq!(result.download_size_bytes, 12_500_000);
        assert_eq!(result.installed_size_delta_bytes, 3_200_000);
    }

    #[test]
    fn parses_remove_and_freed_space() {
        let input = r#"The following packages will be REMOVED:
  nano
After this operation, 2.4 MB disk space will be freed.
Remv nano [7.2-2build1]
0 upgraded, 0 newly installed, 1 to remove and 0 not upgraded.
"#;
        let result = parse_simulation(input);
        assert_eq!(result.changes.len(), 1);
        assert_eq!(result.changes[0].action, "remove");
        assert_eq!(result.installed_size_delta_bytes, -2_400_000);
    }

    #[test]
    fn parses_decimal_and_binary_sizes() {
        assert_eq!(parse_size_bytes("1.5 MB"), Some(1_500_000));
        assert_eq!(parse_size_bytes("2 MiB"), Some(2_097_152));
        assert_eq!(parse_size_bytes("invalid"), None);
    }
}
