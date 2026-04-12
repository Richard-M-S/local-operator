use async_trait::async_trait;
use serde_json::{json, Value};

use crate::{
    error::AppError,
    models::tool::{RiskTier, ToolDescriptor},
};

use super::registry::Tool;

pub struct HomeAssistantSummaryTool {
    base_url: String,
    token_env: String,
}

impl HomeAssistantSummaryTool {
    pub fn new(base_url: String, token_env: String) -> Self {
        Self { base_url, token_env }
    }
}

#[async_trait]
impl Tool for HomeAssistantSummaryTool {
    fn descriptor(&self) -> ToolDescriptor {
        ToolDescriptor {
            name: "ha.get_summary".to_string(),
            description: "Return a light Home Assistant API status summary".to_string(),
            risk_tier: RiskTier::Tier0,
            requires_confirmation: false,
        }
    }

    async fn execute(&self, _args: Value) -> Result<Value, AppError> {
        let token = std::env::var(&self.token_env)
            .map_err(|_| AppError::Internal(format!("missing env var {}", self.token_env)))?;

        let url = format!("{}/api/", self.base_url.trim_end_matches('/'));

        let client = reqwest::Client::new();
        let resp = client
            .get(url)
            .bearer_auth(token)
            .send()
            .await
            .map_err(|e| AppError::Internal(format!("home assistant request failed: {e}")))?;

        let status = resp.status();

        let body: Value = resp
            .json()
            .await
            .unwrap_or_else(|_| json!({ "raw": "unable to parse response body" }));

        if !status.is_success() {
            return Err(AppError::Internal(format!(
                "home assistant returned status {}",
                status
            )));
        }

        Ok(json!({
            "reachable": true,
            "status_code": status.as_u16(),
            "body": body
        }))
    }
}