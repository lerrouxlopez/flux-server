use async_nats::Client;

pub async fn connect(url: &str) -> anyhow::Result<Client> {
    Ok(async_nats::connect(url).await?)
}

