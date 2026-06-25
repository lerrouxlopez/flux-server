#![forbid(unsafe_code)]

use futures::stream::{self, BoxStream};
use lorelei_core::error::LoreleiError;
use lorelei_core::traits::SongProvider;
use lorelei_core::types::{
    EmbeddingRequest, EmbeddingResponse, ProviderCapabilities, SongChunk, SongRequest,
    SongResponse,
};
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};
use tokio::time::sleep;
use tracing::{debug, info};

const DEFAULT_BASE_URL: &str = "https://api.anthropic.com";
const ANTHROPIC_VERSION: &str = "2023-06-01";
const DEFAULT_MAX_TOKENS: u32 = 4096;

pub struct AnthropicProvider {
    pub name: String,
    pub base_url: String,
    pub api_key: String,
    pub chat_model: String,
    pub capabilities: ProviderCapabilities,
    client: reqwest::Client,
}

impl AnthropicProvider {
    pub fn new(
        name: impl Into<String>,
        base_url: Option<String>,
        api_key: impl Into<String>,
        chat_model: impl Into<String>,
        capabilities: ProviderCapabilities,
    ) -> Result<Self, LoreleiError> {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(60))
            .build()
            .map_err(|e| LoreleiError::Internal(format!("http client init failed: {e}")))?;
        Ok(Self {
            name: name.into(),
            base_url: base_url
                .unwrap_or_else(|| DEFAULT_BASE_URL.to_string())
                .trim_end_matches('/')
                .to_string(),
            api_key: api_key.into(),
            chat_model: chat_model.into(),
            capabilities,
            client,
        })
    }

    fn log_prompts_enabled() -> bool {
        std::env::var("LORELEI_LOG_PROMPTS")
            .ok()
            .is_some_and(|v| v == "1" || v.eq_ignore_ascii_case("true"))
    }

    fn endpoint(&self, subpath: &str) -> String {
        format!("{}/{}", self.base_url, subpath.trim_start_matches('/'))
    }

    async fn request_with_retry(
        &self,
        url: String,
        body: &MessagesRequest,
    ) -> Result<(MessagesResponse, u32, Duration), LoreleiError> {
        let started = Instant::now();
        let mut attempt = 0u32;
        let mut delay_ms = 250u64;
        loop {
            attempt += 1;
            let res = self
                .client
                .post(&url)
                .header("x-api-key", &self.api_key)
                .header("anthropic-version", ANTHROPIC_VERSION)
                .json(body)
                .send()
                .await;

            match res {
                Ok(resp) => {
                    if resp.status().is_success() {
                        let parsed = resp.json::<MessagesResponse>().await.map_err(|e| {
                            LoreleiError::Provider(format!("invalid response: {e}"))
                        })?;
                        let retries = attempt.saturating_sub(1);
                        return Ok((parsed, retries, started.elapsed()));
                    }
                    if is_retryable_status(resp.status()) && attempt < 5 {
                        sleep(Duration::from_millis(delay_ms)).await;
                        delay_ms = (delay_ms * 2).min(4000);
                        continue;
                    }

                    let status = resp.status();
                    let msg = format!("provider `{}` http {}", self.name, status.as_u16());
                    return Err(LoreleiError::Provider(msg));
                }
                Err(e) => {
                    if attempt < 5 {
                        sleep(Duration::from_millis(delay_ms)).await;
                        delay_ms = (delay_ms * 2).min(4000);
                        continue;
                    }
                    return Err(LoreleiError::Provider(format!(
                        "provider `{}` request failed: {}",
                        self.name, e
                    )));
                }
            }
        }
    }
}

#[async_trait::async_trait]
impl SongProvider for AnthropicProvider {
    fn capabilities(&self) -> ProviderCapabilities {
        self.capabilities.clone()
    }

    async fn complete(&self, request: SongRequest) -> Result<SongResponse, LoreleiError> {
        let url = self.endpoint("v1/messages");

        if Self::log_prompts_enabled() {
            debug!(
                run_id = %request.run_id.0,
                provider = %self.name,
                model = %self.chat_model,
                prompt = %request.input,
                "song.prompt"
            );
        } else {
            debug!(
                run_id = %request.run_id.0,
                provider = %self.name,
                model = %self.chat_model,
                prompt_len = request.input.len(),
                "song.prompt_redacted"
            );
        }

        let body = MessagesRequest {
            model: self.chat_model.clone(),
            max_tokens: DEFAULT_MAX_TOKENS,
            messages: vec![AnthropicMessage {
                role: "user".to_string(),
                content: request.input.clone(),
            }],
            temperature: request.temperature,
        };

        let (resp, retries, latency) = self.request_with_retry(url, &body).await?;
        let output = resp
            .content
            .into_iter()
            .filter_map(|block| (block.block_type == "text").then_some(block.text))
            .collect::<Vec<_>>()
            .join("");

        info!(
            run_id = %request.run_id.0,
            provider = %self.name,
            model = %self.chat_model,
            latency_ms = latency.as_millis() as u64,
            retry_count = retries,
            input_tokens = resp.usage.as_ref().map(|u| u.input_tokens),
            output_tokens = resp.usage.as_ref().map(|u| u.output_tokens),
            "song.complete"
        );

        Ok(SongResponse {
            output,
            reasoning_summary: None,
            tool_calls: vec![],
        })
    }

    async fn stream(
        &self,
        request: SongRequest,
    ) -> Result<BoxStream<'static, SongChunk>, LoreleiError> {
        if !self.capabilities.supports_streaming {
            return Err(LoreleiError::Unsupported(
                "provider does not support streaming".to_string(),
            ));
        }

        // Minimal streaming: fall back to non-streaming and chunk the result.
        let full = self.complete(request).await?;
        let chunks: Vec<SongChunk> = full
            .output
            .as_bytes()
            .chunks(16)
            .map(|c| SongChunk {
                delta: String::from_utf8_lossy(c).to_string(),
                done: false,
            })
            .collect();
        let mut all = chunks;
        all.push(SongChunk {
            delta: String::new(),
            done: true,
        });
        Ok(Box::pin(stream::iter(all)))
    }

    async fn embed(&self, _request: EmbeddingRequest) -> Result<EmbeddingResponse, LoreleiError> {
        Err(LoreleiError::Unsupported(
            "anthropic provider does not support embeddings".to_string(),
        ))
    }
}

fn is_retryable_status(status: StatusCode) -> bool {
    matches!(
        status,
        StatusCode::REQUEST_TIMEOUT
            | StatusCode::TOO_MANY_REQUESTS
            | StatusCode::BAD_GATEWAY
            | StatusCode::SERVICE_UNAVAILABLE
            | StatusCode::GATEWAY_TIMEOUT
    ) || status.is_server_error()
}

#[derive(Debug, Serialize)]
struct MessagesRequest {
    model: String,
    max_tokens: u32,
    messages: Vec<AnthropicMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
}

#[derive(Debug, Serialize)]
struct AnthropicMessage {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct MessagesResponse {
    content: Vec<ContentBlock>,
    #[serde(default)]
    usage: Option<AnthropicUsage>,
}

#[derive(Debug, Deserialize)]
struct ContentBlock {
    #[serde(rename = "type")]
    block_type: String,
    #[serde(default)]
    text: String,
}

#[derive(Debug, Deserialize)]
struct AnthropicUsage {
    #[serde(default)]
    input_tokens: u32,
    #[serde(default)]
    output_tokens: u32,
}
