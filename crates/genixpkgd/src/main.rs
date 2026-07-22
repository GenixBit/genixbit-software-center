mod appstream;
mod apt;
mod dpkg;

use std::{collections::HashSet, path::PathBuf};

use anyhow::Context;
use genixbit_package_model::{AppRecord, PackageRecord, UpdateRecord};
use zbus::{connection, interface};

const BUS_NAME: &str = "com.genixbit.PackageManager1";
const OBJECT_PATH: &str = "/com/genixbit/PackageManager1";
const DEFAULT_DPKG_STATUS: &str = "/var/lib/dpkg/status";

#[derive(Debug)]
struct PackageManager {
    dpkg_status_path: PathBuf,
}

impl Default for PackageManager {
    fn default() -> Self {
        let dpkg_status_path = std::env::var_os("GENIXPKGD_DPKG_STATUS")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(DEFAULT_DPKG_STATUS));
        Self { dpkg_status_path }
    }
}

impl PackageManager {
    async fn installed_packages(&self) -> anyhow::Result<Vec<PackageRecord>> {
        let status = tokio::fs::read_to_string(&self.dpkg_status_path)
            .await
            .with_context(|| {
                format!(
                    "failed to read dpkg status from {}",
                    self.dpkg_status_path.display()
                )
            })?;
        Ok(dpkg::parse_status(&status))
    }
}

#[interface(name = "com.genixbit.PackageManager1")]
impl PackageManager {
    async fn version(&self) -> String {
        env!("CARGO_PKG_VERSION").to_owned()
    }

    async fn list_installed(&self) -> zbus::fdo::Result<Vec<PackageRecord>> {
        self.installed_packages().await.map_err(dbus_failed)
    }

    async fn check_updates(&self) -> zbus::fdo::Result<Vec<UpdateRecord>> {
        apt::check_updates().await.map_err(dbus_failed)
    }

    async fn search_catalog(&self, query: &str) -> zbus::fdo::Result<Vec<AppRecord>> {
        let installed = self.installed_packages().await.map_err(dbus_failed)?;
        let installed_names = installed
            .into_iter()
            .map(|package| package.name)
            .collect::<HashSet<_>>();
        appstream::search(query, &installed_names)
            .await
            .map_err(dbus_failed)
    }

    async fn install(&self, package: &str) -> zbus::fdo::Result<String> {
        validate_package_name(package)?;
        Err(zbus::fdo::Error::NotSupported(
            "APT transactions are not enabled in the read-only release".to_owned(),
        ))
    }

    async fn remove(&self, package: &str) -> zbus::fdo::Result<String> {
        validate_package_name(package)?;
        Err(zbus::fdo::Error::NotSupported(
            "APT transactions are not enabled in the read-only release".to_owned(),
        ))
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "genixpkgd=info".into()),
        )
        .init();

    let use_session_bus = std::env::var("GENIXPKGD_BUS").is_ok_and(|value| value == "session");
    let builder = if use_session_bus {
        connection::Builder::session().context("failed to connect to the session D-Bus")?
    } else {
        connection::Builder::system().context("failed to connect to the system D-Bus")?
    };

    let _connection = builder
        .name(BUS_NAME)?
        .serve_at(OBJECT_PATH, PackageManager::default())?
        .build()
        .await
        .context("failed to publish the GenixBit package-management service")?;

    tracing::info!(bus = BUS_NAME, path = OBJECT_PATH, "genixpkgd is running");
    tokio::signal::ctrl_c().await?;
    Ok(())
}

fn dbus_failed(error: impl std::fmt::Display) -> zbus::fdo::Error {
    zbus::fdo::Error::Failed(error.to_string())
}

fn validate_package_name(package: &str) -> zbus::fdo::Result<()> {
    let valid = !package.is_empty()
        && package.len() <= 128
        && package.chars().all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '+' | '-' | '.' | ':')
        });

    if valid {
        Ok(())
    } else {
        Err(zbus::fdo::Error::InvalidArgs(
            "invalid package name".to_owned(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::validate_package_name;

    #[test]
    fn accepts_debian_package_names() {
        for value in ["curl", "libgtk-4-1", "g++", "python3.13", "pkg:amd64"] {
            assert!(validate_package_name(value).is_ok(), "{value}");
        }
    }

    #[test]
    fn rejects_shell_and_path_input() {
        for value in ["", "../curl", "curl;reboot", "$(id)", "package name"] {
            assert!(validate_package_name(value).is_err(), "{value}");
        }
    }
}
