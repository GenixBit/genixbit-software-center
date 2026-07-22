from pathlib import Path

main = Path("crates/software-center/src/main.rs")
text = main.read_text()
text = text.replace(
    "    SystemSnapshot, TransactionRecord, UpdateRecord,\n};",
    "    SystemSnapshot, TransactionEvent, TransactionRecord, UpdateRecord,\n};",
    1,
)
text = text.replace(
    "const ACTIVITY_LIMIT: u64 = 100;\n",
    "const ACTIVITY_LIMIT: u64 = 100;\nconst ACTIVITY_EVENT_LIMIT: u64 = 500;\n",
    1,
)

old_row = '''        let row = adw::ActionRow::builder()
            .title(activity_title(record))
            .subtitle(format!("Transaction #{} · {}", record.id, record.message))
            .build();
'''
new_row = '''        let row = adw::ActionRow::builder()
            .title(activity_title(record))
            .subtitle(format!("Transaction #{} · {}", record.id, record.message))
            .activatable(true)
            .build();
'''
if old_row in text:
    text = text.replace(old_row, new_row, 1)
elif new_row not in text:
    raise SystemExit("Activity row marker not found")

old_append = '''        ui.activity_list.append(&row);
    }
}

fn activity_title(record: &TransactionRecord) -> String {
'''
new_append = '''        let callback_ui = ui.clone();
        let callback_record = record.clone();
        row.connect_activated(move |_| {
            start_transaction_details(&callback_ui, &callback_record)
        });
        ui.activity_list.append(&row);
    }
}

fn start_transaction_details(ui: &UiState, record: &TransactionRecord) {
    let status = gtk::Label::new(Some("Loading transaction lifecycle events…"));
    status.set_xalign(0.0);
    status.set_wrap(true);

    let list = gtk::ListBox::new();
    list.set_selection_mode(gtk::SelectionMode::None);
    list.add_css_class("boxed-list");

    let content = gtk::Box::new(gtk::Orientation::Vertical, 12);
    content.set_margin_top(18);
    content.set_margin_bottom(18);
    content.set_margin_start(18);
    content.set_margin_end(18);
    content.append(&status);
    let scrolled = gtk::ScrolledWindow::builder()
        .hexpand(true)
        .vexpand(true)
        .child(&list)
        .build();
    content.append(&scrolled);

    let window = gtk::Window::builder()
        .title(format!("Transaction details — #{}", record.id))
        .transient_for(&ui.window)
        .modal(true)
        .default_width(760)
        .default_height(640)
        .child(&content)
        .build();
    window.present();

    let record = record.clone();
    let transaction_id = record.id;
    let (sender, receiver) = mpsc::channel();
    thread::spawn(move || {
        let result = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(anyhow::Error::from)
            .and_then(|runtime| {
                runtime.block_on(client::transaction_events(0, ACTIVITY_EVENT_LIMIT))
            })
            .map(|events| activity_events_for_transaction(events, transaction_id));
        let _ = sender.send(result);
    });

    glib::timeout_add_local(Duration::from_millis(100), move || {
        match receiver.try_recv() {
            Ok(Ok(events)) => {
                render_transaction_details(&status, &list, &record, &events);
                glib::ControlFlow::Break
            }
            Ok(Err(error)) => {
                status.set_text(&format!("Unable to load transaction details: {error}"));
                glib::ControlFlow::Break
            }
            Err(TryRecvError::Empty) => glib::ControlFlow::Continue,
            Err(TryRecvError::Disconnected) => {
                status.set_text("The transaction detail worker stopped unexpectedly.");
                glib::ControlFlow::Break
            }
        }
    });
}

fn render_transaction_details(
    status: &gtk::Label,
    list: &gtk::ListBox,
    record: &TransactionRecord,
    events: &[TransactionEvent],
) {
    clear_list(list);
    if events.is_empty() {
        status.set_text(&format!(
            "No lifecycle events are currently available for transaction #{}.",
            record.id
        ));
        append_detail_row(list, "Current state", activity_state_label(&record.state));
        append_detail_row(list, "Latest message", &record.message);
        return;
    }

    status.set_text(&format!(
        "{} lifecycle events for {}. This timeline is read only.",
        events.len(),
        activity_title(record)
    ));
    for event in events {
        let row = adw::ActionRow::builder()
            .title(activity_event_title(&event.event))
            .subtitle(format!("Event #{} · {}", event.sequence, event.message))
            .build();
        let badge = gtk::Label::new(Some(activity_state_label(&event.state)));
        badge.add_css_class(if event.level == "error" {
            "error"
        } else {
            activity_state_css_class(&event.state)
        });
        row.add_suffix(&badge);
        if let Some(fraction) = activity_progress_fraction(event.progress_basis_points) {
            let progress = gtk::ProgressBar::new();
            progress.set_fraction(fraction);
            progress.set_width_request(120);
            row.add_suffix(&progress);
        }
        list.append(&row);
    }
}

fn activity_events_for_transaction(
    mut events: Vec<TransactionEvent>,
    transaction_id: u64,
) -> Vec<TransactionEvent> {
    events.retain(|event| event.transaction_id == transaction_id);
    events.sort_by_key(|event| event.sequence);
    events
}

fn activity_event_title(event: &str) -> &str {
    match event {
        "preview-created" => "Preview created",
        "queued" => "Queued",
        "running" => "Simulation started",
        "progress" => "Progress",
        "log" => "APT output",
        "cancellation-requested" => "Cancellation requested",
        "cancelled" => "Cancelled",
        "failed" => "Failed",
        "interrupted" => "Interrupted",
        "completed" => "Completed",
        _ => "Lifecycle event",
    }
}

fn activity_title(record: &TransactionRecord) -> String {
'''
if old_append in text:
    text = text.replace(old_append, new_append, 1)
