mod appstream;
mod apt;
mod apt_live;
mod apt_plan;
mod apt_simulation;
mod authorization;
mod dpkg;
mod journal;
mod simulation_control;
mod transaction;

use std::{collections::HashSet, path::PathBuf};

use anyhow::Context;
use apt_live::AptSimulationOutcome;
use authorization::AuthorizationHelper;
use genixbit_package_model::{
    AppRecord, CatalogPage, FeaturedCollection, PackageDetailRecord, PackageRecord, SystemHealth,
    SystemSnapshot, TransactionEvent, TransactionPreview, TransactionQueueSnapshot,
    TransactionRecord, UpdateRecord,
};
use journal::TransactionJournal;
use simulation_control::SimulationControl;
use transaction::TransactionManager;
use zbus::{Connection, connection, interface, message::Header, object_server::SignalEmitter};

const BUS_NAME: &str = "com.genixbit.PackageManager1";
const OBJECT_PATH: &str = "/com/genixbit/PackageManager1";
const DEFAULT_DPKG_STATUS: &str = "/var/lib/dpkg/status";
const REBOOT_REQUIRED_PATH: &str = "/var/run/reboot-required";

#[derive(Debug)]
struct PackageManager {
    dpkg_status_path: PathBuf,
    authorization: AuthorizationHelper,
    transactions: TransactionManager,
    simulation_control: SimulationControl,
}

impl Default for PackageManager {
    fn default() -> Self {
        let dpkg_status_path = std::env::var_os("GENIXPKGD_DPKG_STATUS")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(DEFAULT_DPKG_STATUS));
        Self {
            dpkg_status_path,
            authorization: AuthorizationHelper::from_environment(),
            transactions: TransactionManager::new(TransactionJournal::from_environment()),
            simulation_control: SimulationControl::default(),
        }
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

    async fn preview_package_transaction(
        &self,
        kind: &str,
        package: &str,
    ) -> zbus::fdo::Result<(TransactionPreview, TransactionEvent)> {
        validate_package_name(package)?;
        let policy = apt::package_policy(package).await.map_err(dbus_failed)?;
        let installed = normalized_version(&policy.installed_version);
        let candidate = normalized_version(&policy.candidate_version);

        match kind {
            "install" if candidate.is_empty() => {
                return Err(zbus::fdo::Error::InvalidArgs(format!(
                    "no install candidate is available for {package}"
                )));
            }
            "remove" if installed.is_empty() => {
                return Err(zbus::fdo::Error::InvalidArgs(format!(
                    "{package} is not installed"
                )));
            }
            "upgrade" if !policy.upgradable => {
                return Err(zbus::fdo::Error::InvalidArgs(format!(
                    "no upgrade is available for {package}"
                )));
            }
            _ => {}
        }

        let simulation = apt_simulation::simulate(kind, package)
            .await
            .map_err(dbus_failed)?;
        self.transactions
            .create_preview(TransactionPreview {
                id: 0,
                kind: kind.to_owned(),
                package: package.to_owned(),
                changes: simulation.changes,
                download_size_bytes: simulation.download_size_bytes,
                installed_size_delta_bytes: simulation.installed_size_delta_bytes,
                requires_reboot: false,
                ready: false,
                summary: format!(
                    "{} Package execution remains disabled until the protected runner milestone.",
                    simulation.summary
                ),
            })
            .map_err(dbus_failed)
    }

    async fn emit_lifecycle_event(emitter: &SignalEmitter<'_>, event: &TransactionEvent) {
        if let Err(error) = emitter.transaction_event(event).await {
            tracing::warn!(
                sequence = event.sequence,
                transaction_id = event.transaction_id,
                %error,
                "failed to emit transaction lifecycle signal"
            );
        }
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
        appstream::search_page(query, &installed_names, offset, limit)
            .await
            .map_err(dbus_failed)
    }

    async fn preview_install(
        &self,
        package: &str,
        #[zbus(signal_emitter)] emitter: SignalEmitter<'_>,
    ) -> zbus::fdo::Result<TransactionPreview> {
        let (preview, event) = self.preview_package_transaction("install", package).await?;
        Self::emit_lifecycle_event(&emitter, &event).await;
        Ok(preview)
    }

    async fn preview_remove(
        &self,
        package: &str,
        #[zbus(signal_emitter)] emitter: SignalEmitter<'_>,
    ) -> zbus::fdo::Result<TransactionPreview> {
        let (preview, event) = self.preview_package_transaction("remove", package).await?;
        Self::emit_lifecycle_event(&emitter, &event).await;
        Ok(preview)
    }

