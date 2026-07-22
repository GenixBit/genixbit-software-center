use std::process::{ExitStatus, Stdio};

use anyhow::{Context, bail};
use tokio::{io::AsyncReadExt, sync::watch};

use crate::{
    apt_plan::AptCommandPlan,
    apt_simulation::{AptSimulation, parse_simulation},
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AptSimulationOutcome {
    Completed(AptSimulation),
    Cancelled,
}

enum WaitOutcome {
    Exited(ExitStatus),
    Cancelled,
}

pub async fn run_cancellable(
    operation: &str,
    package: &str,
    mut cancellation: watch::Receiver<bool>,
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
    let stdout_task = tokio::spawn(read_stream(stdout));
    let stderr_task = tokio::spawn(read_stream(stderr));

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

async fn read_stream<R>(mut stream: R) -> anyhow::Result<Vec<u8>>
where
    R: tokio::io::AsyncRead + Unpin,
{
    let mut output = Vec::new();
    stream
        .read_to_end(&mut output)
        .await
        .context("failed to read APT subprocess output")?;
    Ok(output)
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

    use super::wait_for_cancellation;

    #[tokio::test]
    async fn observes_a_cancellation_request() {
        let (sender, mut receiver) = watch::channel(false);
        sender.send(true).expect("cancellation should send");
        wait_for_cancellation(&mut receiver).await;
        assert!(*receiver.borrow());
    }
}
