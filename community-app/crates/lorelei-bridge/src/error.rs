#[derive(Debug, thiserror::Error)]
pub enum BridgeError {
    #[error("database error: {0}")]
    Database(#[from] sqlx::Error),
    #[error("credential error: {0}")]
    Secrets(#[from] secrets::SecretsError),
    #[error("lorelei is not enabled for this organization")]
    NotEnabled,
    #[error("no usable LLM credential is available for this request")]
    NoCredentialAvailable,
    #[error("request to lorelei-harbor failed: {0}")]
    Http(#[from] reqwest::Error),
    #[error("lorelei-harbor returned an error: {0}")]
    Harbor(String),
}
