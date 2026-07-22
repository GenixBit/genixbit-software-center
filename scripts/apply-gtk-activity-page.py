from pathlib import Path

main = Path("crates/software-center/src/main.rs")
text = main.read_text()
text = text.replace(
    "    SystemSnapshot, UpdateRecord,\n};",
    "    SystemSnapshot, TransactionRecord, UpdateRecord,\n};",
    1,
)
text = text.replace(
    "const INSTALLED_PAGE_SIZE: usize = 100;\n",
    "const INSTALLED_PAGE_SIZE: usize = 100;\nconst ACTIVITY_LIMIT: u64 = 100;\n",
    1,
)
text = text.replace(
    "    updates_status: gtk::Label,\n    updates_list: gtk::ListBox,\n",
    "    updates_status: gtk::Label,\n    updates_list: gtk::ListBox,\n    activity_status: gtk::Label,\n    activity_list: gtk::ListBox,\n",
    1,
)

updates_page = '''    add_widget_page(
        &stack,
        "updates",
        "Updates",
        "software-update-available-symbolic",
        &updates_page,
    );
'''
activity_page = updates_page + '''
    let (activity_page, activity_status, activity_list) = build_list_page(
        "Transaction activity",
        "Review recent package previews, simulations, cancellations, failures and interrupted work.",
    );
    add_widget_page(
        &stack,
        "activity",
        "Activity",
        "document-open-recent-symbolic",
        &activity_page,
    );
'''
if updates_page in text and '"activity"' not in text:
    text = text.replace(updates_page, activity_page, 1)

text = text.replace(
    "        updates_status,\n        updates_list,\n",
    "        updates_status,\n        updates_list,\n        activity_status,\n        activity_list,\n",
    1,
)
text = text.replace(
    "        refresh.connect_clicked(move |_| start_snapshot_load(&ui));",
    "        refresh.connect_clicked(move |_| {\n            start_snapshot_load(&ui);\n            start_activity_load(&ui);\n        });",
    1,
)
text = text.replace(
    "    start_snapshot_load(&ui);\n    start_featured_collections_load(&ui);",
    "    start_snapshot_load(&ui);\n    start_activity_load(&ui);\n    start_featured_collections_load(&ui);",
    1,
)

activity_functions = '''
fn start_activity_load(ui: &UiState) {
    ui.activity_status
        .set_text("Loading recent package transaction activity…");
    clear_list(&ui.activity_list);

    let (sender, receiver) = mpsc::channel();
    thread::spawn(move || {
        let result = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(anyhow::Error::from)
            .and_then(|runtime| runtime.block_on(client::recent_transactions(ACTIVITY_LIMIT)));
        let _ = sender.send(result);
    });

    let ui = ui.clone();
    glib::timeout_add_local(Duration::from_millis(100), move || {
        match receiver.try_recv() {
            Ok(Ok(records)) => {
                render_activity(&ui, &records);
                glib::ControlFlow::Break
            }
            Ok(Err(error)) => {
                ui.activity_status
                    .set_text(&format!("Unable to load transaction activity: {error}"));
                glib::ControlFlow::Break
            }
            Err(TryRecvError::Empty) => glib::ControlFlow::Continue,
            Err(TryRecvError::Disconnected) => {
                ui.activity_status
                    .set_text("The transaction activity worker stopped unexpectedly.");
                glib::ControlFlow::Break
            }
        }
    });
}

fn render_activity(ui: &UiState, records: &[TransactionRecord]) {
    clear_list(&ui.activity_list);
    if records.is_empty() {
        ui.activity_status
            .set_text("No package transaction activity has been recorded.");
        return;
    }

    ui.activity_status.set_text(&format!(
        "Showing {} recent transactions. Package execution remains simulation-only.",
        records.len()
    ));
    for record in records {
        let row = adw::ActionRow::builder()
            .title(activity_title(record))
            .subtitle(format!("Transaction #{} · {}", record.id, record.message))
            .build();

        let badge = gtk::Label::new(Some(activity_state_label(&record.state)));
        badge.add_css_class(activity_state_css_class(&record.state));
        row.add_suffix(&badge);

        if let Some(fraction) = activity_progress_fraction(record.progress_basis_points) {
            let progress = gtk::ProgressBar::new();
            progress.set_fraction(fraction);
            progress.set_width_request(120);
            progress.set_tooltip_text(Some(&format!(
                "{}% complete",
                record.progress_basis_points / 100
            )));
            row.add_suffix(&progress);
        }
        ui.activity_list.append(&row);
    }
}

fn activity_title(record: &TransactionRecord) -> String {
    let operation = match record.kind.as_str() {
        "install" => "Install",
        "remove" => "Remove",
        "upgrade" => "Upgrade",
        _ => "Transaction",
    };
    format!("{operation} {}", record.package)
}

fn activity_state_label(state: &str) -> &str {
    match state {
        "queued" => "Queued",
        "running" => "Running",
        "completed" => "Completed",
        "failed" => "Failed",
        "cancelled" => "Cancelled",
        "interrupted" => "Interrupted",
        other => other,
    }
}

fn activity_state_css_class(state: &str) -> &'static str {
    match state {
        "failed" | "interrupted" => "error",
        "completed" => "success",
        "queued" | "running" => "accent",
        _ => "dim-label",
    }
}

fn activity_progress_fraction(progress_basis_points: u32) -> Option<f64> {
    if progress_basis_points == 0 || progress_basis_points >= 10_000 {
        None
    } else {
        Some(f64::from(progress_basis_points) / 10_000.0)
    }
}

'''
marker = "fn start_featured_collections_load(ui: &UiState) {"
if "fn start_activity_load(" not in text:
    if marker not in text:
        raise SystemExit("activity insertion marker not found")
    text = text.replace(marker, activity_functions + marker, 1)

text = text.replace(
    "    ui.updates_status.set_text(message);\n}",
    "    ui.updates_status.set_text(message);\n    ui.activity_status.set_text(message);\n}\n",
    1,
)

if "mod activity_tests" not in text:
    tests = '''

#[cfg(test)]
mod activity_tests {
    use genixbit_package_model::TransactionRecord;

    use super::{activity_progress_fraction, activity_state_label, activity_title};

    fn record(kind: &str, package: &str) -> TransactionRecord {
        TransactionRecord {
            kind: kind.to_owned(),
            package: package.to_owned(),
            ..TransactionRecord::default()
        }
    }

    #[test]
    fn formats_known_and_unknown_activity_titles() {
        assert_eq!(activity_title(&record("install", "curl")), "Install curl");
        assert_eq!(activity_title(&record("remove", "nano")), "Remove nano");
        assert_eq!(activity_title(&record("upgrade", "git")), "Upgrade git");
        assert_eq!(activity_title(&record("custom", "tool")), "Transaction tool");
    }

    #[test]
    fn labels_terminal_and_active_states() {
        assert_eq!(activity_state_label("running"), "Running");
        assert_eq!(activity_state_label("interrupted"), "Interrupted");
        assert_eq!(activity_state_label("custom"), "custom");
    }

    #[test]
    fn shows_progress_only_for_incomplete_active_values() {
        assert_eq!(activity_progress_fraction(0), None);
        assert_eq!(activity_progress_fraction(10_000), None);
        assert_eq!(activity_progress_fraction(5_000), Some(0.5));
    }
}
'''
    text += tests

main.write_text(text)

roadmap = Path("docs/ROADMAP.md")
roadmap_text = roadmap.read_text()
roadmap_text = roadmap_text.replace(
    "- [ ] Activity history",
    "- [x] Read-only transaction Activity history page",
    1,
)
roadmap.write_text(roadmap_text)
