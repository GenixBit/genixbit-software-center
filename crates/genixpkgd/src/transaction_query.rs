use anyhow::bail;
use genixbit_package_model::TransactionRecord;

const MAX_RECENT_TRANSACTIONS: u64 = 500;

pub fn recent_transactions(
    records: impl IntoIterator<Item = TransactionRecord>,
    limit: u64,
) -> anyhow::Result<Vec<TransactionRecord>> {
    if limit == 0 || limit > MAX_RECENT_TRANSACTIONS {
        bail!("recent transaction limit must be between 1 and {MAX_RECENT_TRANSACTIONS}");
    }

    let mut records = records.into_iter().collect::<Vec<_>>();
    records.sort_by(|left, right| {
        right
            .updated_unix_ms
            .cmp(&left.updated_unix_ms)
            .then_with(|| right.id.cmp(&left.id))
    });
    records.truncate(limit as usize);
    Ok(records)
}

#[cfg(test)]
mod tests {
    use genixbit_package_model::TransactionRecord;

    use super::recent_transactions;

    fn record(id: u64, updated_unix_ms: u64) -> TransactionRecord {
        TransactionRecord {
            id,
            preview_id: id,
            kind: "install".to_owned(),
            package: format!("package-{id}"),
            state: "completed".to_owned(),
            progress_basis_points: 10_000,
            can_cancel: false,
            created_unix_ms: 1,
            updated_unix_ms,
            message: "Completed".to_owned(),
        }
    }

    #[test]
    fn returns_newest_records_first_with_a_stable_id_tiebreaker() {
        let result = recent_transactions(
            [record(1, 100), record(2, 200), record(3, 200)],
            3,
        )
        .expect("records should sort");
        assert_eq!(
            result.iter().map(|record| record.id).collect::<Vec<_>>(),
            [3, 2, 1]
        );
    }

    #[test]
    fn applies_the_requested_limit() {
        let result = recent_transactions([record(1, 100), record(2, 200)], 1)
            .expect("records should load");
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].id, 2);
    }

    #[test]
    fn rejects_unbounded_or_empty_queries() {
        assert!(recent_transactions(Vec::<TransactionRecord>::new(), 0).is_err());
        assert!(recent_transactions(Vec::<TransactionRecord>::new(), 501).is_err());
    }
}
