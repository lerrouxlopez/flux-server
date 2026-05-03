use async_nats::jetstream::{self, consumer, stream};

#[derive(Debug, Clone)]
pub struct JetStreamConfig {
    pub notifications_stream: String,
    pub audit_stream: String,
    pub search_stream: String,
    pub email_stream: String,
    pub cleanup_stream: String,
}

impl Default for JetStreamConfig {
    fn default() -> Self {
        Self {
            notifications_stream: "notifications".to_string(),
            audit_stream: "audit".to_string(),
            search_stream: "search".to_string(),
            email_stream: "email".to_string(),
            cleanup_stream: "cleanup".to_string(),
        }
    }
}

pub fn context(client: async_nats::Client) -> jetstream::Context {
    jetstream::new(client)
}

pub async fn ensure_streams(js: &jetstream::Context, cfg: &JetStreamConfig) -> anyhow::Result<()> {
    ensure_stream(
        js,
        &cfg.notifications_stream,
        vec!["org.*.user.*.notification.created"],
    )
    .await?;
    ensure_stream(js, &cfg.audit_stream, vec!["org.*.>"]).await?;
    ensure_stream(js, &cfg.search_stream, vec!["org.*.channel.*.message.*"]).await?;
    ensure_stream(
        js,
        &cfg.email_stream,
        vec!["org.*.user.*.notification.created"],
    )
    .await?;
    ensure_stream(js, &cfg.cleanup_stream, vec!["cleanup.>"]).await?;
    Ok(())
}

async fn ensure_stream(
    js: &jetstream::Context,
    name: &str,
    subjects: Vec<&'static str>,
) -> anyhow::Result<()> {
    let subjects: Vec<String> = subjects.into_iter().map(|s| s.to_string()).collect();
    let config = stream::Config {
        name: name.to_string(),
        subjects,
        storage: stream::StorageType::File,
        retention: stream::RetentionPolicy::Limits,
        max_age: std::time::Duration::from_secs(14 * 24 * 60 * 60),
        ..Default::default()
    };

    match js.get_stream(name).await {
        Ok(_stream) => {
            // Keep existing stream config as-is; migrations happen explicitly when needed.
        }
        Err(_) => {
            js.create_stream(config).await?;
        }
    }
    Ok(())
}

pub async fn ensure_durable_consumer(
    js: &jetstream::Context,
    stream: &str,
    durable_name: &str,
    filter_subject: Option<&str>,
) -> anyhow::Result<consumer::Consumer<consumer::pull::Config>> {
    let s = js.get_stream(stream).await?;

    let cfg = consumer::pull::Config {
        durable_name: Some(durable_name.to_string()),
        filter_subject: filter_subject.unwrap_or("").to_string(),
        ack_policy: consumer::AckPolicy::Explicit,
        ack_wait: std::time::Duration::from_secs(30),
        max_deliver: 10,
        ..Default::default()
    };

    match s.get_consumer(durable_name).await {
        Ok(c) => Ok(c),
        Err(_) => Ok(s.create_consumer(cfg).await?),
    }
}
