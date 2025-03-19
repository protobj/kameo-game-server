use crossbeam::sync::WaitGroup;
use std::time::Duration;

pub const DEFAULT_STOP_TIMEOUT: Duration = Duration::from_secs(60);

pub trait Service {
    fn name(&self) -> &str;
    async fn init(&mut self, wait_for_stop: Option<WaitGroup>) -> anyhow::Result<()> {
        self.start().await?;
        Ok(self.wait_stop(wait_for_stop).await)
    }
    async fn start(&mut self) -> anyhow::Result<()>;
    async fn wait_stop(&mut self, wait_for_stop: Option<WaitGroup>) {
        if let Some(signal) = wait_for_stop {
            signal.wait()
        } else {
            let ctrl_c = async {
                tokio::signal::ctrl_c()
                    .await
                    .expect(format!("[{}]Failed to install Ctrl+C handler", self.name()).as_str());
            };
            #[cfg(unix)]
            let terminate = async {
                use tokio::signal::unix::{SignalKind, signal};
                let mut sigterm = signal(SignalKind::terminate())
                    .expect(format!("[{}]failed to install signal", self.name()).as_str());

                sigterm.recv().await;
            };
            #[cfg(not(unix))]
            let terminate = std::future::pending::<()>();

            tokio::select! {
                _ = ctrl_c => {},
                _ = terminate => {},
            };
        }
        match tokio::time::timeout(DEFAULT_STOP_TIMEOUT, self.stop()).await {
            Ok(_) => {
                tracing::info!("[{}]service stopped", self.name());
            }
            Err(err) => tracing::error!(error = ?err, "[{}]stop", self.name()),
        }
    }

    async fn stop(&mut self);
}
