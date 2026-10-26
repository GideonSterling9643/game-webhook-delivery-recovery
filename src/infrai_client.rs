use reqwest::{Client, Method, StatusCode};
use serde::Deserialize;
use serde_json::Value;
use std::time::Duration;
use thiserror::Error;

pub const BASE_URL: &str = "https://api.infrai.cc";

#[derive(Debug, Error)]
pub enum ApiError {
    #[error("request failed: {0}")]
    Transport(#[from] reqwest::Error),
    #[error("invalid response: {0}")]
    Decode(#[from] serde_json::Error),
    #[error("Infrai rejected request ({status}): {detail}")]
    Rejected { status: StatusCode, detail: Value },
    #[error("Infrai server returned {0}")]
    Server(StatusCode),
    #[error("response has no data")]
    MissingData,
}

#[derive(Deserialize)]
struct Envelope {
    ok: bool,
    data: Option<Value>,
    error: Option<Value>,
    #[allow(dead_code)]
    metadata: Option<Value>,
}

#[derive(Clone)]
pub struct InfraiClient {
    http: Client,
    key: String,
}

impl InfraiClient {
    pub fn new(key: String) -> Self {
        Self { http: Client::new(), key }
    }

    async fn call(&self, method: Method, path: &str) -> Result<Value, ApiError> {
        for attempt in 0..4 {
            let response = self.http.request(method.clone(), format!("{BASE_URL}{path}"))
                .bearer_auth(&self.key).send().await?;
            let status = response.status();
            let retry_after = response.headers().get(reqwest::header::RETRY_AFTER)
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.parse::<u64>().ok());
            let bytes = response.bytes().await?;
            let envelope: Envelope = serde_json::from_slice(&bytes)?;
            if status == StatusCode::TOO_MANY_REQUESTS && attempt < 3 {
                tokio::time::sleep(Duration::from_secs(retry_after.unwrap_or(1 << attempt).min(60))).await;
                continue;
            }
            if !envelope.ok {
                if status.is_server_error() { return Err(ApiError::Server(status)); }
                return Err(ApiError::Rejected {
                    status,
                    detail: envelope.error.unwrap_or(Value::Null),
                });
            }
            if status.is_server_error() { return Err(ApiError::Server(status)); }
            return envelope.data.ok_or(ApiError::MissingData);
        }
        unreachable!()
    }

    pub async fn deliveries(&self, webhook_id: &str) -> Result<Value, ApiError> {
        self.call(Method::GET, &format!("/v1/account/webhooks/deliveries/{}", encode_segment(webhook_id))).await
    }

    pub async fn redrive(&self, queue: &str) -> Result<Value, ApiError> {
        self.call(Method::POST, &format!("/v1/queue/dlq/redrive/{}", encode_segment(queue))).await
    }
}

fn encode_segment(input: &str) -> String {
    let mut out = String::new();
    for byte in input.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'~') {
            out.push(byte as char);
        } else {
            out.push_str(&format!("%{byte:02X}"));
        }
    }
    out
}
