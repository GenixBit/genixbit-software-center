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
    TransactionEvent, TransactionPreview, TransactionQueueSnapshot, TransactionRecord,
};

use crate::journal::TransactionJournal;

const KIND_INSTALL: &str = "install";
const KIND_REMOVE: &str = "remove";
const KIND_UPGRADE: &str = "upgrade";
const STATE_PREVIEWED: &str = "previewed";
const STATE_QUEUED: &str = "queued";
const STATE_RUNNING: &str = "running";
const STATE_COMPLETED: &str = "completed";
const STATE_CANCELLED: &str = "cancelled";
const MAX_EVENT_HISTORY: usize = 500;

#[derive(Debug, Default)]
struct ManagerState {
    previews: HashMap<u64, TransactionPreview>,
    records: HashMap<u64, TransactionRecord>,
    queue: VecDeque<u64>,
    active: Option<u64>,
    events: VecDeque<TransactionEvent>,
}

#[derive(Debug)]
pub struct TransactionManager {
    next_preview_id: AtomicU64,
    next_transaction_id: AtomicU64,
    next_event_sequence: AtomicU64,
    state: Mutex<ManagerState>,
    journal: TransactionJournal,
}

impl TransactionManager {
    pub fn new(journal: TransactionJournal) -> Self {
        Self {
            next_preview_id: AtomicU64::new(1),
            next_transaction_id: AtomicU64::new(1),
            next_event_sequence: AtomicU64::new(1),
            state: Mutex::new(ManagerState::default()),
            journal,
        }
    }

    pub fn create_preview(
        &self,
        mut preview: TransactionPreview,
    ) -> anyhow::Result<(TransactionPreview, TransactionEvent)> {
        validate_kind(&preview.kind)?;
        if preview.package.is_empty() {
            bail!("transaction preview package cannot be empty");
        }

        preview.id = self.next_preview_id.fetch_add(1, Ordering::Relaxed);
        preview.ready = true;
        let event = self.preview_event(&preview);
        let mut state = self
            .state
            .lock()
            .map_err(|_| anyhow::anyhow!("transaction manager lock was poisoned"))?;
        state.previews.insert(preview.id, preview.clone());
        push_event(&mut state.events, event.clone());
        Ok((preview, event))
    }

    pub fn queue_preview(
        &self,
        preview_id: u64,
    ) -> anyhow::Result<(TransactionRecord, TransactionEvent)> {
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
        let event = self.record_event("queued", &record, "info");

        self.journal.append(&record)?;
        state.queue.push_back(record.id);
        state.records.insert(record.id, record.clone());
        push_event(&mut state.events, event.clone());
        Ok((record, event))
    }

