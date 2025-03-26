use crate::config::{GlobalConfig, ServerRoleId};
use crossbeam::sync::WaitGroup;
use std::ops::Deref;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::watch::Receiver;
use tokio::sync::watch::error::RecvError;

pub enum Signal {
    None,
    Stop,
}
#[async_trait::async_trait]
pub trait Node: Send + Sync {
    async fn init(
        &self,
        global_config: Arc<GlobalConfig>,
        server_role_id: ServerRoleId,
        mut signal_rx: Receiver<Signal>,
    ) -> anyhow::Result<()> {
        tracing::info!("starting node :{}", server_role_id);

        self.start(global_config, server_role_id.clone()).await?;

        loop {
            let changed = signal_rx.changed().await;
            match changed {
                Ok(_) => {
                    let signal = signal_rx.borrow();
                    match signal.deref() {
                        Signal::Stop => {
                            break;
                        }
                        _ => {}
                    }
                }
                Err(e) => {
                    tracing::error!("signal recv err :{}", server_role_id);
                }
            }
        }
        tracing::info!("stopping node :{}", server_role_id);
        self.stop().await?;
        tracing::info!("stop node :{}", server_role_id);
        Ok(())
    }

    async fn start(
        &self,
        global_config: Arc<GlobalConfig>,
        server_role_id: ServerRoleId,
    ) -> anyhow::Result<()>;

    async fn stop(&self) -> anyhow::Result<()>;
}
