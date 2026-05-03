use crate::envelope::EventEnvelope;
use async_nats::Client;
use serde::Serialize;

pub async fn publish<T: Serialize>(
    nats: &Client,
    subject: impl Into<String>,
    envelope: &EventEnvelope<T>,
) -> anyhow::Result<()> {
    let payload = serde_json::to_vec(envelope)?;
    nats.publish(subject.into(), payload.into()).await?;
    Ok(())
}
