use genixbit_package_model::TransactionRecord;

/// User-facing selector value that disables operation filtering.
pub const ALL_OPERATIONS: &str = "All operations";
/// User-facing selector value that disables state filtering.
pub const ALL_STATES: &str = "All states";

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ActivitySummary {
    pub total: usize,
    pub active: usize,
    pub completed: usize,
    pub failed: usize,
    pub cancelled: usize,
    pub interrupted: usize,
}

impl ActivitySummary {
    pub fn status_text(self) -> String {
        format!(
            "{} recorded · {} active · {} completed · {} failed · {} cancelled · {} interrupted",
            self.total,
            self.active,
            self.completed,
            self.failed,
            self.cancelled,
            self.interrupted
        )
    }
}

/// Returns matching records without changing their existing newest-first order.
pub fn filter_records<'a>(
    records: &'a [TransactionRecord],
    query: &str,
    operation: &str,
    state: &str,
) -> Vec<&'a TransactionRecord> {
    let query = query.trim().to_ascii_lowercase();

    records
        .iter()
        .filter(|record| {
            let operation_matches = operation.is_empty()
                || operation == ALL_OPERATIONS
                || operation.eq_ignore_ascii_case(&record.kind);
            let state_matches = state.is_empty()
                || state == ALL_STATES
                || state.eq_ignore_ascii_case(&record.state);
            let query_matches = query.is_empty()
                || record.package.to_ascii_lowercase().contains(&query)
                || record.message.to_ascii_lowercase().contains(&query)
                || record.kind.to_ascii_lowercase().contains(&query)
                || record.state.to_ascii_lowercase().contains(&query)
                || record.id.to_string().contains(&query);

            operation_matches && state_matches && query_matches
        })
        .collect()
}

pub fn summarize_records(records: &[TransactionRecord]) -> ActivitySummary {
    let mut summary = ActivitySummary {
        total: records.len(),
        ..ActivitySummary::default()
    };

    for record in records {
        match record.state.as_str() {
            "queued" | "running" => summary.active += 1,
            "completed" => summary.completed += 1,
            "failed" => summary.failed += 1,
            "cancelled" => summary.cancelled += 1,
            "interrupted" => summary.interrupted += 1,
            _ => {}
        }
    }

    summary
}

#[cfg(test)]
mod tests {
    use genixbit_package_model::TransactionRecord;

    use super::{
        ALL_OPERATIONS, ALL_STATES, ActivitySummary, filter_records, summarize_records,
    };

    fn record(id: u64, kind: &str, package: &str, state: &str, message: &str) -> TransactionRecord {
        TransactionRecord {
            id,
            preview_id: id,
            kind: kind.to_owned(),
            package: package.to_owned(),
            state: state.to_owned(),
            progress_basis_points: 0,
            can_cancel: false,
            created_unix_ms: 100,
            updated_unix_ms: 100,
            message: message.to_owned(),
        }
    }

    #[test]
    fn filters_by_package_message_and_transaction_id() {
        let records = [
            record(41, "install", "curl", "completed", "Simulation completed"),
            record(42, "remove", "nano", "failed", "APT simulation failed"),
        ];

        assert_eq!(
            filter_records(&records, "curl", ALL_OPERATIONS, ALL_STATES),
            [&records[0]]
        );
        assert_eq!(
            filter_records(&records, "simulation failed", ALL_OPERATIONS, ALL_STATES),
            [&records[1]]
        );
        assert_eq!(
            filter_records(&records, "42", ALL_OPERATIONS, ALL_STATES),
            [&records[1]]
        );
    }

    #[test]
    fn combines_operation_and_state_filters() {
        let records = [
            record(1, "install", "curl", "completed", "done"),
            record(2, "install", "git", "failed", "failed"),
            record(3, "remove", "nano", "completed", "done"),
        ];

        assert_eq!(
            filter_records(&records, "", "install", "completed"),
            [&records[0]]
        );
        assert_eq!(
            filter_records(&records, "", "remove", ALL_STATES),
            [&records[2]]
        );
    }

    #[test]
    fn all_filters_preserve_input_order() {
        let records = [
            record(3, "upgrade", "curl", "running", "running"),
            record(2, "remove", "nano", "cancelled", "cancelled"),
            record(1, "install", "git", "completed", "completed"),
        ];

        assert_eq!(
            filter_records(&records, "", ALL_OPERATIONS, ALL_STATES),
            records.iter().collect::<Vec<_>>()
        );
    }

    #[test]
    fn summarizes_known_transaction_states() {
        let records = [
            record(1, "install", "a", "queued", "queued"),
            record(2, "install", "b", "running", "running"),
            record(3, "install", "c", "completed", "completed"),
            record(4, "install", "d", "failed", "failed"),
            record(5, "install", "e", "cancelled", "cancelled"),
            record(6, "install", "f", "interrupted", "interrupted"),
            record(7, "install", "g", "unknown", "unknown"),
        ];

        assert_eq!(
            summarize_records(&records),
            ActivitySummary {
                total: 7,
                active: 2,
                completed: 1,
                failed: 1,
                cancelled: 1,
                interrupted: 1,
            }
        );
    }

    #[test]
    fn formats_summary_status_text() {
        let summary = ActivitySummary {
            total: 12,
            active: 2,
            completed: 5,
            failed: 2,
            cancelled: 1,
            interrupted: 2,
        };

        assert_eq!(
            summary.status_text(),
            "12 recorded · 2 active · 5 completed · 2 failed · 1 cancelled · 2 interrupted"
        );
    }
}
