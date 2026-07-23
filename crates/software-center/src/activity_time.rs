use std::time::{SystemTime, UNIX_EPOCH};

use genixbit_package_model::TransactionRecord;

pub fn current_unix_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis().min(u128::from(u64::MAX)) as u64)
        .unwrap_or_default()
}

pub fn timing_text(record: &TransactionRecord, now_unix_ms: u64) -> String {
    let updated_age_ms = now_unix_ms.saturating_sub(record.updated_unix_ms);
    let duration_ms = record.updated_unix_ms.saturating_sub(record.created_unix_ms);
    format!(
        "Updated {} ago · Duration {}",
        format_duration(updated_age_ms),
        format_duration(duration_ms)
    )
}

pub fn format_duration(duration_ms: u64) -> String {
    const SECOND: u64 = 1_000;
    const MINUTE: u64 = 60 * SECOND;
    const HOUR: u64 = 60 * MINUTE;
    const DAY: u64 = 24 * HOUR;

    match duration_ms {
        0..SECOND => "<1s".to_owned(),
        SECOND..MINUTE => format!("{}s", duration_ms / SECOND),
        MINUTE..HOUR => format!("{}m", duration_ms / MINUTE),
        HOUR..DAY => format!("{}h", duration_ms / HOUR),
        _ => format!("{}d", duration_ms / DAY),
    }
}

#[cfg(test)]
mod tests {
    use genixbit_package_model::TransactionRecord;

    use super::{format_duration, timing_text};

    #[test]
    fn formats_elapsed_ranges_compactly() {
        assert_eq!(format_duration(0), "<1s");
        assert_eq!(format_duration(999), "<1s");
        assert_eq!(format_duration(12_999), "12s");
        assert_eq!(format_duration(5 * 60_000), "5m");
        assert_eq!(format_duration(3 * 3_600_000), "3h");
        assert_eq!(format_duration(2 * 86_400_000), "2d");
    }

    #[test]
    fn formats_update_age_and_transaction_duration() {
        let record = TransactionRecord {
            created_unix_ms: 1_000,
            updated_unix_ms: 121_000,
            ..TransactionRecord::default()
        };

        assert_eq!(
            timing_text(&record, 3_721_000),
            "Updated 1h ago · Duration 2m"
        );
    }

    #[test]
    fn future_or_reversed_timestamps_fail_closed() {
        let record = TransactionRecord {
            created_unix_ms: 10_000,
            updated_unix_ms: 5_000,
            ..TransactionRecord::default()
        };

        assert_eq!(
            timing_text(&record, 1_000),
            "Updated <1s ago · Duration <1s"
        );
    }
}
