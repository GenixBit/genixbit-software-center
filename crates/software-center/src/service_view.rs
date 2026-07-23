use genixbit_package_model::ServiceRecord;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ServiceSummary {
    pub total: usize,
    pub active: usize,
    pub failed: usize,
    pub inactive: usize,
    pub unavailable: usize,
    pub enabled: usize,
}

impl ServiceSummary {
    pub fn status_text(&self) -> String {
        if self.total == 0 {
            return "No approved services are configured.".to_owned();
        }
        format!(
            "{} approved services: {} active, {} failed, {} inactive, {} unavailable and {} enabled.",
            self.total, self.active, self.failed, self.inactive, self.unavailable, self.enabled
        )
    }
}

pub fn summarize_services(services: &[ServiceRecord]) -> ServiceSummary {
    let mut summary = ServiceSummary {
        total: services.len(),
        ..ServiceSummary::default()
    };
    for service in services {
        if service.load_state == "not-found" || service.load_state == "error" {
            summary.unavailable += 1;
        }
        match service.active_state.as_str() {
            "active" | "activating" | "reloading" => summary.active += 1,
            "failed" => summary.failed += 1,
            _ => summary.inactive += 1,
        }
        if matches!(
            service.unit_file_state.as_str(),
            "enabled" | "enabled-runtime" | "static" | "indirect"
        ) {
            summary.enabled += 1;
        }
    }
    summary
}

pub fn service_state_label(service: &ServiceRecord) -> &str {
    if service.load_state == "not-found" || service.load_state == "error" {
        return "Unavailable";
    }
    match service.active_state.as_str() {
        "active" => "Active",
        "activating" => "Starting",
        "deactivating" => "Stopping",
        "reloading" => "Reloading",
        "failed" => "Failed",
        "inactive" => "Inactive",
        _ => "Unknown",
    }
}

pub fn service_state_css_class(service: &ServiceRecord) -> &'static str {
    match service_state_label(service) {
        "Active" => "success",
        "Starting" | "Stopping" | "Reloading" => "accent",
        "Failed" | "Unavailable" => "error",
        _ => "dim-label",
    }
}

#[cfg(test)]
mod tests {
    use genixbit_package_model::ServiceRecord;

    use super::{ServiceSummary, service_state_label, summarize_services};

    fn service(load: &str, active: &str, unit_file: &str) -> ServiceRecord {
        ServiceRecord {
            name: "example.service".to_owned(),
            description: "Example".to_owned(),
            load_state: load.to_owned(),
            active_state: active.to_owned(),
            sub_state: "running".to_owned(),
            unit_file_state: unit_file.to_owned(),
        }
    }

    #[test]
    fn summarizes_approved_service_states() {
        let services = [
            service("loaded", "active", "enabled"),
            service("loaded", "failed", "disabled"),
            service("loaded", "inactive", "static"),
            service("not-found", "inactive", "disabled"),
        ];
        assert_eq!(
            summarize_services(&services),
            ServiceSummary {
                total: 4,
                active: 1,
                failed: 1,
                inactive: 2,
                unavailable: 1,
                enabled: 2,
            }
        );
    }

    #[test]
    fn formats_stable_summary_text() {
        assert_eq!(
            ServiceSummary {
                total: 2,
                active: 1,
                failed: 0,
                inactive: 1,
                unavailable: 0,
                enabled: 1,
            }
            .status_text(),
            "2 approved services: 1 active, 0 failed, 1 inactive, 0 unavailable and 1 enabled."
        );
    }

    #[test]
    fn labels_unavailable_before_active_state() {
        assert_eq!(
            service_state_label(&service("not-found", "inactive", "disabled")),
            "Unavailable"
        );
        assert_eq!(
            service_state_label(&service("loaded", "failed", "disabled")),
            "Failed"
        );
        assert_eq!(
            service_state_label(&service("loaded", "active", "enabled")),
            "Active"
        );
    }
}
