mod appstream;
mod apt;
mod dpkg;

use std::{collections::HashSet, path::PathBuf};

use anyhow::Context;
use genixbit_package_model::{
    AppRecord, CatalogPage, FeaturedCollection, PackageDetailRecord, PackageRecord, SystemHealth,
    SystemSnapshot, UpdateRecord,
};
use zbus::{connection, interface};

const BUS_NAME: &str = "com.genixbit.PackageManager1";
const OBJECT_PATH: &str = "/com/genixbit/PackageManager1";
const DEFAULT_DPKG_STATUS: &str = "/var/lib/dpkg/status";
const REBOOT_REQUIRED_PATH: &str = "/var/run/reboot-required";

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
    async fn dpkg_status(&self) -> anyhow::Result<String> {
        tokio::fs::read_to_string(&self.dpkg_status_path)
            .await
            .with_context(|| {
                format!(
                    "failed to read dpkg status from {}",
                    self.dpkg_status_path.display()
                )
            })
    }

    async fn installed_packages(&self) -> anyhow::Result<Vec<PackageRecord>> {
        Ok(dpkg::parse_status(&self.dpkg_status().await?))
    }

    async fn installed_names(&self) -> anyhow::Result<HashSet<String>> {
        Ok(self
            .installed_packages()
            .await?
            .into_iter()
            .map(|package| package.name)
            .collect())
    }

    async fn snapshot(&self) -> anyhow::Result<SystemSnapshot> {
        let status = self.dpkg_status().await?;
        let installed = dpkg::parse_status(&status);
        let status_metrics = dpkg::status_metrics(&status);
        let apt_available = apt::is_available().await;
        let updates = apt::check_updates().await.unwrap_or_default();
        let appstream_available = appstream::is_available().await;

        let mut update_sources = updates
            .iter()
            .map(|update| update.source.clone())
            .collect::<Vec<_>>();
        update_sources.sort();
        update_sources.dedup();

        let health = SystemHealth {
            dpkg_status_readable: true,
            apt_available,
            appstream_available,
            reboot_required: std::path::Path::new(REBOOT_REQUIRED_PATH).exists(),
            installed_count: installed.len() as u64,
            installed_size_kib: installed
                .iter()
                .map(|package| package.installed_size_kib)
                .sum(),
            essential_count: installed.iter().filter(|package| package.essential).count() as u64,
            broken_package_count: status_metrics.broken_package_count,
            update_count: updates.len() as u64,
            security_update_count: updates.iter().filter(|update| update.security).count() as u64,
            update_sources,
        };

        Ok(SystemSnapshot {
            installed,
            updates,
            health,
        })
    }
}

#[interface(name = "com.genixbit.PackageManager1")]
impl PackageManager {
    async fn version(&self) -> String {
        env!("CARGO_PKG_VERSION").to_owned()
    }

    async fn system_snapshot(&self) -> zbus::fdo::Result<SystemSnapshot> {
        self.snapshot().await.map_err(dbus_failed)
    }

    async fn system_health(&self) -> zbus::fdo::Result<SystemHealth> {
        self.snapshot()
            .await
            .map(|snapshot| snapshot.health)
            .map_err(dbus_failed)
    }

    async fn list_installed(&self) -> zbus::fdo::Result<Vec<PackageRecord>> {
        self.installed_packages().await.map_err(dbus_failed)
    }

    async fn check_updates(&self) -> zbus::fdo::Result<Vec<UpdateRecord>> {
        apt::check_updates().await.map_err(dbus_failed)
    }

    async fn package_details(&self, package: &str) -> zbus::fdo::Result<PackageDetailRecord> {
        validate_package_name(package)?;
        let status = self.dpkg_status().await.map_err(dbus_failed)?;
        let mut details = dpkg::package_details(&status, package);
        if !details.found {
            return Ok(details);
        }

        if let Ok(policy) = apt::package_policy(package).await {
            details.candidate_version = policy.candidate_version;
            details.origin = policy.origin;
            details.upgradable = policy.upgradable;
            details.security_update = policy.security_update;
        }
        Ok(details)
    }

    async fn featured_collections(&self) -> Vec<FeaturedCollection> {
        appstream::featured_collections()
    }

    async fn search_catalog(&self, query: &str) -> zbus::fdo::Result<Vec<AppRecord>> {
        let installed_names = self.installed_names().await.map_err(dbus_failed)?;
        appstream::search(query, &installed_names)
            .await
            .map_err(dbus_failed)
    }

    async fn search_catalog_page(
        &self,
        query: &str,
        offset: u64,
        limit: u64,
    ) -> zbus::fdo::Result<CatalogPage> {
        let installed_names = self.installed_names().await.map_err(dbus_failed)?;
        appstream::search_page(query, offset, limit, &installed_names)
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
