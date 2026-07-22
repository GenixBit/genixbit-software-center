use std::{
    collections::{HashMap, VecDeque},
    sync::{
        Mutex,
        atomic::{AtomicU64, Ordering},
    },
    time::{SystemTime, UNIX_EPOCH},
};

use anyhow::{Context, bail};
use genixbit_package_model::{
    TransactionChange, TransactionPreview, TransactionQueueSnapshot, TransactionRecord,
};

use crate::journal::TransactionJournal;

const KIND_INSTALL: &str = "install";
const KIND_REMOVE: &str = "remove";
const KIND_UPGRADE: &str = "upgrade";
const STATE_QUEUED: &str = "queued";
const STATE_CANCELLED: &str = "cancelled";

#[derive(Debug, Default)]
struct ManagerState {
    previews: HashMap<u64, TransactionPreview>,
    records: HashMap<u64, TransactionRecord>,
    queue: VecDeque<u64>,
}

#[derive(Debug)]
pub struct TransactionManager {
    next_preview_id: AtomicU64,
    next_transaction_id: AtomicU64,
    state: Mutex<ManagerState>,
    journal: TransactionJournal,
}

impl TransactionManager {
    pub fn new(journal: TransactionJournal) -> Self {
        Self {
            next_preview_id: AtomicU64::new(1),
            next_transaction_id: AtomicU64::new(1),
            state: Mutex::new(ManagerState::default()),
            journal,
        }
    }

    pub fn create_preview(
        &self,
        kind: &str,
        package: &str,
        current_version: &str,
        candidate_version: &str,
        summary: String,
    ) -> anyhow::Result<TransactionPreview> {
        validate_kind(kind)?;
        let preview = TransactionPreview {
            id: self.next_preview_id.fetch_add(1, Ordering::Relaxed),
            kind: kind.to_owned(),
            package: package.to_owned(),
            changes: vec![TransactionChange {
                package: package.to_owned(),
                action: kind.to_owned(),
                current_version: normalize_version(current_version),
                candidate_version: normalize_version(candidate_version),
            }],
            download_size_bytes: 0,
            installed_size_delta_bytes: 0,
            requires_reboot: false,
            ready: true,
            summary,
        };

        self.state
            .lock()
            .map_err(|_| anyhow::anyhow!("transaction manager lock was poisoned"))?
            .previews
            .insert(preview.id, preview.clone());
        Ok(preview)
    }

    pub fn queue_preview(&self, preview_id: u64) -> anyhow::Result<TransactionRecord> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| anyhow::anyhow!("transaction manager lock was poisoned"))?;
        let preview = state
            .previews
            .get(&preview_id)
            .cloned()
            .with_context(|| format!("transaction preview {preview_id} was not found"))?;
        if !preview.ready {
            bail!("transaction preview {preview_id} is not ready");
        }

        let timestamp = now_unix_ms();
        let record = TransactionRecord {
            id: self.next_transaction_id.fetch_add(1, Ordering::Relaxed),
            preview_id,
            kind: preview.kind,
            package: preview.package,
            state: STATE_QUEUED.to_owned(),
            progress_basis_points: 0,
            can_cancel: true,
            created_unix_ms: timestamp,
            updated_unix_ms: timestamp,
            message: "Queued safely; package execution is disabled in this milestone".to_owned(),
        };

        self.journal.append(&record)?;
        state.queue.push_back(record.id);
        state.records.insert(record.id, record.clone());
        Ok(record)
    }

    pub fn cancel(&self, transaction_id: u64) -> anyhow::Result<TransactionRecord> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| anyhow::anyhow!("transaction manager lock was poisoned"))?;
        let current = state
            .records
            .get(&transaction_id)
            .cloned()
            .with_context(|| format!("transaction {transaction_id} was not found"))?;
        if current.state != STATE_QUEUED || !current.can_cancel {
            bail!("transaction {transaction_id} can no longer be cancelled");
        }

        let mut cancelled = current;
        cancelled.state = STATE_CANCELLED.to_owned();
        cancelled.can_cancel = false;
        cancelled.updated_unix_ms = now_unix_ms();
        cancelled.message = "Cancelled before package execution".to_owned();

        self.journal.append(&cancelled)?;
        state.queue.retain(|id| *id != transaction_id);
        state.records.insert(transaction_id, cancelled.clone());
        Ok(cancelled)
    }

    pub fn snapshot(&self) -> anyhow::Result<TransactionQueueSnapshot> {
        let state = self
            .state
            .lock()
            .map_err(|_| anyhow::anyhow!("transaction manager lock was poisoned"))?;
        let queued = state
            .queue
            .iter()
            .filter_map(|id| state.records.get(id).cloned())
            .collect();
        Ok(TransactionQueueSnapshot {
            has_active: false,
            active: TransactionRecord::default(),
            queued,
        })
    }

    pub fn journal(&self) -> anyhow::Result<Vec<TransactionRecord>> {
        self.journal.read_all()
    }

    pub fn journal_path(&self) -> &std::path::Path {
        self.journal.path()
    }
}

fn validate_kind(kind: &str) -> anyhow::Result<()> {
    if matches!(kind, KIND_INSTALL | KIND_REMOVE | KIND_UPGRADE) {
        Ok(())
    } else {
        bail!("unsupported transaction kind {kind}")
    }
}

fn normalize_version(version: &str) -> String {
    match version.trim() {
        "" | "(none)" => String::new(),
        value => value.to_owned(),
    }
}

fn now_unix_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis().try_into().unwrap_or(u64::MAX))
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use std::{fs, path::PathBuf, process, time::SystemTime};

    use crate::journal::TransactionJournal;

    use super::TransactionManager;

    fn manager() -> (TransactionManager, PathBuf) {
        let unique = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .expect("clock should be after the Unix epoch")
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "genixpkgd-transaction-test-{}-{unique}.log",
            process::id()
        ));
        (
            TransactionManager::new(TransactionJournal::new(path.clone())),
            path,
        )
    }

    #[test]
    fn queues_previews_in_serial_order() {
        let (manager, path) = manager();
        let first = manager
            .create_preview("install", "curl", "", "8.0", "Install curl".to_owned())
            .expect("preview should be created");
        let second = manager
            .create_preview("remove", "nano", "7.2", "", "Remove nano".to_owned())
            .expect("preview should be created");

        let first_record = manager
            .queue_preview(first.id)
            .expect("first preview should queue");
        let second_record = manager
            .queue_preview(second.id)
            .expect("second preview should queue");
        let snapshot = manager.snapshot().expect("snapshot should load");

        assert!(!snapshot.has_active);
        assert_eq!(snapshot.queued.len(), 2);
        assert_eq!(snapshot.queued[0].id, first_record.id);
        assert_eq!(snapshot.queued[1].id, second_record.id);
        fs::remove_file(path).expect("test journal should be removable");
    }

    #[test]
    fn cancellation_is_limited_to_queued_transactions() {
        let (manager, path) = manager();
        let preview = manager
            .create_preview("upgrade", "curl", "7.0", "8.0", "Upgrade curl".to_owned())
            .expect("preview should be created");
        let record = manager
            .queue_preview(preview.id)
            .expect("preview should queue");
        let cancelled = manager.cancel(record.id).expect("queue item should cancel");

        assert_eq!(cancelled.state, "cancelled");
        assert!(!cancelled.can_cancel);
        assert!(
            manager
                .snapshot()
                .expect("snapshot should load")
                .queued
                .is_empty()
        );
        assert!(manager.cancel(record.id).is_err());
        fs::remove_file(path).expect("test journal should be removable");
    }
}
