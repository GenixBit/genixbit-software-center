use std::{
    fs::{self, OpenOptions},
    io::{self, Write},
    path::{Path, PathBuf},
};

use anyhow::{Context, bail};
use genixbit_package_model::TransactionEvent;

const MAX_PERSISTED_EVENTS: usize = 1_000;
const COMPACT_AFTER_BYTES: u64 = 4 * 1_048_576;

#[derive(Clone, Debug)]
pub struct EventJournal {
    path: PathBuf,
}

impl EventJournal {
    pub fn from_transaction_journal(transaction_path: &Path) -> Self {
        let mut path = transaction_path.to_path_buf();
        let file_name = transaction_path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| "transactions.log".to_owned());
        path.set_file_name(format!("{file_name}.events"));
        Self { path }
    }

    pub fn append(&self, event: &TransactionEvent) -> anyhow::Result<()> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent).with_context(|| {
                format!(
                    "failed to create transaction event journal directory {}",
                    parent.display()
                )
            })?;
        }

        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
            .with_context(|| {
                format!(
                    "failed to open transaction event journal {}",
                    self.path.display()
                )
            })?;
        writeln!(file, "{}", encode(event)).with_context(|| {
            format!(
                "failed to append transaction event journal {}",
                self.path.display()
            )
        })?;
        file.sync_data().with_context(|| {
            format!(
                "failed to sync transaction event journal {}",
                self.path.display()
            )
        })?;
        drop(file);

        if fs::metadata(&self.path)
            .map(|metadata| metadata.len() > COMPACT_AFTER_BYTES)
            .unwrap_or(false)
        {
            self.compact()?;
        }
        Ok(())
    }

    pub fn read_recent(&self, limit: usize) -> anyhow::Result<Vec<TransactionEvent>> {
        if limit == 0 {
            return Ok(Vec::new());
        }
        let content = match fs::read_to_string(&self.path) {
            Ok(content) => content,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(error) => {
                return Err(error).with_context(|| {
                    format!(
                        "failed to read transaction event journal {}",
                        self.path.display()
                    )
                });
            }
        };

        let events = content
            .lines()
            .enumerate()
            .filter(|(_, line)| !line.trim().is_empty())
            .map(|(index, line)| {
                decode(line).with_context(|| {
                    format!(
                        "invalid transaction event journal entry {} in {}",
                        index + 1,
                        self.path.display()
                    )
                })
            })
            .collect::<anyhow::Result<Vec<_>>>()?;
        let start = events.len().saturating_sub(limit);
        Ok(events[start..].to_vec())
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    fn compact(&self) -> anyhow::Result<()> {
        let events = self.read_recent(MAX_PERSISTED_EVENTS)?;
        let temporary = self.path.with_extension("events.tmp");
        {
            let mut file = fs::File::create(&temporary).with_context(|| {
                format!(
                    "failed to create compacted transaction event journal {}",
                    temporary.display()
                )
            })?;
            for event in &events {
                writeln!(file, "{}", encode(event))?;
            }
            file.sync_data()?;
        }
        fs::rename(&temporary, &self.path).with_context(|| {
            format!(
                "failed to replace transaction event journal {}",
                self.path.display()
            )
        })?;
        Ok(())
    }
}

fn encode(event: &TransactionEvent) -> String {
    [
        event.sequence.to_string(),
        sanitize(&event.event),
        event.transaction_id.to_string(),
        event.preview_id.to_string(),
        sanitize(&event.kind),
        sanitize(&event.package),
        sanitize(&event.state),
        event.progress_basis_points.to_string(),
        sanitize(&event.level),
        sanitize(&event.message),
        event.created_unix_ms.to_string(),
    ]
    .join("|")
}

fn decode(line: &str) -> anyhow::Result<TransactionEvent> {
    let fields = line.splitn(11, '|').collect::<Vec<_>>();
    if fields.len() != 11 {
        bail!("expected 11 event fields, found {}", fields.len());
    }

    Ok(TransactionEvent {
        sequence: fields[0].parse().context("invalid event sequence")?,
        event: fields[1].to_owned(),
        transaction_id: fields[2].parse().context("invalid transaction id")?,
        preview_id: fields[3].parse().context("invalid preview id")?,
        kind: fields[4].to_owned(),
        package: fields[5].to_owned(),
        state: fields[6].to_owned(),
        progress_basis_points: fields[7].parse().context("invalid progress value")?,
        level: fields[8].to_owned(),
        message: fields[9].to_owned(),
        created_unix_ms: fields[10].parse().context("invalid event timestamp")?,
    })
}

fn sanitize(value: &str) -> String {
    value
        .chars()
        .map(|character| match character {
            '|' | '\n' | '\r' => ' ',
            other => other,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use std::{fs, process, time::SystemTime};

    use genixbit_package_model::TransactionEvent;

    use super::EventJournal;

    fn event(sequence: u64) -> TransactionEvent {
        TransactionEvent {
            sequence,
            event: "queued".to_owned(),
            transaction_id: sequence,
            preview_id: sequence,
            kind: "install".to_owned(),
            package: "curl".to_owned(),
            state: "queued".to_owned(),
            progress_basis_points: 0,
            level: "info".to_owned(),
            message: format!("Queued event {sequence}"),
            created_unix_ms: 100 + sequence,
        }
    }

    #[test]
    fn appends_and_reads_recent_events() {
        let unique = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .expect("clock should be after the Unix epoch")
            .as_nanos();
        let transaction_path = std::env::temp_dir().join(format!(
            "genixpkgd-event-journal-test-{}-{unique}.log",
            process::id()
        ));
        let journal = EventJournal::from_transaction_journal(&transaction_path);
        for sequence in 1..=5 {
            journal
                .append(&event(sequence))
                .expect("event should append");
        }

        assert_eq!(
            journal.read_recent(3).expect("events should load"),
            [event(3), event(4), event(5)]
        );
        fs::remove_file(journal.path()).expect("event journal should be removable");
    }

    #[test]
    fn sanitizes_event_fields() {
        let unique = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .expect("clock should be after the Unix epoch")
            .as_nanos();
        let transaction_path = std::env::temp_dir().join(format!(
            "genixpkgd-event-sanitize-test-{}-{unique}.log",
            process::id()
        ));
        let journal = EventJournal::from_transaction_journal(&transaction_path);
        let mut value = event(1);
        value.message = "line|one\nline two".to_owned();
        journal.append(&value).expect("event should append");
        let restored = journal.read_recent(1).expect("event should load");
        assert_eq!(restored[0].message, "line one line two");
        fs::remove_file(journal.path()).expect("event journal should be removable");
    }
}
