use async_nats::Client;

pub mod core;
pub mod envelope;
pub mod jetstream;
pub mod subjects;

pub async fn connect(url: &str) -> anyhow::Result<Client> {
    Ok(async_nats::connect(url).await?)
}
