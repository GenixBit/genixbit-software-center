use anyhow::{Context, bail};
use tokio::sync::{Mutex, watch};

#[derive(Clone, Debug)]
struct ActiveSimulation {
    transaction_id: u64,
    cancellation: watch::Sender<bool>,
}

#[derive(Debug, Default)]
pub struct SimulationControl {
    active: Mutex<Option<ActiveSimulation>>,
}

impl SimulationControl {
    pub async fn register(&self, transaction_id: u64) -> anyhow::Result<watch::Receiver<bool>> {
        let mut active = self.active.lock().await;
        if active.is_some() {
            bail!("a simulation cancellation handle is already registered");
        }
        let (cancellation, receiver) = watch::channel(false);
        *active = Some(ActiveSimulation {
            transaction_id,
            cancellation,
        });
        Ok(receiver)
    }

    pub async fn request(&self, transaction_id: u64) -> anyhow::Result<()> {
        let active = self.active.lock().await;
        let active = active
            .as_ref()
            .context("no active simulation cancellation handle is registered")?;
        if active.transaction_id != transaction_id {
            bail!("transaction {transaction_id} is not the registered active simulation");
        }
        active
            .cancellation
            .send(true)
            .map_err(|_| anyhow::anyhow!("the active simulation already stopped"))
    }

    pub async fn clear(&self, transaction_id: u64) -> anyhow::Result<()> {
        let mut active = self.active.lock().await;
        if let Some(current) = active.as_ref()
            && current.transaction_id != transaction_id
        {
            bail!(
                "transaction {transaction_id} cannot clear simulation {}",
                current.transaction_id
            );
        }
        *active = None;
        Ok(())
    }

    pub async fn is_active(&self, transaction_id: u64) -> bool {
        self.active
            .lock()
            .await
            .as_ref()
            .is_some_and(|active| active.transaction_id == transaction_id)
    }
}

#[cfg(test)]
mod tests {
    use super::SimulationControl;

    #[tokio::test]
    async fn registers_requests_and_clears_one_simulation() {
        let control = SimulationControl::default();
        let receiver = control.register(7).await.expect("handle should register");
        assert!(control.is_active(7).await);
        control.request(7).await.expect("cancellation should send");
        assert!(*receiver.borrow());
        control.clear(7).await.expect("handle should clear");
        assert!(!control.is_active(7).await);
    }

    #[tokio::test]
    async fn rejects_mismatched_and_duplicate_controls() {
        let control = SimulationControl::default();
        let _receiver = control.register(7).await.expect("handle should register");
        assert!(control.register(8).await.is_err());
        assert!(control.request(8).await.is_err());
        assert!(control.clear(8).await.is_err());
    }
}
