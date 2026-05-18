use crate::envelope::EventEnvelope;
use async_nats::Client;
use serde::Serialize;

pub async fn publish<T: Serialize>(
    nats: &Client,
    subject: impl Into<String>,
    envelope: &EventEnvelope<T>,
) -> anyhow::Result<()> {
    let payload = serde_json::to_vec(envelope)?;
    let subj = subject.into();
    metrics::counter!("nats_publish_attempts_total").increment(1);
    match nats.publish(subj, payload.into()).await {
        Ok(_) => Ok(()),
        Err(e) => {
            metrics::counter!("nats_publish_failures_total").increment(1);
            Err(e.into())
        }
    }
}
