from pathlib import Path

path = Path("crates/software-center/src/main.rs")
text = path.read_text()


def replace_once(old: str, new: str) -> None:
    global text
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"expected one marker, found {count}: {old[:100]!r}")
    text = text.replace(old, new, 1)


replace_once(
    "mod activity_filter;\nmod client;\n",
    "mod activity_filter;\nmod activity_time;\nmod client;\n",
)
replace_once(
    "use activity_filter::{ALL_OPERATIONS, ALL_STATES, filter_records, summarize_records};\n",
    "use activity_filter::{ALL_OPERATIONS, ALL_STATES, filter_records, summarize_records};\nuse activity_time::{current_unix_ms, timing_text};\n",
)
replace_once(
    '''    ui.activity_status.set_text(&format!(
        "Showing {} of {} recent transactions. Package execution remains simulation-only.",
        filtered.len(),
        records.len()
    ));
    for record in filtered {
''',
    '''    ui.activity_status.set_text(&format!(
        "Showing {} of {} recent transactions. Package execution remains simulation-only.",
        filtered.len(),
        records.len()
    ));
    let now_unix_ms = current_unix_ms();
    for record in filtered {
''',
)
replace_once(
    '''            .title(activity_title(record))
            .subtitle(format!("Transaction #{} · {}", record.id, record.message))
            .activatable(true)
''',
    '''            .title(activity_title(record))
            .subtitle(format!(
                "Transaction #{} · {} · {}",
                record.id,
                record.message,
                timing_text(record, now_unix_ms)
            ))
            .activatable(true)
''',
)
replace_once(
    '''        status.set_text(&format!(
            "No lifecycle events are currently available for transaction #{}.",
            record.id
        ));
''',
    '''        status.set_text(&format!(
            "No lifecycle events are currently available for transaction #{}. {}.",
            record.id,
            timing_text(record, current_unix_ms())
        ));
''',
)
replace_once(
    '''    status.set_text(&format!(
        "{} lifecycle events for {}. This timeline is read only.",
        events.len(),
        activity_title(record)
    ));
''',
    '''    status.set_text(&format!(
        "{} lifecycle events for {}. {}. This timeline is read only.",
        events.len(),
        activity_title(record),
        timing_text(record, current_unix_ms())
    ));
''',
)

path.write_text(text)
