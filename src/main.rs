use qs001_server::start;
#[tokio::main]
pub async fn main() -> anyhow::Result<()> {
    start().await
}
