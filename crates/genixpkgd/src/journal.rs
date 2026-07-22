use std::{
    fs::{self, OpenOptions},
    io::{self, Write},
    path::{Path, PathBuf},
};

use anyhow::{Context, bail};
use genixbit_package_model::TransactionRecord;

const SYSTEM_JOURNAL_PATH: &str = "/var/lib/genixpkgd/transactions.log";

#[derive(Clone, Debug)]
pub struct TransactionJournal {
    path: PathBuf,
}

impl TransactionJournal {
    pub fn from_environment() -> Self {
        if let Some(path) = std::env::var_os("GENIXPKGD_JOURNAL") {
            return Self::new(PathBuf::from(path));
        }

        let session_bus = std::env::var("GENIXPKGD_BUS").is_ok_and(|value| value == "session");
        if session_bus {
            return Self::new(std::env::temp_dir().join("genixpkgd-transactions.log"));
        }

        Self::new(PathBuf::from(SYSTEM_JOURNAL_PATH))
    }

    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    pub fn append(&self, record: &TransactionRecord) -> anyhow::Result<()> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent).with_context(|| {
                format!("failed to create transaction journal directory {}", parent.display())
            })?;
        }

        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
            .with_context(|| {
                format!("failed to open transaction journal {}", self.path.display())
            })?;
        writeln!(file, "{}", encode(record)).with_context(|| {
            format!("failed to append transaction journal {}", self.path.display())
        })?;
        file.sync_data().with_context(|| {
            format!("failed to sync transaction journal {}", self.path.display())
        })?;
        Ok(())
    }

    pub fn read_all(&self) -> anyhow::Result<Vec<TransactionRecord>> {
        let content = match fs::read_to_string(&self.path) {
            Ok(content) => content,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(error) => {
                return Err(error).with_context(|| {
                    format!("failed to read transaction journal {}", self.path.display())
                });
            }
        };

        content
            .lines()
            .enumerate()
            .filter(|(_, line)| !line.trim().is_empty())
            .map(|(index, line)| {
                decode(line).with_context(|| {
                    format!(
                        "invalid transaction journal entry {} in {}",
                        index + 1,
                        self.path.display()
                    )
                })
            })
            .collect()
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

fn encode(record: &TransactionRecord) -> String {
    [
        record.id.to_string(),
        record.preview_id.to_string(),
        sanitize(&record.kind),
        sanitize(&record.package),
        sanitize(&record.state),
        record.progress_basis_points.to_string(),
        record.can_cancel.to_string(),
        record.created_unix_ms.to_string(),
        record.updated_unix_ms.to_string(),
        sanitize(&record.message),
    ]
    .join("|")
}

fn decode(line: &str) -> anyhow::Result<TransactionRecord> {
    let fields = line.splitn(10, '|').collect::<Vec<_>>();
    if fields.len() != 10 {
        bail!("expected 10 journal fields, found {}", fields.len());
    }

    Ok(TransactionRecord {
        id: fields[0].parse().context("invalid transaction id")?,
        preview_id: fields[1].parse().context("invalid preview id")?,
        kind: fields[2].to_owned(),
        package: fields[3].to_owned(),
        state: fields[4].to_owned(),
        progress_basis_points: fields[5].parse().context("invalid progress value")?,
        can_cancel: fields[6].parse().context("invalid cancellation flag")?,
        created_unix_ms: fields[7].parse().context("invalid creation timestamp")?,
        updated_unix_ms: fields[8].parse().context("invalid update timestamp")?,
        message: fields[9].to_owned(),
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

    use genixbit_package_model::TransactionRecord;

    use super::TransactionJournal;

    #[test]
    fn appends_and_reads_transaction_records() {
        let unique = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .expect("clock should be after the Unix epoch")
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "genixpkgd-journal-test-{}-{unique}.log",
            process::id()
        ));
        let journal = TransactionJournal::new(path.clone());
        let record = TransactionRecord {
            id: 7,
            preview_id: 3,
            kind: "install".to_owned(),
            package: "curl".to_owned(),
            state: "queued".to_owned(),
            progress_basis_points: 0,
            can_cancel: true,
            created_unix_ms: 100,
            updated_unix_ms: 100,
            message: "Waiting safely".to_owned(),
        };

        journal.append(&record).expect("journal append should work");
        assert_eq!(journal.read_all().expect("journal read should work"), [record]);
        fs::remove_file(path).expect("test journal should be removable");
    }
}
