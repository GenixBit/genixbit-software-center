from pathlib import Path

transaction = Path("crates/genixpkgd/src/transaction.rs")
text = transaction.read_text()
begin_start = text.index("    pub fn begin_next_simulation(")
progress_start = text.index("    pub fn update_simulation_progress(", begin_start)
begin_block = text[begin_start:progress_start]
begin_block = begin_block.replace("record.can_cancel = false;", "record.can_cancel = true;", 1)
text = text[:begin_start] + begin_block + text[progress_start:]

cancel_start = text.index("    pub fn cancel(")
if "    pub fn request_simulation_cancellation(" not in text:
    methods = '''    pub fn request_simulation_cancellation(
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
        if record.state != STATE_RUNNING || !record.can_cancel {
            bail!("transaction {transaction_id} cannot be cancelled");
        }
        record.can_cancel = false;
        record.updated_unix_ms = now_unix_ms();
        record.message = "Cancellation requested for the active APT simulation".to_owned();
        let event = self.record_event("cancellation-requested", &record, "info");

        self.journal.append(&record)?;
        state.records.insert(transaction_id, record.clone());
        push_event(&mut state.events, event.clone());
        Ok((record, event))
    }

    pub fn cancel_active_simulation(
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
        record.state = STATE_CANCELLED.to_owned();
        record.can_cancel = false;
        record.updated_unix_ms = now_unix_ms();
        record.message = "Active APT simulation cancelled; no packages were changed".to_owned();
        let event = self.record_event("cancelled", &record, "info");

        self.journal.append(&record)?;
        state.active = None;
        state.records.insert(transaction_id, record.clone());
        push_event(&mut state.events, event.clone());
        Ok((record, event))
    }

'''
    text = text[:cancel_start] + methods + text[cancel_start:]

text = text.replace(
    '        assert_eq!(running.state, "running");\n        assert_eq!(reviewed_preview.id, first.id);',
    '        assert_eq!(running.state, "running");\n        assert!(running.can_cancel);\n        assert_eq!(reviewed_preview.id, first.id);',
    1,
)
if "fn active_simulation_supports_requested_cancellation()" not in text:
    marker = '    #[test]\n    fn failed_simulation_releases_the_active_slot() {'
    test = '''    #[test]
    fn active_simulation_supports_requested_cancellation() {
        let (manager, path) = manager();
        let (preview, _) = manager
            .create_preview(preview("install", "curl"))
            .expect("preview should be created");
        let (record, _) = manager
            .queue_preview(preview.id)
            .expect("preview should queue");
        manager
            .begin_next_simulation()
            .expect("simulation should start");

        let (requested, requested_event) = manager
            .request_simulation_cancellation(record.id)
            .expect("cancellation should be requested");
        assert!(!requested.can_cancel);
        assert_eq!(requested_event.event, "cancellation-requested");

        let (cancelled, cancelled_event) = manager
            .cancel_active_simulation(record.id)
            .expect("active simulation should cancel");
        assert_eq!(cancelled.state, "cancelled");
        assert_eq!(cancelled_event.event, "cancelled");
        assert!(!manager.snapshot().expect("snapshot should load").has_active);
        fs::remove_file(path).expect("test journal should be removable");
    }

'''
    if marker not in text:
        raise SystemExit("active cancellation test marker not found")
    text = text.replace(marker, test + marker, 1)
transaction.write_text(text)

main = Path("crates/genixpkgd/src/main.rs")
text = main.read_text()
if "mod apt_live;" not in text:
    text = text.replace("mod apt;\n", "mod apt;\nmod apt_live;\n", 1)
if "mod simulation_control;" not in text:
    text = text.replace("mod journal;\n", "mod journal;\nmod simulation_control;\n", 1)
if "use apt_live::AptSimulationOutcome;" not in text:
    text = text.replace(
        "use anyhow::Context;\n",
        "use anyhow::Context;\nuse apt_live::AptSimulationOutcome;\n",
        1,
    )
if "use simulation_control::SimulationControl;" not in text:
    text = text.replace(
        "use journal::TransactionJournal;\n",
        "use journal::TransactionJournal;\nuse simulation_control::SimulationControl;\n",
        1,
    )
if "simulation_control: SimulationControl," not in text:
    text = text.replace(
        "    transactions: TransactionManager,\n",
        "    transactions: TransactionManager,\n    simulation_control: SimulationControl,\n",
        1,
    )
    text = text.replace(
        "            transactions: TransactionManager::new(TransactionJournal::from_environment()),\n",
        "            transactions: TransactionManager::new(TransactionJournal::from_environment()),\n            simulation_control: SimulationControl::default(),\n",
        1,
    )

