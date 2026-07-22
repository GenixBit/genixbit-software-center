from pathlib import Path

transaction = Path("crates/genixpkgd/src/transaction.rs")
text = transaction.read_text()
text = text.replace(
    "use crate::journal::TransactionJournal;",
    "use crate::{event_journal::EventJournal, journal::TransactionJournal};",
    1,
)
text = text.replace(
    "    journal: TransactionJournal,\n}",
    "    journal: TransactionJournal,\n    event_journal: EventJournal,\n}",
    1,
)
old_constructor = '''    pub fn new(journal: TransactionJournal) -> Self {
        Self {
            next_preview_id: AtomicU64::new(1),
            next_transaction_id: AtomicU64::new(1),
            next_event_sequence: AtomicU64::new(1),
            state: Mutex::new(ManagerState::default()),
            journal,
        }
    }
'''
new_constructor = '''    pub fn new(journal: TransactionJournal) -> Self {
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
if old_constructor in text:
    text = text.replace(old_constructor, new_constructor, 1)
elif new_constructor not in text:
    raise SystemExit("transaction constructor marker not found")

text = text.replace(
    "push_event(&mut state.events, event.clone());",
    "self.store_event(&mut state.events, event.clone());",
)

old_path = '''    pub fn journal_path(&self) -> &std::path::Path {
        self.journal.path()
    }

    fn preview_event(&self, preview: &TransactionPreview) -> TransactionEvent {
'''
new_path = '''    pub fn journal_path(&self) -> &std::path::Path {
        self.journal.path()
    }

    pub fn event_journal_path(&self) -> &std::path::Path {
        self.event_journal.path()
    }

    fn store_event(&self, events: &mut VecDeque<TransactionEvent>, event: TransactionEvent) {
        if event.event != "log"
            && let Err(error) = self.event_journal.append(&event)
        {
            tracing::warn!(
                sequence = event.sequence,
                transaction_id = event.transaction_id,
                %error,
                "failed to persist transaction lifecycle event"
            );
        }
        push_event(events, event);
    }

    fn preview_event(&self, preview: &TransactionPreview) -> TransactionEvent {
'''
if old_path in text:
    text = text.replace(old_path, new_path, 1)
elif new_path not in text:
    raise SystemExit("journal path marker not found")

text = text.replace(
    "use crate::journal::TransactionJournal;",
    "use crate::{event_journal::EventJournal, journal::TransactionJournal};",
    1,
)
manager_end = '''    fn manager() -> (TransactionManager, PathBuf) {
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
'''
cleanup_block = manager_end + '''
    fn cleanup(path: PathBuf) {
        let event_journal = EventJournal::from_transaction_journal(&path);
        let _ = fs::remove_file(event_journal.path());
        let _ = fs::remove_file(path);
    }
'''
if manager_end in text and "    fn cleanup(path: PathBuf)" not in text:
    text = text.replace(manager_end, cleanup_block, 1)
text = text.replace(
    'fs::remove_file(path).expect("test journal should be removable");',
    "cleanup(path);",
)

if "fn restores_persistent_lifecycle_events_and_sequence()" not in text:
    marker = '    #[test]\n    fn event_history_is_ordered_and_cursor_based() {'
    test = '''    #[test]
    fn restores_persistent_lifecycle_events_and_sequence() {
        let (manager, path) = manager();
        let (preview, preview_event) = manager
            .create_preview(preview("install", "curl"))
            .expect("preview should be created");
        let (_, queued_event) = manager
            .queue_preview(preview.id)
            .expect("preview should queue");
        let event_path = manager.event_journal_path().to_path_buf();
        drop(manager);

        let restored = TransactionManager::new(TransactionJournal::new(path.clone()));
        assert_eq!(
            restored.events(0, 10).expect("restored events should load"),
            [preview_event, queued_event.clone()]
        );
        let (_, next_event) = restored
            .create_preview(preview("remove", "nano"))
            .expect("new preview should be created");
        assert!(next_event.sequence > queued_event.sequence);
        drop(restored);

        assert!(event_path.exists());
        cleanup(path);
    }

'''
    if marker not in text:
        raise SystemExit("event history test marker not found")
    text = text.replace(marker, test + marker, 1)
transaction.write_text(text)

main = Path("crates/genixpkgd/src/main.rs")
text = main.read_text()
if "mod event_journal;" not in text:
    text = text.replace("mod dpkg;\n", "mod dpkg;\nmod event_journal;\n", 1)
old_method = '''    async fn transaction_events(
        &self,
        after_sequence: u64,
        limit: u64,
    ) -> zbus::fdo::Result<Vec<TransactionEvent>> {
        self.transactions
            .events(after_sequence, limit)
            .map_err(dbus_failed)
    }

    async fn transaction_journal(&self) -> zbus::fdo::Result<Vec<TransactionRecord>> {
'''
new_method = '''    async fn transaction_events(
        &self,
        after_sequence: u64,
        limit: u64,
    ) -> zbus::fdo::Result<Vec<TransactionEvent>> {
        self.transactions
            .events(after_sequence, limit)
            .map_err(dbus_failed)
    }

    async fn transaction_event_journal_path(&self) -> String {
        self.transactions.event_journal_path().display().to_string()
    }

    async fn transaction_journal(&self) -> zbus::fdo::Result<Vec<TransactionRecord>> {
'''
if old_method in text:
    text = text.replace(old_method, new_method, 1)
elif new_method not in text:
    raise SystemExit("D-Bus event journal marker not found")
main.write_text(text)
