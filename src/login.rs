use qs001_server::start;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    start().await
}
