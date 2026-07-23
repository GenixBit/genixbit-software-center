use genixbit_package_model::UpdateRecord;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SecurityAdvisory {
    pub id: String,
    pub title: String,
    pub package: String,
    pub current_version: String,
    pub candidate_version: String,
    pub architecture: String,
    pub source: String,
    pub coverage_note: String,
}

pub fn advisory_for_update(update: &UpdateRecord) -> Option<SecurityAdvisory> {
    if !update.security {
        return None;
    }

    let source = if update.source.trim().is_empty() {
        "unreported-source"
    } else {
        update.source.trim()
    };
    let id = format!(
        "APT-{}-{}",
        stable_component(source),
        stable_component(&update.name)
    );

    Some(SecurityAdvisory {
        id,
        title: format!("Security update for {}", update.name),
        package: update.name.clone(),
        current_version: update.current_version.clone(),
        candidate_version: update.candidate_version.clone(),
        architecture: update.architecture.clone(),
        source: update.source.clone(),
        coverage_note: "Derived from local APT security metadata. CVE identifiers, severity scoring and vendor bulletin text are not available offline.".to_owned(),
    })
}

pub fn security_advisories(updates: &[UpdateRecord]) -> Vec<SecurityAdvisory> {
    updates.iter().filter_map(advisory_for_update).collect()
}

fn stable_component(value: &str) -> String {
    let mut output = String::new();
    let mut last_was_dash = false;
    for character in value.chars() {
        if character.is_ascii_alphanumeric() {
            output.push(character.to_ascii_uppercase());
            last_was_dash = false;
        } else if !last_was_dash && !output.is_empty() {
            output.push('-');
            last_was_dash = true;
        }
    }
    while output.ends_with('-') {
        output.pop();
    }
    if output.is_empty() {
        "UNKNOWN".to_owned()
    } else {
        output
    }
}

#[cfg(test)]
mod tests {
    use genixbit_package_model::UpdateRecord;

    use super::{advisory_for_update, security_advisories};

    fn update(name: &str, source: &str, security: bool) -> UpdateRecord {
        UpdateRecord {
            name: name.to_owned(),
            current_version: "1.0".to_owned(),
            candidate_version: "1.1".to_owned(),
            architecture: "amd64".to_owned(),
            source: source.to_owned(),
            security,
        }
    }

    #[test]
    fn creates_stable_local_advisory_ids() {
        let advisory = advisory_for_update(&update("openssl", "Ubuntu Security", true))
            .expect("security advisory");
        assert_eq!(advisory.id, "APT-UBUNTU-SECURITY-OPENSSL");
        assert_eq!(advisory.title, "Security update for openssl");
    }

    #[test]
    fn ignores_non_security_updates_and_preserves_order() {
        let updates = [
            update("curl", "security", true),
            update("nano", "updates", false),
            update("openssl", "security", true),
        ];
        let advisories = security_advisories(&updates);
        assert_eq!(advisories.len(), 2);
        assert_eq!(advisories[0].package, "curl");
        assert_eq!(advisories[1].package, "openssl");
    }

    #[test]
    fn explains_offline_coverage_limits() {
        let advisory = advisory_for_update(&update("curl", "", true)).expect("advisory");
        assert_eq!(advisory.id, "APT-UNREPORTED-SOURCE-CURL");
        assert!(advisory.coverage_note.contains("CVE identifiers"));
        assert!(advisory.coverage_note.contains("offline"));
    }
}
