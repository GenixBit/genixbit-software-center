use genixbit_package_model::TransactionRecord;

pub const ALL_OPERATIONS: &str = "All operations";
pub const ALL_STATES: &str = "All states";

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
            let state_matches =
                state.is_empty() || state == ALL_STATES || state.eq_ignore_ascii_case(&record.state);
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

#[cfg(test)]
mod tests {
    use genixbit_package_model::TransactionRecord;

    use super::{ALL_OPERATIONS, ALL_STATES, filter_records};

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
}