    pub fn begin_next_simulation(&self) -> anyhow::Result<(TransactionRecord, TransactionEvent)> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| anyhow::anyhow!("transaction manager lock was poisoned"))?;
        if state.active.is_some() {
            bail!("a transaction simulation is already active");
        }
        let transaction_id = *state
            .queue
            .front()
            .context("no queued transaction is available for simulation")?;
        let mut record = state
            .records
            .get(&transaction_id)
            .cloned()
            .with_context(|| format!("transaction {transaction_id} was not found"))?;
        record.state = STATE_RUNNING.to_owned();
        record.progress_basis_points = 1_000;
        record.can_cancel = false;
        record.updated_unix_ms = now_unix_ms();
        record.message =
            "Simulation runner started; no package command will be executed".to_owned();
        let event = self.record_event("running", &record, "info");

        self.journal.append(&record)?;
        state.queue.pop_front();
        state.active = Some(transaction_id);
        state.records.insert(transaction_id, record.clone());
        push_event(&mut state.events, event.clone());
        Ok((record, event))
    }

    pub fn update_simulation_progress(
        &self,
        transaction_id: u64,
        progress_basis_points: u32,
        message: &str,
    ) -> anyhow::Result<(TransactionRecord, TransactionEvent)> {
        if !(1_001..10_000).contains(&progress_basis_points) {
            bail!("simulation progress must be between 1001 and 9999 basis points");
        }
        let mut state = self
            .state
            .lock()
            .map_err(|_| anyhow::anyhow!("transaction manager lock was poisoned"))?;
        if state.active != Some(transaction_id) {
            bail!("transaction {transaction_id} is not the active simulation");
        }
        let mut record = state
            .records
            .get(&transaction_id)
            .cloned()
            .with_context(|| format!("transaction {transaction_id} was not found"))?;
        if record.state != STATE_RUNNING {
            bail!("transaction {transaction_id} is not running");
        }
        record.progress_basis_points = progress_basis_points;
        record.updated_unix_ms = now_unix_ms();
        record.message = message.to_owned();
        let event = self.record_event("progress", &record, "info");

        self.journal.append(&record)?;
        state.records.insert(transaction_id, record.clone());
        push_event(&mut state.events, event.clone());
        Ok((record, event))
    }

    pub fn complete_simulation(
        &self,
        transaction_id: u64,
    ) -> anyhow::Result<(TransactionRecord, TransactionEvent)> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| anyhow::anyhow!("transaction manager lock was poisoned"))?;
        if state.active != Some(transaction_id) {
            bail!("transaction {transaction_id} is not the active simulation");
        }
        let mut record = state
            .records
            .get(&transaction_id)
            .cloned()
            .with_context(|| format!("transaction {transaction_id} was not found"))?;
        if record.state != STATE_RUNNING {
            bail!("transaction {transaction_id} is not running");
        }
        record.state = STATE_COMPLETED.to_owned();
        record.progress_basis_points = 10_000;
        record.can_cancel = false;
        record.updated_unix_ms = now_unix_ms();
        record.message = "Simulation completed successfully; no packages were changed".to_owned();
        let event = self.record_event("completed", &record, "info");

        self.journal.append(&record)?;
        state.active = None;
        state.records.insert(transaction_id, record.clone());
        push_event(&mut state.events, event.clone());
        Ok((record, event))
    }

    pub fn cancel(
        &self,
        transaction_id: u64,
    ) -> anyhow::Result<(TransactionRecord, TransactionEvent)> {
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
        let event = self.record_event("cancelled", &cancelled, "info");

        self.journal.append(&cancelled)?;
        state.queue.retain(|id| *id != transaction_id);
        state.records.insert(transaction_id, cancelled.clone());
        push_event(&mut state.events, event.clone());
        Ok((cancelled, event))
    }

    pub fn snapshot(&self) -> anyhow::Result<TransactionQueueSnapshot> {
        let state = self
            .state
            .lock()
            .map_err(|_| anyhow::anyhow!("transaction manager lock was poisoned"))?;
        let active = state
            .active
            .and_then(|id| state.records.get(&id).cloned())
            .unwrap_or_default();
        let queued = state
            .queue
            .iter()
            .filter_map(|id| state.records.get(id).cloned())
            .collect();
        Ok(TransactionQueueSnapshot {
            has_active: state.active.is_some(),
            active,
            queued,
        })
    }

    pub fn events(&self, after_sequence: u64, limit: u64) -> anyhow::Result<Vec<TransactionEvent>> {
        if limit == 0 || limit > MAX_EVENT_HISTORY as u64 {
            bail!("event history limit must be between 1 and {MAX_EVENT_HISTORY}");
        }
        let state = self
            .state
            .lock()
            .map_err(|_| anyhow::anyhow!("transaction manager lock was poisoned"))?;
        Ok(state
            .events
            .iter()
            .filter(|event| event.sequence > after_sequence)
            .take(limit as usize)
            .cloned()
            .collect())
    }

    pub fn journal(&self) -> anyhow::Result<Vec<TransactionRecord>> {
        self.journal.read_all()
    }

    pub fn journal_path(&self) -> &std::path::Path {
        self.journal.path()
    }

    fn preview_event(&self, preview: &TransactionPreview) -> TransactionEvent {
        TransactionEvent {
            sequence: self.next_event_sequence.fetch_add(1, Ordering::Relaxed),
            event: "preview-created".to_owned(),
            transaction_id: 0,
            preview_id: preview.id,
            kind: preview.kind.clone(),
            package: preview.package.clone(),
            state: STATE_PREVIEWED.to_owned(),
            progress_basis_points: 0,
            level: "info".to_owned(),
            message: preview.summary.clone(),
            created_unix_ms: now_unix_ms(),
        }
    }

    fn record_event(
        &self,
        event_name: &str,
        record: &TransactionRecord,
        level: &str,
    ) -> TransactionEvent {
        TransactionEvent {
            sequence: self.next_event_sequence.fetch_add(1, Ordering::Relaxed),
            event: event_name.to_owned(),
            transaction_id: record.id,
            preview_id: record.preview_id,
            kind: record.kind.clone(),
            package: record.package.clone(),
            state: record.state.clone(),
            progress_basis_points: record.progress_basis_points,
            level: level.to_owned(),
            message: record.message.clone(),
            created_unix_ms: record.updated_unix_ms,
        }
    }
}

fn push_event(events: &mut VecDeque<TransactionEvent>, event: TransactionEvent) {
    if events.len() == MAX_EVENT_HISTORY {
        events.pop_front();
    }
    events.push_back(event);
}

