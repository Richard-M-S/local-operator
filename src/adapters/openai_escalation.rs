use reqwest::Client;
use serde_json::{json, Value};

use crate::{config::OpenAiEscalationConfig, error::AppError};

#[derive(Clone)]
pub struct OpenAiEscalationClient {
    client: Client,
    base_url: String,
    api_key_env: String,
    model: String,
}

#[derive(Clone, Debug)]
pub struct OpenAiEscalationOutput {
    pub raw_response: Value,
    pub output_text: String,
    pub parsed_response: Value,
}

impl OpenAiEscalationClient {
    pub fn new(config: OpenAiEscalationConfig) -> anyhow::Result<Self> {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(config.timeout_seconds))
            .build()?;

        Ok(Self {
            client,
            base_url: config.base_url.trim_end_matches('/').to_string(),
            api_key_env: config.api_key_env,
            model: config.model,
        })
    }

    pub async fn send_redacted_request(
        &self,
        redacted_request: &Value,
    ) -> Result<OpenAiEscalationOutput, AppError> {
        let api_key = std::env::var(&self.api_key_env)
            .map_err(|_| AppError::Internal(format!("missing env var {}", self.api_key_env)))?;
        let url = format!("{}/responses", self.base_url);
        let redacted_json = serde_json::to_string_pretty(redacted_request)
            .map_err(|err| AppError::Internal(err.to_string()))?;

        let body = json!({
            "model": self.model,
            "input": [
                {
                    "role": "system",
                    "content": "You are ChatGPT assisting Local Operator. Return only structured JSON matching the schema. Recommendations are advisory only; do not claim to have executed actions."
                },
                {
                    "role": "user",
                    "content": format!(
                        "Analyze this redacted Local Operator escalation request and return JSON. Do not request secrets. Do not execute any recommended actions.\n\n{}",
                        redacted_json
                    )
                }
            ],
            "text": {
                "format": {
                    "type": "json_schema",
                    "name": "chatgpt_escalation_response",
                    "strict": true,
                    "schema": {
                        "type": "object",
                        "additionalProperties": false,
                        "required": [
                            "summary",
                            "findings",
                            "recommended_next_steps",
                            "actions_executed",
                            "requires_local_operator_action"
                        ],
                        "properties": {
                            "summary": { "type": "string" },
                            "findings": {
                                "type": "array",
                                "items": {
                                    "type": "object",
                                    "additionalProperties": false,
                                    "required": ["severity", "title", "detail"],
                                    "properties": {
                                        "severity": {
                                            "type": "string",
                                            "enum": ["info", "low", "medium", "high"]
                                        },
                                        "title": { "type": "string" },
                                        "detail": { "type": "string" }
                                    }
                                }
                            },
                            "recommended_next_steps": {
                                "type": "array",
                                "items": { "type": "string" }
                            },
                            "actions_executed": {
                                "type": "array",
                                "items": { "type": "string" }
                            },
                            "requires_local_operator_action": { "type": "boolean" }
                        }
                    }
                }
            }
        });

        let resp = self
            .client
            .post(url)
            .bearer_auth(api_key)
            .json(&body)
            .send()
            .await
            .map_err(|err| {
                AppError::Internal(format!("OpenAI escalation request failed: {err}"))
            })?;

        let status = resp.status();
        let raw_response: Value = resp.json().await.map_err(|err| {
            AppError::Internal(format!("OpenAI escalation response parse failed: {err}"))
        })?;

        if !status.is_success() {
            return Err(AppError::Internal(format!(
                "OpenAI escalation returned {status}: {raw_response}"
            )));
        }

        let output_text = extract_output_text(&raw_response);
        let parsed_response = serde_json::from_str(&output_text).unwrap_or_else(|_| {
            raw_response
                .get("output")
                .cloned()
                .unwrap_or_else(|| raw_response.clone())
        });

        Ok(OpenAiEscalationOutput {
            raw_response,
            output_text,
            parsed_response,
        })
    }
}

fn extract_output_text(raw_response: &Value) -> String {
    if let Some(output_text) = raw_response
        .get("output_text")
        .and_then(|value| value.as_str())
    {
        return output_text.to_string();
    }

    raw_response
        .get("output")
        .and_then(|value| value.as_array())
        .into_iter()
        .flatten()
        .flat_map(|item| {
            item.get("content")
                .and_then(|value| value.as_array())
                .into_iter()
                .flatten()
        })
        .filter_map(|content| content.get("text").and_then(|value| value.as_str()))
        .collect::<Vec<_>>()
        .join("\n")
}