    async fn preview_upgrade(
        &self,
        package: &str,
        #[zbus(signal_emitter)] emitter: SignalEmitter<'_>,
    ) -> zbus::fdo::Result<TransactionPreview> {
        let (preview, event) = self.preview_package_transaction("upgrade", package).await?;
        Self::emit_lifecycle_event(&emitter, &event).await;
        Ok(preview)
    }

    async fn queue_transaction(
        &self,
        preview_id: u64,
        #[zbus(connection)] connection: &Connection,
        #[zbus(header)] header: Header<'_>,
        #[zbus(signal_emitter)] emitter: SignalEmitter<'_>,
    ) -> zbus::fdo::Result<TransactionRecord> {
        let sender = header.sender().ok_or_else(|| {
            zbus::fdo::Error::AccessDenied("missing authenticated D-Bus caller identity".to_owned())
        })?;
        self.authorization
            .authorize_transaction_control(connection, sender, "queueing a package transaction")
            .await?;
        let (record, event) = self
            .transactions
            .queue_preview(preview_id)
            .map_err(dbus_failed)?;
        Self::emit_lifecycle_event(&emitter, &event).await;
        Ok(record)
    }

    async fn cancel_transaction(
        &self,
        transaction_id: u64,
        #[zbus(connection)] connection: &Connection,
        #[zbus(header)] header: Header<'_>,
        #[zbus(signal_emitter)] emitter: SignalEmitter<'_>,
    ) -> zbus::fdo::Result<TransactionRecord> {
        let sender = header.sender().ok_or_else(|| {
            zbus::fdo::Error::AccessDenied("missing authenticated D-Bus caller identity".to_owned())
        })?;
        self.authorization
            .authorize_transaction_control(connection, sender, "cancelling a package transaction")
            .await?;

        if self.simulation_control.is_active(transaction_id).await {
            self.simulation_control
                .request(transaction_id)
                .await
                .map_err(dbus_failed)?;
            let (record, event) = self
                .transactions
                .request_simulation_cancellation(transaction_id)
                .map_err(dbus_failed)?;
            Self::emit_lifecycle_event(&emitter, &event).await;
            return Ok(record);
        }

        let (record, event) = self
            .transactions
            .cancel(transaction_id)
            .map_err(dbus_failed)?;
        Self::emit_lifecycle_event(&emitter, &event).await;
        Ok(record)
    }

