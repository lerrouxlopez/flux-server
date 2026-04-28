use tracing::{info, warn};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    telemetry::init();

    let cfg = config::AppConfig::from_env()?;
    let nats = events::connect(&cfg.nats_url).await?;
    info!("worker connected to NATS");

    // Placeholder: subscribe to internal events (JetStream consumers later).
    // Keep worker running.
    loop {
        tokio::time::sleep(std::time::Duration::from_secs(30)).await;
        warn!("worker heartbeat (no consumers configured yet)");
        let _ = &nats;
    }
}