cancel_method_start = text.index("    async fn cancel_transaction(")
run_method_start = text.index("    async fn run_next_simulation(", cancel_method_start)
new_cancel = '''    async fn cancel_transaction(
        &self,
        transaction_id: u64,
        #[zbus(connection)] connection: &Connection,
        #[zbus(header)] header: Header<'_>,
        #[zbus(signal_emitter)] emitter: SignalEmitter<'_>,
    ) -> zbus::fdo::Result<TransactionRecord> {
        let sender = header.sender().ok_or_else(|| {
            zbus::fdo::Error::AccessDenied(
                "missing authenticated D-Bus caller identity".to_owned(),
            )
        })?;
        self.authorization
            .authorize_transaction_control(connection, sender, "cancelling a package transaction")
            .await?;

        if self.simulation_control.is_active(transaction_id).await {
            self.simulation_control
                .request(transaction_id)
                .await
                .map_err(dbus_failed)?;
            let (record, event) = self
                .transactions
                .request_simulation_cancellation(transaction_id)
                .map_err(dbus_failed)?;
            Self::emit_lifecycle_event(&emitter, &event).await;
            return Ok(record);
        }

        let (record, event) = self
            .transactions
            .cancel(transaction_id)
            .map_err(dbus_failed)?;
        Self::emit_lifecycle_event(&emitter, &event).await;
        Ok(record)
    }

'''
text = text[:cancel_method_start] + new_cancel + text[run_method_start:]

method_start = text.index("    async fn run_next_simulation(")
queue_start = text.index("    async fn transaction_queue(", method_start)
method_header_end = text.index("        let sender =", method_start)
new_body = '''        let sender = header.sender().ok_or_else(|| {
            zbus::fdo::Error::AccessDenied("missing authenticated D-Bus caller identity".to_owned())
        })?;
        self.authorization
            .authorize_transaction_control(
                connection,
                sender,
                "running a simulated package transaction",
            )
            .await?;

        let (running, reviewed_preview, running_event) = self
            .transactions
            .begin_next_simulation()
            .map_err(dbus_failed)?;
        Self::emit_lifecycle_event(&emitter, &running_event).await;

        let cancellation = match self.simulation_control.register(running.id).await {
            Ok(cancellation) => cancellation,
            Err(error) => {
                let message = format!("failed to register simulation cancellation: {error}");
                let (failed, failed_event) = self
                    .transactions
                    .fail_simulation(running.id, &message)
                    .map_err(dbus_failed)?;
                Self::emit_lifecycle_event(&emitter, &failed_event).await;
                return Err(zbus::fdo::Error::Failed(failed.message));
            }
        };

        let outcome = apt_live::run_cancellable(&running.kind, &running.package, cancellation).await;
        if let Err(error) = self.simulation_control.clear(running.id).await {
            let message = format!("failed to clear simulation cancellation handle: {error}");
            let (failed, failed_event) = self
                .transactions
                .fail_simulation(running.id, &message)
                .map_err(dbus_failed)?;
            Self::emit_lifecycle_event(&emitter, &failed_event).await;
            return Err(zbus::fdo::Error::Failed(failed.message));
        }

        let simulation = match outcome {
            Ok(AptSimulationOutcome::Completed(simulation)) => simulation,
            Ok(AptSimulationOutcome::Cancelled) => {
                let (cancelled, cancelled_event) = self
                    .transactions
                    .cancel_active_simulation(running.id)
                    .map_err(dbus_failed)?;
                Self::emit_lifecycle_event(&emitter, &cancelled_event).await;
                return Ok(cancelled);
            }
            Err(error) => {
                let message = format!("APT simulation subprocess failed: {error}");
                let (failed, failed_event) = self
                    .transactions
                    .fail_simulation(running.id, &message)
                    .map_err(dbus_failed)?;
                Self::emit_lifecycle_event(&emitter, &failed_event).await;
                return Err(zbus::fdo::Error::Failed(failed.message));
            }
        };

        let preview_changed = simulation.changes != reviewed_preview.changes
            || simulation.download_size_bytes != reviewed_preview.download_size_bytes
            || simulation.installed_size_delta_bytes
                != reviewed_preview.installed_size_delta_bytes;
        if preview_changed {
            let message = "APT simulation changed since review; create and approve a new preview";
            let (failed, failed_event) = self
                .transactions
                .fail_simulation(running.id, message)
                .map_err(dbus_failed)?;
            Self::emit_lifecycle_event(&emitter, &failed_event).await;
            return Err(zbus::fdo::Error::Failed(failed.message));
        }

        let progress_message = format!("APT simulation verified: {}", simulation.summary);
        let (_, progress_event) = self
            .transactions
            .update_simulation_progress(running.id, 9_000, &progress_message)
            .map_err(dbus_failed)?;
        Self::emit_lifecycle_event(&emitter, &progress_event).await;

        let (completed, completed_event) = self
            .transactions
            .complete_simulation(running.id)
            .map_err(dbus_failed)?;
        Self::emit_lifecycle_event(&emitter, &completed_event).await;
        Ok(completed)
    }

'''
text = text[:method_header_end] + new_body + text[queue_start:]
main.write_text(text)
