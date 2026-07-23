use genixbit_package_model::ServiceRecord;

pub const ALL_SERVICE_STATES: &str = "All states";

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

pub fn service_filters_active(query: &str, state: &str) -> bool {
    !query.trim().is_empty() || (!state.is_empty() && state != ALL_SERVICE_STATES)
}

pub fn filter_services<'a>(
    services: &'a [ServiceRecord],
    query: &str,
    state: &str,
) -> Vec<&'a ServiceRecord> {
    let query = query.trim().to_ascii_lowercase();
    services
        .iter()
        .filter(|service| {
            query.is_empty()
                || service.name.to_ascii_lowercase().contains(&query)
                || service.description.to_ascii_lowercase().contains(&query)
                || service.load_state.to_ascii_lowercase().contains(&query)
                || service.active_state.to_ascii_lowercase().contains(&query)
                || service.sub_state.to_ascii_lowercase().contains(&query)
                || service
                    .unit_file_state
                    .to_ascii_lowercase()
                    .contains(&query)
        })
        .filter(|service| {
            if state.is_empty() || state == ALL_SERVICE_STATES {
                return true;
            }
            let label = service_state_label(service);
            if state == "Transitional" {
                matches!(label, "Starting" | "Stopping" | "Reloading")
            } else {
                label.eq_ignore_ascii_case(state)
            }
        })
        .collect()
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

    use super::{
        ALL_SERVICE_STATES, ServiceSummary, filter_services, service_filters_active,
        service_state_label, summarize_services,
    };

    fn service(name: &str, load: &str, active: &str, unit_file: &str) -> ServiceRecord {
        ServiceRecord {
            name: name.to_owned(),
            description: format!("{name} description"),
            load_state: load.to_owned(),
            active_state: active.to_owned(),
            sub_state: "running".to_owned(),
            unit_file_state: unit_file.to_owned(),
        }
    }

    #[test]
    fn summarizes_approved_service_states() {
        let services = [
            service("one.service", "loaded", "active", "enabled"),
            service("two.service", "loaded", "failed", "disabled"),
            service("three.service", "loaded", "inactive", "static"),
            service("four.service", "not-found", "inactive", "disabled"),
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
    fn detects_active_filters() {
        assert!(!service_filters_active("", ALL_SERVICE_STATES));
        assert!(!service_filters_active("  ", ""));
        assert!(service_filters_active("genix", ALL_SERVICE_STATES));
        assert!(service_filters_active("", "Failed"));
    }

    #[test]
    fn filters_by_query_and_user_facing_state() {
        let services = [
            service("genixpkgd.service", "loaded", "active", "enabled"),
            service("worker.service", "loaded", "failed", "disabled"),
            service("missing.service", "not-found", "inactive", "disabled"),
            service("starting.service", "loaded", "activating", "enabled"),
        ];
        assert_eq!(
            filter_services(&services, "genix", ALL_SERVICE_STATES),
            [&services[0]]
        );
        assert_eq!(filter_services(&services, "", "Failed"), [&services[1]]);
        assert_eq!(
            filter_services(&services, "", "Unavailable"),
            [&services[2]]
        );
        assert_eq!(
            filter_services(&services, "", "Transitional"),
            [&services[3]]
        );
    }

    #[test]
    fn labels_unavailable_before_active_state() {
        assert_eq!(
            service_state_label(&service(
                "missing.service",
                "not-found",
                "inactive",
                "disabled"
            )),
            "Unavailable"
        );
        assert_eq!(
            service_state_label(&service("failed.service", "loaded", "failed", "disabled")),
            "Failed"
        );
        assert_eq!(
            service_state_label(&service("active.service", "loaded", "active", "enabled")),
            "Active"
        );
    }
}
