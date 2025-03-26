use common::config::ServerRoleId;
use std::ops::Deref;
use tokio::sync::watch::Receiver;

pub mod game;
pub mod gate;
pub mod login;
pub mod world;

pub enum Signal {
    None,
    Stop,
}
#[async_trait::async_trait]
pub trait Node: Send + Sync {
    async fn init(&mut self, mut signal_rx: Receiver<Signal>) -> anyhow::Result<()> {
        tracing::info!("starting node :{}", self.server_role_id());

        self.start().await?;
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
                    tracing::error!("signal recv err :{}", self.server_role_id());
                }
            }
        }
        tracing::info!("stopping node :{}", self.server_role_id());
        self.stop().await?;
        tracing::info!("stop node :{}", self.server_role_id());
        Ok(())
    }

    async fn start(&mut self) -> anyhow::Result<()>;

    async fn stop(&mut self) -> anyhow::Result<()>;

    fn server_role_id(&self) -> ServerRoleId;
}
