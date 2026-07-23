use std::collections::BTreeSet;

use genixbit_package_model::UpdateRecord;

pub const ALL_SECURITY_SOURCES: &str = "All sources";

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct SecuritySummary {
    pub total_updates: usize,
    pub security_updates: usize,
    pub sources: Vec<String>,
}

impl SecuritySummary {
    pub fn status_text(&self) -> String {
        if self.security_updates == 0 {
            return format!(
                "No security updates are currently reported. {} total package updates remain available.",
                self.total_updates
            );
        }

        let source_text = if self.sources.is_empty() {
            "repository source not reported".to_owned()
        } else {
            self.sources.join(", ")
        };
        format!(
            "{} security updates are available from {}. {} total package updates are available.",
            self.security_updates, source_text, self.total_updates
        )
    }
}

pub fn security_updates(updates: &[UpdateRecord]) -> Vec<&UpdateRecord> {
    updates.iter().filter(|update| update.security).collect()
}

pub fn security_filters_active(query: &str, source: &str) -> bool {
    !query.trim().is_empty() || (!source.is_empty() && source != ALL_SECURITY_SOURCES)
}

pub fn filter_security_updates<'a>(
    updates: &'a [UpdateRecord],
    query: &str,
    source: &str,
) -> Vec<&'a UpdateRecord> {
    let query = query.trim().to_ascii_lowercase();

    updates
        .iter()
        .filter(|update| update.security)
        .filter(|update| {
            source.is_empty()
                || source == ALL_SECURITY_SOURCES
                || update.source.eq_ignore_ascii_case(source)
        })
        .filter(|update| {
            query.is_empty()
                || update.name.to_ascii_lowercase().contains(&query)
                || update.current_version.to_ascii_lowercase().contains(&query)
                || update
                    .candidate_version
                    .to_ascii_lowercase()
                    .contains(&query)
                || update.architecture.to_ascii_lowercase().contains(&query)
                || update.source.to_ascii_lowercase().contains(&query)
        })
        .collect()
}

pub fn summarize_security(updates: &[UpdateRecord]) -> SecuritySummary {
    let security = security_updates(updates);
    let sources = security
        .iter()
        .map(|update| update.source.trim())
        .filter(|source| !source.is_empty())
        .map(ToOwned::to_owned)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();

    SecuritySummary {
        total_updates: updates.len(),
        security_updates: security.len(),
        sources,
    }
}

#[cfg(test)]
mod tests {
    use genixbit_package_model::UpdateRecord;

    use super::{
        ALL_SECURITY_SOURCES, SecuritySummary, filter_security_updates, security_filters_active,
        security_updates, summarize_security,
    };

    fn update(name: &str, source: &str, security: bool) -> UpdateRecord {
        UpdateRecord {
            name: name.to_owned(),
            current_version: "1".to_owned(),
            candidate_version: "2".to_owned(),
            architecture: "amd64".to_owned(),
            source: source.to_owned(),
            security,
        }
    }

    #[test]
    fn returns_only_security_updates_in_input_order() {
        let updates = [
            update("curl", "Ubuntu-Security", true),
            update("nano", "Ubuntu-Updates", false),
            update("openssl", "Ubuntu-Security", true),
        ];

        assert_eq!(security_updates(&updates), [&updates[0], &updates[2]]);
    }

    #[test]
    fn detects_query_and_source_filter_state() {
        assert!(!security_filters_active("", ALL_SECURITY_SOURCES));
        assert!(!security_filters_active("   ", ""));
        assert!(security_filters_active("curl", ALL_SECURITY_SOURCES));
        assert!(security_filters_active("", "Ubuntu-Security"));
    }

    #[test]
    fn filters_security_updates_by_query_and_source() {
        let updates = [
            update("curl", "security-a", true),
            update("openssl", "security-b", true),
            update("nano", "security-a", false),
        ];

        assert_eq!(
            filter_security_updates(&updates, "open", ALL_SECURITY_SOURCES),
            [&updates[1]]
        );
        assert_eq!(
            filter_security_updates(&updates, "", "security-a"),
            [&updates[0]]
        );
        assert_eq!(
            filter_security_updates(&updates, "curl", "security-b"),
            Vec::<&UpdateRecord>::new()
        );
    }

    #[test]
    fn summarizes_unique_sorted_sources() {
        let updates = [
            update("curl", "security-b", true),
            update("git", "security-a", true),
            update("nano", "security-b", true),
            update("vim", "updates", false),
        ];

        assert_eq!(
            summarize_security(&updates),
            SecuritySummary {
                total_updates: 4,
                security_updates: 3,
                sources: vec!["security-a".to_owned(), "security-b".to_owned()],
            }
        );
    }

    #[test]
    fn formats_clear_and_actionable_status_text() {
        let none = SecuritySummary {
            total_updates: 2,
            security_updates: 0,
            sources: Vec::new(),
        };
        assert_eq!(
            none.status_text(),
            "No security updates are currently reported. 2 total package updates remain available."
        );

        let available = SecuritySummary {
            total_updates: 5,
            security_updates: 2,
            sources: vec!["security".to_owned()],
        };
        assert_eq!(
            available.status_text(),
            "2 security updates are available from security. 5 total package updates are available."
        );
    }
}
