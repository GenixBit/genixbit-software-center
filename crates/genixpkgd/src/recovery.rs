use std::collections::HashMap;

use genixbit_package_model::TransactionRecord;

const STATE_QUEUED: &str = "queued";
const STATE_RUNNING: &str = "running";
const STATE_INTERRUPTED: &str = "interrupted";

#[derive(Debug, Default)]
pub struct RecoveredTransactions {
    pub records: HashMap<u64, TransactionRecord>,
    pub interrupted: Vec<TransactionRecord>,
    pub next_transaction_id: u64,
    pub next_preview_id: u64,
}

pub fn recover_transactions(
    journal_records: Vec<TransactionRecord>,
    recovered_unix_ms: u64,
) -> RecoveredTransactions {
    let mut records = HashMap::new();
    let mut maximum_transaction_id = 0;
    let mut maximum_preview_id = 0;

    for record in journal_records {
        maximum_transaction_id = maximum_transaction_id.max(record.id);
        maximum_preview_id = maximum_preview_id.max(record.preview_id);
        records.insert(record.id, record);
    }

    let mut interrupted = records
        .values_mut()
        .filter(|record| matches!(record.state.as_str(), STATE_QUEUED | STATE_RUNNING))
        .map(|record| {
            record.state = STATE_INTERRUPTED.to_owned();
            record.can_cancel = false;
            record.updated_unix_ms = recovered_unix_ms;
            record.message =
                "Interrupted by daemon restart; create and review a new transaction preview"
                    .to_owned();
            record.clone()
        })
        .collect::<Vec<_>>();
    interrupted.sort_by_key(|record| record.id);

    RecoveredTransactions {
        records,
        interrupted,
        next_transaction_id: maximum_transaction_id.saturating_add(1).max(1),
        next_preview_id: maximum_preview_id.saturating_add(1).max(1),
    }
}

#[cfg(test)]
mod tests {
    use genixbit_package_model::TransactionRecord;

    use super::recover_transactions;

    fn record(id: u64, preview_id: u64, state: &str) -> TransactionRecord {
        TransactionRecord {
            id,
            preview_id,
            kind: "install".to_owned(),
            package: "curl".to_owned(),
            state: state.to_owned(),
            progress_basis_points: 0,
            can_cancel: true,
            created_unix_ms: 100,
            updated_unix_ms: 100,
            message: state.to_owned(),
        }
    }

    #[test]
    fn keeps_only_the_latest_record_for_each_transaction() {
        let recovered = recover_transactions(
            vec![record(7, 3, "queued"), record(7, 3, "completed")],
            500,
        );
        assert_eq!(recovered.records[&7].state, "completed");
        assert!(recovered.interrupted.is_empty());
        assert_eq!(recovered.next_transaction_id, 8);
        assert_eq!(recovered.next_preview_id, 4);
    }

    #[test]
    fn marks_queued_and_running_records_interrupted() {
        let recovered = recover_transactions(
            vec![record(2, 4, "queued"), record(3, 5, "running")],
            900,
        );
        assert_eq!(recovered.interrupted.len(), 2);
        for record in recovered.records.values() {
            assert_eq!(record.state, "interrupted");
            assert!(!record.can_cancel);
            assert_eq!(record.updated_unix_ms, 900);
        }
    }

    #[test]
    fn starts_identifiers_at_one_for_an_empty_journal() {
        let recovered = recover_transactions(Vec::new(), 100);
        assert_eq!(recovered.next_transaction_id, 1);
        assert_eq!(recovered.next_preview_id, 1);
    }
}
