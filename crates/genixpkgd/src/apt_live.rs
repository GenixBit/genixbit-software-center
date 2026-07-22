use std::process::{ExitStatus, Stdio};

use anyhow::{Context, bail};
use tokio::{
    io::{AsyncBufReadExt, AsyncRead, BufReader},
    sync::{mpsc, watch},
};

use crate::{
    apt_plan::AptCommandPlan,
    apt_simulation::{AptSimulation, parse_simulation},
};

const MAX_CAPTURE_BYTES: usize = 1_048_576;
const MAX_LOG_CHARACTERS: usize = 4_096;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AptSimulationOutcome {
    Completed(AptSimulation),
    Cancelled,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AptSimulationLog {
    pub level: String,
    pub message: String,
    pub progress_basis_points: Option<u32>,
}

enum WaitOutcome {
    Exited(ExitStatus),
    Cancelled,
}

pub async fn run_cancellable(
    operation: &str,
    package: &str,
    mut cancellation: watch::Receiver<bool>,
    logs: mpsc::Sender<AptSimulationLog>,
) -> anyhow::Result<AptSimulationOutcome> {
    let plan = AptCommandPlan::simulation(operation, package)?;
    let mut command = plan.command()?;
    command.stdout(Stdio::piped()).stderr(Stdio::piped());

    let mut child = command
        .spawn()
        .with_context(|| format!("failed to start APT simulation for {package}"))?;
    let stdout = child
        .stdout
        .take()
        .context("APT simulation stdout pipe was not available")?;
    let stderr = child
        .stderr
        .take()
        .context("APT simulation stderr pipe was not available")?;
    let stdout_task = tokio::spawn(read_stream(stdout, "info", logs.clone()));
    let stderr_task = tokio::spawn(read_stream(stderr, "error", logs));

    let outcome = tokio::select! {
        status = child.wait() => WaitOutcome::Exited(status.context("failed to wait for APT simulation")?),
        () = wait_for_cancellation(&mut cancellation) => {
            child.kill().await.context("failed to terminate cancelled APT simulation")?;
            let _ = child.wait().await;
            WaitOutcome::Cancelled
        }
    };

    let stdout = stdout_task
        .await
        .context("APT stdout reader task failed")??;
    let stderr = stderr_task
        .await
        .context("APT stderr reader task failed")??;

    match outcome {
        WaitOutcome::Cancelled => Ok(AptSimulationOutcome::Cancelled),
        WaitOutcome::Exited(status) if status.success() => Ok(AptSimulationOutcome::Completed(
            parse_simulation(&String::from_utf8_lossy(&stdout)),
        )),
        WaitOutcome::Exited(_) => bail!(
            "APT {operation} simulation failed for {package}: {}",
            String::from_utf8_lossy(&stderr).trim()
        ),
    }
}

async fn read_stream<R>(
    stream: R,
    level: &'static str,
    logs: mpsc::Sender<AptSimulationLog>,
) -> anyhow::Result<Vec<u8>>
where
    R: AsyncRead + Unpin,
{
    let mut lines = BufReader::new(stream).lines();
    let mut captured = Vec::new();
    while let Some(line) = lines
        .next_line()
        .await
        .context("failed to read APT subprocess output")?
    {
        let message = bounded_log_line(&line);
        if !message.trim().is_empty() {
            let _ = logs
                .send(AptSimulationLog {
                    level: level.to_owned(),
                    progress_basis_points: if level == "info" {
                        progress_for_line(&line)
                    } else {
                        None
                    },
                    message,
                })
                .await;
        }
        if captured.len() < MAX_CAPTURE_BYTES {
            let remaining = MAX_CAPTURE_BYTES - captured.len();
            let bytes = line.as_bytes();
            captured.extend_from_slice(&bytes[..bytes.len().min(remaining)]);
            if captured.len() < MAX_CAPTURE_BYTES {
                captured.push(b'\n');
            }
        }
    }
    Ok(captured)
}

fn progress_for_line(line: &str) -> Option<u32> {
    let line = line.trim();
    if line.starts_with("Reading package lists") {
        Some(2_000)
    } else if line.starts_with("Building dependency tree") {
        Some(3_000)
    } else if line.starts_with("Reading state information") {
        Some(4_000)
    } else if line.starts_with("Calculating upgrade") {
        Some(5_000)
    } else if line.starts_with("The following ") {
        Some(5_500)
    } else if line.starts_with("Inst ") || line.starts_with("Remv ") {
        Some(7_000)
    } else if line.starts_with("Conf ") {
        Some(8_000)
    } else if line.contains(" upgraded,")
        && line.contains(" newly installed,")
        && line.contains(" to remove")
    {
        Some(8_500)
    } else {
        None
    }
}

fn bounded_log_line(line: &str) -> String {
    let mut message = line.chars().take(MAX_LOG_CHARACTERS).collect::<String>();
    if line.chars().count() > MAX_LOG_CHARACTERS {
        message.push('…');
    }
    message
}

async fn wait_for_cancellation(cancellation: &mut watch::Receiver<bool>) {
    while !*cancellation.borrow() {
        if cancellation.changed().await.is_err() {
            std::future::pending::<()>().await;
        }
    }
}

#[cfg(test)]
mod tests {
    use tokio::sync::watch;

    use super::{MAX_LOG_CHARACTERS, bounded_log_line, progress_for_line, wait_for_cancellation};

    #[tokio::test]
    async fn observes_a_cancellation_request() {
        let (sender, mut receiver) = watch::channel(false);
        sender.send(true).expect("cancellation should send");
        wait_for_cancellation(&mut receiver).await;
        assert!(*receiver.borrow());
    }

    #[test]
    fn bounds_subprocess_log_lines() {
        let line = "x".repeat(MAX_LOG_CHARACTERS + 10);
        let bounded = bounded_log_line(&line);
        assert_eq!(bounded.chars().count(), MAX_LOG_CHARACTERS + 1);
        assert!(bounded.ends_with('…'));
    }

    #[test]
    fn classifies_deterministic_apt_progress_stages() {
        let cases = [
            ("Reading package lists...", Some(2_000)),
            ("Building dependency tree...", Some(3_000)),
            ("Reading state information...", Some(4_000)),
            ("Calculating upgrade...", Some(5_000)),
            ("The following NEW packages will be installed:", Some(5_500)),
            ("Inst curl (8.0 stable [amd64])", Some(7_000)),
            ("Remv nano [7.2]", Some(7_000)),
            ("Conf curl (8.0 stable [amd64])", Some(8_000)),
            (
                "1 upgraded, 2 newly installed, 0 to remove and 4 not upgraded.",
                Some(8_500),
            ),
            ("Need to get 1 MB of archives.", None),
        ];
        for (line, expected) in cases {
            assert_eq!(progress_for_line(line), expected, "{line}");
        }
    }
}
