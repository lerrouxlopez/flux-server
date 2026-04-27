use std::time::Duration;
use tracing::{info, warn};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    telemetry::init();

    let database_url = config::required("DATABASE_URL")?;
    let poll_seconds: u64 = config::parse("WORKER_POLL_SECONDS")?.unwrap_or(5);

    let pool = db::connect(&database_url).await?;
    db::ping(&pool).await?;

    info!("worker started");
    loop {
        // placeholder: consume JetStream / run async jobs later
        warn!("worker tick (no jobs configured yet)");
        tokio::time::sleep(Duration::from_secs(poll_seconds)).await;
    }
}

