from pathlib import Path

transaction = Path("crates/genixpkgd/src/transaction.rs")
text = transaction.read_text()
text = text.replace(
    "    recovery::{RecoveredTransactions, recover_transactions},\n};",
    "    recovery::{RecoveredTransactions, recover_transactions},\n    transaction_query::recent_transactions,\n};",
    1,
)
old = '''    pub fn events(&self, after_sequence: u64, limit: u64) -> anyhow::Result<Vec<TransactionEvent>> {
'''
new = '''    pub fn recent_records(&self, limit: u64) -> anyhow::Result<Vec<TransactionRecord>> {
        let state = self
            .state
            .lock()
            .map_err(|_| anyhow::anyhow!("transaction manager lock was poisoned"))?;
        recent_transactions(state.records.values().cloned(), limit)
    }

    pub fn events(&self, after_sequence: u64, limit: u64) -> anyhow::Result<Vec<TransactionEvent>> {
'''
if old in text:
    text = text.replace(old, new, 1)
elif new not in text:
    raise SystemExit("recent records insertion marker not found")

if "fn recent_records_return_one_latest_state_per_transaction()" not in text:
    marker = '    #[test]\n    fn restart_marks_active_work_interrupted_and_advances_ids() {'
    test = '''    #[test]
    fn recent_records_return_one_latest_state_per_transaction() {
        let (manager, path) = manager();
        let (first_preview, _) = manager
            .create_preview(preview("install", "curl"))
            .expect("preview should be created");
        let (first_record, _) = manager
            .queue_preview(first_preview.id)
            .expect("preview should queue");
        manager
            .begin_next_simulation()
            .expect("simulation should start");
        manager
            .complete_simulation(first_record.id)
            .expect("simulation should complete");

        let (second_preview, _) = manager
            .create_preview(preview("remove", "nano"))
            .expect("preview should be created");
        let (second_record, _) = manager
            .queue_preview(second_preview.id)
            .expect("preview should queue");

        let recent = manager.recent_records(10).expect("records should load");
        assert_eq!(recent.len(), 2);
        assert!(recent.iter().any(|record| {
            record.id == first_record.id && record.state == "completed"
        }));
        assert!(recent.iter().any(|record| {
            record.id == second_record.id && record.state == "queued"
        }));
        assert!(manager.recent_records(0).is_err());
        cleanup(path);
    }

'''
    if marker not in text:
        raise SystemExit("recent records test marker not found")
    text = text.replace(marker, test + marker, 1)
transaction.write_text(text)

main = Path("crates/genixpkgd/src/main.rs")
text = main.read_text()
if "mod transaction_query;" not in text:
    text = text.replace("mod transaction;\n", "mod transaction;\nmod transaction_query;\n", 1)
old_method = '''    async fn transaction_queue(&self) -> zbus::fdo::Result<TransactionQueueSnapshot> {
        self.transactions.snapshot().map_err(dbus_failed)
    }

    async fn transaction_events(
'''
new_method = '''    async fn transaction_queue(&self) -> zbus::fdo::Result<TransactionQueueSnapshot> {
        self.transactions.snapshot().map_err(dbus_failed)
    }

    async fn recent_transactions(
        &self,
        limit: u64,
    ) -> zbus::fdo::Result<Vec<TransactionRecord>> {
        self.transactions.recent_records(limit).map_err(dbus_failed)
    }

    async fn transaction_events(
'''
if old_method in text:
    text = text.replace(old_method, new_method, 1)
elif new_method not in text:
    raise SystemExit("D-Bus recent transactions marker not found")
main.write_text(text)
