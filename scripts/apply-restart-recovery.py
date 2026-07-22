from pathlib import Path

transaction = Path("crates/genixpkgd/src/transaction.rs")
text = transaction.read_text()
text = text.replace(
    "use crate::{event_journal::EventJournal, journal::TransactionJournal};",
    "use crate::{\n    event_journal::EventJournal,\n    journal::TransactionJournal,\n    recovery::{RecoveredTransactions, recover_transactions},\n};",
    1,
)
old_constructor = '''    pub fn new(journal: TransactionJournal) -> Self {
        let event_journal = EventJournal::from_transaction_journal(journal.path());
        let restored_events = match event_journal.read_recent(MAX_EVENT_HISTORY) {
            Ok(events) => events,
            Err(error) => {
                tracing::warn!(%error, "failed to restore transaction event history");
                Vec::new()
            }
        };
        let next_event_sequence = restored_events
            .last()
            .map(|event| event.sequence.saturating_add(1))
            .unwrap_or(1);
        let state = ManagerState {
            events: VecDeque::from(restored_events),
            ..ManagerState::default()
        };
        Self {
            next_preview_id: AtomicU64::new(1),
            next_transaction_id: AtomicU64::new(1),
            next_event_sequence: AtomicU64::new(next_event_sequence),
            state: Mutex::new(state),
            journal,
            event_journal,
        }
    }
'''
new_constructor = '''    pub fn new(journal: TransactionJournal) -> Self {
        let event_journal = EventJournal::from_transaction_journal(journal.path());
        let mut restored_events = match event_journal.read_recent(MAX_EVENT_HISTORY) {
            Ok(events) => VecDeque::from(events),
            Err(error) => {
                tracing::warn!(%error, "failed to restore transaction event history");
                VecDeque::new()
            }
        };
        let mut next_event_sequence = restored_events
            .back()
            .map(|event| event.sequence.saturating_add(1))
            .unwrap_or(1);
        let journal_records = match journal.read_all() {
            Ok(records) => records,
            Err(error) => {
                tracing::warn!(%error, "failed to restore transaction state journal");
                Vec::new()
            }
        };
        let RecoveredTransactions {
            records,
            interrupted,
            next_transaction_id,
            next_preview_id,
        } = recover_transactions(journal_records, now_unix_ms());

        for record in interrupted {
            if let Err(error) = journal.append(&record) {
                tracing::warn!(
                    transaction_id = record.id,
                    %error,
                    "failed to persist interrupted transaction recovery"
                );
            }
            let event = TransactionEvent {
                sequence: next_event_sequence,
                event: "interrupted".to_owned(),
                transaction_id: record.id,
                preview_id: record.preview_id,
                kind: record.kind.clone(),
                package: record.package.clone(),
                state: record.state.clone(),
                progress_basis_points: record.progress_basis_points,
                level: "error".to_owned(),
                message: record.message.clone(),
                created_unix_ms: record.updated_unix_ms,
            };
            next_event_sequence = next_event_sequence.saturating_add(1);
            if let Err(error) = event_journal.append(&event) {
                tracing::warn!(
                    sequence = event.sequence,
                    transaction_id = event.transaction_id,
                    %error,
                    "failed to persist interrupted transaction event"
                );
            }
            push_event(&mut restored_events, event);
        }

        let state = ManagerState {
            records,
            events: restored_events,
            ..ManagerState::default()
        };
        Self {
            next_preview_id: AtomicU64::new(next_preview_id),
            next_transaction_id: AtomicU64::new(next_transaction_id),
            next_event_sequence: AtomicU64::new(next_event_sequence),
            state: Mutex::new(state),
            journal,
            event_journal,
        }
    }
'''
if old_constructor not in text:
    raise SystemExit("transaction constructor marker not found")
text = text.replace(old_constructor, new_constructor, 1)

if "fn restart_marks_active_work_interrupted_and_advances_ids()" not in text:
    marker = '    #[test]\n    fn restores_persistent_lifecycle_events_and_sequence() {'
    test = '''    #[test]
    fn restart_marks_active_work_interrupted_and_advances_ids() {
        let (manager, path) = manager();
        let (first_preview, _) = manager
            .create_preview(preview("install", "curl"))
            .expect("preview should be created");
        let (queued, _) = manager
            .queue_preview(first_preview.id)
            .expect("preview should queue");
        manager
            .begin_next_simulation()
            .expect("simulation should start");
        drop(manager);

        let restored = TransactionManager::new(TransactionJournal::new(path.clone()));
        let snapshot = restored.snapshot().expect("snapshot should load");
        assert!(!snapshot.has_active);
        assert!(snapshot.queued.is_empty());
        let journal = restored.journal().expect("journal should load");
        let interrupted = journal.last().expect("recovery record should exist");
        assert_eq!(interrupted.id, queued.id);
        assert_eq!(interrupted.state, "interrupted");
        assert!(!interrupted.can_cancel);
        assert!(
            restored
                .events(0, 100)
                .expect("events should load")
                .iter()
                .any(|event| event.event == "interrupted" && event.transaction_id == queued.id)
        );

        let (next_preview, _) = restored
            .create_preview(preview("remove", "nano"))
            .expect("next preview should be created");
        assert!(next_preview.id > first_preview.id);
        let (next_record, _) = restored
            .queue_preview(next_preview.id)
            .expect("next preview should queue");
        assert!(next_record.id > queued.id);
        drop(restored);
        cleanup(path);
    }

'''
    if marker not in text:
        raise SystemExit("restart recovery test marker not found")
    text = text.replace(marker, test + marker, 1)
transaction.write_text(text)

main = Path("crates/genixpkgd/src/main.rs")
text = main.read_text()
if "mod recovery;" not in text:
    text = text.replace("mod journal;\n", "mod journal;\nmod recovery;\n", 1)
main.write_text(text)