elif new_append not in text:
    raise SystemExit("Activity detail insertion marker not found")

old_tests_import = '''    use genixbit_package_model::TransactionRecord;

    use super::{activity_progress_fraction, activity_state_label, activity_title};
'''
new_tests_import = '''    use genixbit_package_model::{TransactionEvent, TransactionRecord};

    use super::{
        activity_event_title, activity_events_for_transaction, activity_progress_fraction,
        activity_state_label, activity_title,
    };
'''
if old_tests_import in text:
    text = text.replace(old_tests_import, new_tests_import, 1)
elif new_tests_import not in text:
    raise SystemExit("Activity test import marker not found")

new_tests = '''

    #[test]
    fn filters_and_orders_transaction_timeline_events() {
        let events = vec![
            TransactionEvent {
                sequence: 3,
                transaction_id: 7,
                event: "completed".to_owned(),
                ..TransactionEvent::default()
            },
            TransactionEvent {
                sequence: 1,
                transaction_id: 7,
                event: "queued".to_owned(),
                ..TransactionEvent::default()
            },
            TransactionEvent {
                sequence: 2,
                transaction_id: 8,
                event: "running".to_owned(),
                ..TransactionEvent::default()
            },
        ];
        let result = activity_events_for_transaction(events, 7);
        assert_eq!(
            result.iter().map(|event| event.sequence).collect::<Vec<_>>(),
            [1, 3]
        );
    }

    #[test]
    fn labels_known_and_unknown_lifecycle_events() {
        assert_eq!(activity_event_title("preview-created"), "Preview created");
        assert_eq!(activity_event_title("log"), "APT output");
        assert_eq!(activity_event_title("custom"), "Lifecycle event");
    }
'''
last_test_end = '''    fn shows_progress_only_for_incomplete_active_values() {
        assert_eq!(activity_progress_fraction(0), None);
        assert_eq!(activity_progress_fraction(10_000), None);
        assert_eq!(activity_progress_fraction(5_000), Some(0.5));
    }
}
'''
replacement = last_test_end[:-2] + new_tests + "}\n"
if last_test_end in text:
    text = text.replace(last_test_end, replacement, 1)
elif "fn filters_and_orders_transaction_timeline_events()" not in text:
    raise SystemExit("Activity tests append marker not found")

main.write_text(text)
