use std::collections::BTreeSet;

use genixbit_package_model::UpdateRecord;

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

    use super::{SecuritySummary, security_updates, summarize_security};

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