fn validate_kind(kind: &str) -> anyhow::Result<()> {
    if matches!(kind, KIND_INSTALL | KIND_REMOVE | KIND_UPGRADE) {
        Ok(())
    } else {
        bail!("unsupported transaction kind {kind}")
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

    use genixbit_package_model::TransactionPreview;

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

    fn preview(kind: &str, package: &str) -> TransactionPreview {
        TransactionPreview {
            kind: kind.to_owned(),
            package: package.to_owned(),
            summary: format!("Preview {kind} {package}"),
            ..TransactionPreview::default()
        }
    }

    #[test]
    fn queues_previews_in_serial_order() {
        let (manager, path) = manager();
        let (first, first_event) = manager
            .create_preview(preview("install", "curl"))
            .expect("preview should be created");
        let (second, second_event) = manager
            .create_preview(preview("remove", "nano"))
            .expect("preview should be created");

        let (first_record, first_queue_event) = manager
            .queue_preview(first.id)
            .expect("first preview should queue");
        let (second_record, second_queue_event) = manager
            .queue_preview(second.id)
            .expect("second preview should queue");
        let snapshot = manager.snapshot().expect("snapshot should load");

        assert!(!snapshot.has_active);
        assert_eq!(snapshot.queued.len(), 2);
        assert_eq!(snapshot.queued[0].id, first_record.id);
        assert_eq!(snapshot.queued[1].id, second_record.id);
        assert!(first_event.sequence < second_event.sequence);
        assert!(second_event.sequence < first_queue_event.sequence);
        assert!(first_queue_event.sequence < second_queue_event.sequence);
        fs::remove_file(path).expect("test journal should be removable");
    }

    #[test]
    fn simulation_runner_processes_one_transaction_at_a_time() {
        let (manager, path) = manager();
        let (first, _) = manager
            .create_preview(preview("install", "curl"))
            .expect("preview should be created");
        let (second, _) = manager
            .create_preview(preview("remove", "nano"))
            .expect("preview should be created");
        let (first_record, _) = manager
            .queue_preview(first.id)
            .expect("preview should queue");
        manager
            .queue_preview(second.id)
            .expect("preview should queue");

        let (running, running_event) = manager
            .begin_next_simulation()
            .expect("simulation should start");
        let active_snapshot = manager.snapshot().expect("snapshot should load");
        assert!(active_snapshot.has_active);
        assert_eq!(active_snapshot.active.id, first_record.id);
        assert_eq!(active_snapshot.queued.len(), 1);
        assert_eq!(running.state, "running");
        assert_eq!(running_event.event, "running");
        assert!(manager.begin_next_simulation().is_err());

        let (progress, progress_event) = manager
            .update_simulation_progress(running.id, 5_000, "Replaying APT simulation")
            .expect("progress should update");
        assert_eq!(progress.progress_basis_points, 5_000);
        assert_eq!(progress_event.event, "progress");

        let (completed, completed_event) = manager
            .complete_simulation(running.id)
            .expect("simulation should complete");
        assert_eq!(completed.state, "completed");
        assert_eq!(completed.progress_basis_points, 10_000);
        assert_eq!(completed_event.event, "completed");
        let completed_snapshot = manager.snapshot().expect("snapshot should load");
        assert!(!completed_snapshot.has_active);
        assert_eq!(completed_snapshot.queued.len(), 1);
        fs::remove_file(path).expect("test journal should be removable");
    }

    #[test]
    fn cancellation_is_limited_to_queued_transactions() {
        let (manager, path) = manager();
        let (preview, _) = manager
            .create_preview(preview("upgrade", "curl"))
            .expect("preview should be created");
        let (record, _) = manager
            .queue_preview(preview.id)
            .expect("preview should queue");
        let (cancelled, event) = manager.cancel(record.id).expect("queue item should cancel");

        assert_eq!(cancelled.state, "cancelled");
        assert!(!cancelled.can_cancel);
        assert_eq!(event.event, "cancelled");
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

    #[test]
    fn event_history_is_ordered_and_cursor_based() {
        let (manager, path) = manager();
        let (preview, preview_event) = manager
            .create_preview(preview("install", "curl"))
            .expect("preview should be created");
        let (_, queue_event) = manager
            .queue_preview(preview.id)
            .expect("preview should queue");

        let all = manager.events(0, 10).expect("events should load");
        assert_eq!(all, [preview_event.clone(), queue_event.clone()]);
        assert_eq!(
            manager
                .events(preview_event.sequence, 10)
                .expect("cursor events should load"),
            [queue_event]
        );
        assert!(manager.events(0, 0).is_err());
        fs::remove_file(path).expect("test journal should be removable");
    }
}
