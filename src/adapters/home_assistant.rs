use reqwest::Client;
use serde_json::Value;

use crate::error::AppError;

#[derive(Clone)]
pub struct HomeAssistantClient {
    base_url: String,
    token_env: String,
    client: Client,
}

impl HomeAssistantClient {
    pub fn new(base_url: String, token_env: String, timeout_seconds: u64) -> anyhow::Result<Self> {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(timeout_seconds))
            .build()?;

        Ok(Self {
            base_url,
            token_env,
            client,
        })
    }

    fn token(&self) -> Result<String, AppError> {
        std::env::var(&self.token_env)
            .map_err(|_| AppError::Internal(format!("missing env var {}", self.token_env)))
    }

    fn url(&self, path: &str) -> String {
        format!(
            "{}/{}",
            self.base_url.trim_end_matches('/'),
            path.trim_start_matches('/')
        )
    }

    pub async fn get_root(&self) -> Result<Value, AppError> {
        self.get_json("/api/").await
    }

    pub async fn get_states(&self) -> Result<Value, AppError> {
        self.get_json("/api/states").await
    }

    pub async fn get_entity_state(&self, entity_id: &str) -> Result<Value, AppError> {
        let path = format!("/api/states/{}", entity_id);
        self.get_json(&path).await
    }

    async fn get_json(&self, path: &str) -> Result<Value, AppError> {
        let token = self.token()?;
        let url = self.url(path);

        let resp = self
            .client
            .get(url)
            .bearer_auth(token)
            .send()
            .await
            .map_err(|e| AppError::Internal(format!("home assistant request failed: {e}")))?;

        let status = resp.status();
        let body = resp
            .json::<Value>()
            .await
            .map_err(|e| AppError::Internal(format!("failed to parse HA response: {e}")))?;

        if !status.is_success() {
            return Err(AppError::Internal(format!(
                "home assistant returned status {}",
                status
            )));
        }

        Ok(body)
    }
}