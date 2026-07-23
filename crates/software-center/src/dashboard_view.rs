use genixbit_package_model::{ServiceRecord, SystemHealth, TransactionRecord};

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct DashboardSummary {
    pub installed_packages: u64,
    pub updates: u64,
    pub security_updates: u64,
    pub broken_packages: u64,
    pub reboot_required: bool,
    pub approved_services: usize,
    pub active_services: usize,
    pub failed_services: usize,
    pub recent_transactions: usize,
    pub active_transactions: usize,
    pub failed_transactions: usize,
    pub interrupted_transactions: usize,
}

impl DashboardSummary {
    pub fn status_text(&self) -> String {
        let attention = self.broken_packages
            + self.security_updates
            + self.failed_services as u64
            + self.failed_transactions as u64
            + self.interrupted_transactions as u64;
        if attention == 0 && !self.reboot_required {
            format!(
                "System overview is clear. {} packages installed, {} updates, {} approved services and {} recent transactions.",
                self.installed_packages, self.updates, self.approved_services, self.recent_transactions
            )
        } else {
            format!(
                "System overview reports {attention} attention items. {} packages installed, {} updates and {} recent transactions.",
                self.installed_packages, self.updates, self.recent_transactions
            )
        }
    }
}

pub fn summarize_dashboard(
    health: &SystemHealth,
    services: &[ServiceRecord],
    transactions: &[TransactionRecord],
) -> DashboardSummary {
    DashboardSummary {
        installed_packages: health.installed_count,
        updates: health.update_count,
        security_updates: health.security_update_count,
        broken_packages: health.broken_package_count,
        reboot_required: health.reboot_required,
        approved_services: services.len(),
        active_services: services
            .iter()
            .filter(|service| matches!(service.active_state.as_str(), "active" | "activating" | "reloading"))
            .count(),
        failed_services: services
            .iter()
            .filter(|service| service.active_state == "failed")
            .count(),
        recent_transactions: transactions.len(),
        active_transactions: transactions
            .iter()
            .filter(|record| matches!(record.state.as_str(), "queued" | "running" | "cancelling"))
            .count(),
        failed_transactions: transactions
            .iter()
            .filter(|record| record.state == "failed")
            .count(),
        interrupted_transactions: transactions
            .iter()
            .filter(|record| record.state == "interrupted")
            .count(),
    }
}

#[cfg(test)]
mod tests {
    use genixbit_package_model::{ServiceRecord, SystemHealth, TransactionRecord};

    use super::{DashboardSummary, summarize_dashboard};

    #[test]
    fn aggregates_package_service_and_transaction_state() {
        let health = SystemHealth {
            installed_count: 42,
            update_count: 4,
            security_update_count: 2,
            broken_package_count: 1,
            reboot_required: true,
            ..SystemHealth::default()
        };
        let services = [
            ServiceRecord { active_state: "active".into(), ..ServiceRecord::default() },
            ServiceRecord { active_state: "failed".into(), ..ServiceRecord::default() },
        ];
        let transactions = [
            TransactionRecord { state: "running".into(), ..TransactionRecord::default() },
            TransactionRecord { state: "failed".into(), ..TransactionRecord::default() },
            TransactionRecord { state: "interrupted".into(), ..TransactionRecord::default() },
        ];
        assert_eq!(
            summarize_dashboard(&health, &services, &transactions),
            DashboardSummary {
                installed_packages: 42,
                updates: 4,
                security_updates: 2,
                broken_packages: 1,
                reboot_required: true,
                approved_services: 2,
                active_services: 1,
                failed_services: 1,
                recent_transactions: 3,
                active_transactions: 1,
                failed_transactions: 1,
                interrupted_transactions: 1,
            }
        );
    }

    #[test]
    fn formats_clear_and_attention_states() {
        let clear = DashboardSummary { installed_packages: 10, approved_services: 1, ..DashboardSummary::default() };
        assert!(clear.status_text().starts_with("System overview is clear."));
        let attention = DashboardSummary { security_updates: 2, ..DashboardSummary::default() };
        assert!(attention.status_text().contains("2 attention items"));
    }
}
