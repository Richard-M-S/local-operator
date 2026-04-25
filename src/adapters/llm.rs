use reqwest::Client;
use serde_json::{json, Value};

use crate::{config::LlmConfig, error::AppError};

#[derive(Clone)]
pub struct LlmClient {
    client: Client,
    base_url: String,
    model: String,
}

impl LlmClient {
    pub fn new(config: LlmConfig) -> anyhow::Result<Self> {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(config.timeout_seconds))
            .build()?;

        Ok(Self {
            client,
            base_url: config.base_url.trim_end_matches('/').to_string(),
            model: config.model,
        })
    }

    pub async fn chat(&self, system: &str, user: &str) -> Result<String, AppError> {
        let url = format!("{}/api/chat", self.base_url);

        let body = json!({
            "model": self.model,
            "stream": false,
            "messages": [
                { "role": "system", "content": system },
                { "role": "user", "content": user }
            ]
        });

        let resp = self.client
            .post(url)
            .json(&body)
            .send()
            .await
            .map_err(|e| AppError::Internal(format!("LLM request failed: {e}")))?;

        let status = resp.status();
        let value: Value = resp.json().await
            .map_err(|e| AppError::Internal(format!("LLM response parse failed: {e}")))?;

        if !status.is_success() {
            return Err(AppError::Internal(format!("LLM returned {status}: {value}")));
        }

        let content = value
            .get("message")
            .and_then(|m| m.get("content"))
            .and_then(|c| c.as_str())
            .unwrap_or("")
            .to_string();

        Ok(content)
    }
}