use std::collections::BTreeSet;

use anyhow::{Context, bail};
use genixbit_package_model::ServiceRecord;
use tokio::process::Command;

const DEFAULT_APPROVED_SERVICES: &[&str] = &["genixpkgd.service"];
const APPROVED_SERVICES_ENV: &str = "GENIXPKGD_APPROVED_SERVICES";

pub fn approved_service_names() -> anyhow::Result<Vec<String>> {
    let configured = std::env::var(APPROVED_SERVICES_ENV).ok();
    let values = configured
        .as_deref()
        .map(|value| value.split(',').collect::<Vec<_>>())
        .unwrap_or_else(|| DEFAULT_APPROVED_SERVICES.to_vec());

    let mut services = BTreeSet::new();
    for value in values {
        let unit = value.trim();
        if unit.is_empty() {
            continue;
        }
        validate_service_unit(unit)?;
        services.insert(unit.to_owned());
    }
    Ok(services.into_iter().collect())
}

pub async fn inspect_approved_services() -> anyhow::Result<Vec<ServiceRecord>> {
    let services = approved_service_names()?;
    if services.is_empty() {
        return Ok(Vec::new());
    }

    let output = Command::new("systemctl")
        .arg("show")
        .arg("--no-pager")
        .arg("--property=Id,Description,LoadState,ActiveState,SubState,UnitFileState")
        .args(&services)
        .env("LC_ALL", "C")
        .output()
        .await
        .context("failed to execute systemctl show for approved services")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_owned();
        bail!(
            "systemctl show failed{}",
            if stderr.is_empty() {
                String::new()
            } else {
                format!(": {stderr}")
            }
        );
    }

    let stdout = String::from_utf8(output.stdout)
        .context("systemctl returned non-UTF-8 service metadata")?;
    Ok(parse_show_output(&stdout))
}

pub fn validate_service_unit(unit: &str) -> anyhow::Result<()> {
    if unit.is_empty() || unit.len() > 128 || !unit.ends_with(".service") {
        bail!("approved unit must be a .service name of at most 128 bytes");
    }
    if unit.starts_with('.') || unit.starts_with('-') {
        bail!("approved service unit has an invalid leading character");
    }
    if !unit
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-' | b'@'))
    {
        bail!("approved service unit contains unsupported characters");
    }
    Ok(())
}

fn parse_show_output(output: &str) -> Vec<ServiceRecord> {
    output
        .split("\n\n")
        .filter_map(|block| {
            let mut record = ServiceRecord::default();
            for line in block.lines() {
                let Some((key, value)) = line.split_once('=') else {
                    continue;
                };
                match key {
                    "Id" => record.name = value.to_owned(),
                    "Description" => record.description = value.to_owned(),
                    "LoadState" => record.load_state = value.to_owned(),
                    "ActiveState" => record.active_state = value.to_owned(),
                    "SubState" => record.sub_state = value.to_owned(),
                    "UnitFileState" => record.unit_file_state = value.to_owned(),
                    _ => {}
                }
            }
            (!record.name.is_empty()).then_some(record)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{parse_show_output, validate_service_unit};
    use genixbit_package_model::ServiceRecord;

    #[test]
    fn validates_only_safe_service_unit_names() {
        for valid in [
            "genixpkgd.service",
            "dbus-org.freedesktop.resolve1.service",
            "worker@alpha.service",
        ] {
            validate_service_unit(valid).unwrap();
        }

        for invalid in [
            "",
            "../ssh.service",
            "ssh;reboot.service",
            "ssh.socket",
            "-ssh.service",
            "/tmp/ssh.service",
        ] {
            assert!(
                validate_service_unit(invalid).is_err(),
                "accepted {invalid}"
            );
        }
    }

    #[test]
    fn parses_multiple_systemctl_show_blocks() {
        let records = parse_show_output(
            "Id=genixpkgd.service\nDescription=GenixBit package service\nLoadState=loaded\nActiveState=active\nSubState=running\nUnitFileState=enabled\n\nId=example.service\nDescription=Example\nLoadState=not-found\nActiveState=inactive\nSubState=dead\nUnitFileState=disabled\n",
        );

        assert_eq!(
            records,
            [
                ServiceRecord {
                    name: "genixpkgd.service".to_owned(),
                    description: "GenixBit package service".to_owned(),
                    load_state: "loaded".to_owned(),
                    active_state: "active".to_owned(),
                    sub_state: "running".to_owned(),
                    unit_file_state: "enabled".to_owned(),
                },
                ServiceRecord {
                    name: "example.service".to_owned(),
                    description: "Example".to_owned(),
                    load_state: "not-found".to_owned(),
                    active_state: "inactive".to_owned(),
                    sub_state: "dead".to_owned(),
                    unit_file_state: "disabled".to_owned(),
                },
            ]
        );
    }

    #[test]
    fn ignores_empty_and_malformed_blocks() {
        assert!(parse_show_output("Description=Missing ID\n\ninvalid\n").is_empty());
    }
}
