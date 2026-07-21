//! THE vendor seam (R8). This file owns the ONLY Anthropic call in the
//! codebase — no other module names reqwest or the vendor. Key and model come
//! from AiConfig (read from env at startup); the key is used exactly once
//! below, as a request header, and never appears in a log line, an error, or
//! a response body. Every failure mode — transport, timeout, non-2xx,
//! unexpected shape — surfaces as the typed 503 `ai_unavailable`, never a
//! 500 panic, and never an error screen (the UI's contract).

use std::sync::OnceLock;
use std::time::Duration;

use serde::Deserialize;
use serde_json::json;

use crate::error::ApiError;
use crate::state::AiConfig;

const ANTHROPIC_MESSAGES_URL: &str = "https://api.anthropic.com/v1/messages";
const ANTHROPIC_VERSION: &str = "2023-06-01";
const MAX_TOKENS: u32 = 1024;
/// R8: 15s connect + 15s total on the vendor call.
const VENDOR_TIMEOUT: Duration = Duration::from_secs(15);

fn http() -> &'static reqwest::Client {
    static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
    CLIENT.get_or_init(|| {
        reqwest::Client::builder()
            .connect_timeout(VENDOR_TIMEOUT)
            .timeout(VENDOR_TIMEOUT)
            .build()
            .expect("reqwest client builds")
    })
}

#[derive(Deserialize)]
struct MessagesResponse {
    #[serde(default)]
    content: Vec<ContentBlock>,
}

#[derive(Deserialize)]
struct ContentBlock {
    #[serde(default)]
    text: String,
}

/// One system prompt + one user message → the model's text.
pub async fn complete(cfg: &AiConfig, system: &str, user_msg: &str) -> Result<String, ApiError> {
    let Some(key) = cfg.api_key.as_deref() else {
        return Err(ApiError::AiUnavailable("AI is not configured"));
    };

    let body = json!({
        "model": cfg.model,
        "max_tokens": MAX_TOKENS,
        "system": system,
        "messages": [{ "role": "user", "content": user_msg }],
    });

    let response = http()
        .post(ANTHROPIC_MESSAGES_URL)
        .header("x-api-key", key)
        .header("anthropic-version", ANTHROPIC_VERSION)
        .json(&body)
        .send()
        .await
        .map_err(|e| {
            // reqwest error displays carry the URL, never request headers —
            // the key cannot leak through this line.
            tracing::warn!(error = %e, "AI vendor transport failure");
            ApiError::AiUnavailable("the AI service is unreachable")
        })?;

    if !response.status().is_success() {
        tracing::warn!(
            status = response.status().as_u16(),
            model = %cfg.model,
            "AI vendor returned a non-success status"
        );
        return Err(ApiError::AiUnavailable("the AI service returned an error"));
    }

    let parsed: MessagesResponse = response.json().await.map_err(|e| {
        tracing::warn!(error = %e, "AI vendor response did not parse");
        ApiError::AiUnavailable("the AI service returned an unexpected response")
    })?;

    let text: String = parsed
        .content
        .iter()
        .map(|c| c.text.as_str())
        .collect::<Vec<_>>()
        .join("");
    if text.trim().is_empty() {
        return Err(ApiError::AiUnavailable(
            "the AI service returned an empty response",
        ));
    }
    Ok(text)
}
