from pathlib import Path

transaction = Path("crates/genixpkgd/src/transaction.rs")
text = transaction.read_text()
old = '''        if record.state != STATE_RUNNING {
            bail!("transaction {transaction_id} is not running");
        }
        record.progress_basis_points = progress_basis_points;
'''
new = '''        if record.state != STATE_RUNNING {
            bail!("transaction {transaction_id} is not running");
        }
        if progress_basis_points <= record.progress_basis_points {
            bail!(
                "simulation progress must advance beyond {} basis points",
                record.progress_basis_points
            );
        }
        record.progress_basis_points = progress_basis_points;
'''
if old in text:
    text = text.replace(old, new, 1)
elif new not in text:
    raise SystemExit("progress monotonicity marker not found")

old_test = '''        assert_eq!(progress.progress_basis_points, 5_000);
        assert_eq!(progress_event.event, "progress");

        let (completed, completed_event) = manager
'''
new_test = '''        assert_eq!(progress.progress_basis_points, 5_000);
        assert_eq!(progress_event.event, "progress");
        assert!(
            manager
                .update_simulation_progress(running.id, 5_000, "duplicate")
                .is_err()
        );
        assert!(
            manager
                .update_simulation_progress(running.id, 4_000, "regression")
                .is_err()
        );

        let (completed, completed_event) = manager
'''
if old_test in text:
    text = text.replace(old_test, new_test, 1)
elif new_test not in text:
    raise SystemExit("progress test marker not found")
transaction.write_text(text)

main = Path("crates/genixpkgd/src/main.rs")
text = main.read_text()
old_cursor = '''        let mut logs_open = true;
        let outcome = loop {
'''
new_cursor = '''        let mut logs_open = true;
        let mut observed_progress = running.progress_basis_points;
        let outcome = loop {
'''
if old_cursor in text:
    text = text.replace(old_cursor, new_cursor, 1)
elif new_cursor not in text:
    raise SystemExit("progress cursor marker not found")

old_log = '''                        Some(log) => {
                            match self.transactions.record_simulation_log(
                                running.id,
                                &log.level,
                                &log.message,
                            ) {
                                Ok(event) => Self::emit_lifecycle_event(&emitter, &event).await,
                                Err(error) => break Err(error),
                            }
                        }
'''
new_log = '''                        Some(log) => {
                            match self.transactions.record_simulation_log(
                                running.id,
                                &log.level,
                                &log.message,
                            ) {
                                Ok(event) => Self::emit_lifecycle_event(&emitter, &event).await,
                                Err(error) => break Err(error),
                            }
                            if let Some(progress) = log.progress_basis_points
                                && progress > observed_progress
                            {
                                let message = format!("APT simulation progress: {}", log.message);
                                match self.transactions.update_simulation_progress(
                                    running.id,
                                    progress,
                                    &message,
                                ) {
                                    Ok((_, event)) => {
                                        observed_progress = progress;
                                        Self::emit_lifecycle_event(&emitter, &event).await;
                                    }
                                    Err(error) => break Err(error),
                                }
                            }
                        }
'''
if old_log in text:
    text = text.replace(old_log, new_log, 1)
elif new_log not in text:
    raise SystemExit("log progress marker not found")
main.write_text(text)