    async fn run_next_simulation(
        &self,
        #[zbus(connection)] connection: &Connection,
        #[zbus(header)] header: Header<'_>,
        #[zbus(signal_emitter)] emitter: SignalEmitter<'_>,
    ) -> zbus::fdo::Result<TransactionRecord> {
        let sender = header.sender().ok_or_else(|| {
            zbus::fdo::Error::AccessDenied("missing authenticated D-Bus caller identity".to_owned())
        })?;
        self.authorization
            .authorize_transaction_control(
                connection,
                sender,
                "running a simulated package transaction",
            )
            .await?;

        let (running, reviewed_preview, running_event) = self
            .transactions
            .begin_next_simulation()
            .map_err(dbus_failed)?;
        Self::emit_lifecycle_event(&emitter, &running_event).await;

        let cancellation = match self.simulation_control.register(running.id).await {
            Ok(cancellation) => cancellation,
            Err(error) => {
                let message = format!("failed to register simulation cancellation: {error}");
                let (failed, failed_event) = self
                    .transactions
                    .fail_simulation(running.id, &message)
                    .map_err(dbus_failed)?;
                Self::emit_lifecycle_event(&emitter, &failed_event).await;
                return Err(zbus::fdo::Error::Failed(failed.message));
            }
        };

        let (log_sender, mut log_receiver) = tokio::sync::mpsc::channel(64);
        let mut simulation = Box::pin(apt_live::run_cancellable(
            &running.kind,
            &running.package,
            cancellation,
            log_sender,
        ));
        let mut logs_open = true;
        let mut observed_progress = running.progress_basis_points;
        let outcome = loop {
            tokio::select! {
                result = &mut simulation => break result,
                log = log_receiver.recv(), if logs_open => {
                    match log {
                        Some(log) => {
                            match self.transactions.record_simulation_log(
                                running.id,
                                &log.level,
                                &log.message,
                            ) {
                                Ok(event) => Self::emit_lifecycle_event(&emitter, &event).await,
                                Err(error) => break Err(error),
                            }
                            if let Some(progress) = log.progress_basis_points
                                && progress > observed_progress
                            {
                                let message = format!("APT simulation progress: {}", log.message);
                                match self.transactions.update_simulation_progress(
                                    running.id,
                                    progress,
                                    &message,
                                ) {
                                    Ok((_, event)) => {
                                        observed_progress = progress;
                                        Self::emit_lifecycle_event(&emitter, &event).await;
                                    }
                                    Err(error) => break Err(error),
                                }
                            }
                        }
                        None => logs_open = false,
                    }
                }
            }
        };
        drop(simulation);
        if let Err(error) = self.simulation_control.clear(running.id).await {
            let message = format!("failed to clear simulation cancellation handle: {error}");
            let (failed, failed_event) = self
                .transactions
                .fail_simulation(running.id, &message)
                .map_err(dbus_failed)?;
            Self::emit_lifecycle_event(&emitter, &failed_event).await;
            return Err(zbus::fdo::Error::Failed(failed.message));
        }

        let simulation = match outcome {
            Ok(AptSimulationOutcome::Completed(simulation)) => simulation,
            Ok(AptSimulationOutcome::Cancelled) => {
                let (cancelled, cancelled_event) = self
                    .transactions
                    .cancel_active_simulation(running.id)
                    .map_err(dbus_failed)?;
                Self::emit_lifecycle_event(&emitter, &cancelled_event).await;
                return Ok(cancelled);
            }
            Err(error) => {
                let message = format!("APT simulation subprocess failed: {error}");
                let (failed, failed_event) = self
                    .transactions
                    .fail_simulation(running.id, &message)
                    .map_err(dbus_failed)?;
                Self::emit_lifecycle_event(&emitter, &failed_event).await;
                return Err(zbus::fdo::Error::Failed(failed.message));
            }
        };

        let preview_changed = simulation.changes != reviewed_preview.changes
            || simulation.download_size_bytes != reviewed_preview.download_size_bytes
            || simulation.installed_size_delta_bytes != reviewed_preview.installed_size_delta_bytes;
        if preview_changed {
            let message = "APT simulation changed since review; create and approve a new preview";
            let (failed, failed_event) = self
                .transactions
                .fail_simulation(running.id, message)
                .map_err(dbus_failed)?;
            Self::emit_lifecycle_event(&emitter, &failed_event).await;
            return Err(zbus::fdo::Error::Failed(failed.message));
        }

        let progress_message = format!("APT simulation verified: {}", simulation.summary);
        let (_, progress_event) = self
            .transactions
            .update_simulation_progress(running.id, 9_000, &progress_message)
            .map_err(dbus_failed)?;
        Self::emit_lifecycle_event(&emitter, &progress_event).await;

        let (completed, completed_event) = self
            .transactions
            .complete_simulation(running.id)
            .map_err(dbus_failed)?;
        Self::emit_lifecycle_event(&emitter, &completed_event).await;
        Ok(completed)
    }

    async fn transaction_queue(&self) -> zbus::fdo::Result<TransactionQueueSnapshot> {
        self.transactions.snapshot().map_err(dbus_failed)
    }

    async fn transaction_events(
        &self,
        after_sequence: u64,
        limit: u64,
    ) -> zbus::fdo::Result<Vec<TransactionEvent>> {
        self.transactions
            .events(after_sequence, limit)
            .map_err(dbus_failed)
    }

    async fn transaction_journal(&self) -> zbus::fdo::Result<Vec<TransactionRecord>> {
        self.transactions.journal().map_err(dbus_failed)
    }

    async fn transaction_journal_path(&self) -> String {
        self.transactions.journal_path().display().to_string()
    }

    async fn install(&self, package: &str) -> zbus::fdo::Result<String> {
        validate_package_name(package)?;
        Err(zbus::fdo::Error::NotSupported(
            "direct APT execution is disabled; use a reviewed preview and protected transaction flow in a future milestone"
                .to_owned(),
        ))
    }

    async fn remove(&self, package: &str) -> zbus::fdo::Result<String> {
        validate_package_name(package)?;
        Err(zbus::fdo::Error::NotSupported(
            "direct APT execution is disabled; use a reviewed preview and protected transaction flow in a future milestone"
                .to_owned(),
        ))
    }

    #[zbus(signal)]
    async fn transaction_event(
        signal_emitter: &SignalEmitter<'_>,
        event: &TransactionEvent,
    ) -> zbus::Result<()>;
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

fn normalized_version(version: &str) -> String {
    match version.trim() {
        "" | "(none)" => String::new(),
        value => value.to_owned(),
    }
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
    use super::{normalized_version, validate_package_name};

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

    #[test]
    fn normalizes_apt_none_versions() {
        assert_eq!(normalized_version("(none)"), "");
        assert_eq!(normalized_version(" 1.2.3 "), "1.2.3");
    }
}
