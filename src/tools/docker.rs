use async_trait::async_trait;
use serde::Deserialize;
use serde_json::{json, Value};

use crate::{error::AppError, tools::registry::Tool};

pub struct ListContainersTool;

#[async_trait]
impl Tool for ListContainersTool {
    async fn execute(&self, _args: Value) -> Result<Value, AppError> {
        Ok(json!({
            "containers": [
                { "name": "homeassistant", "state": "running", "health": "healthy" },
                { "name": "zwave-js-ui", "state": "running", "health": "healthy" }
            ]
        }))
    }
}

pub struct RestartContainerTool {
    pub allowed: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct RestartArgs {
    name: String,
}

#[async_trait]
impl Tool for RestartContainerTool {
    async fn execute(&self, args: Value) -> Result<Value, AppError> {
        let parsed: RestartArgs = serde_json::from_value(args)
            .map_err(|e| AppError::BadRequest(format!("invalid args: {}", e)))?;

        if !self.allowed.iter().any(|x| x == &parsed.name) {
            return Err(AppError::PolicyDenied(format!(
                "container '{}' is not in the allowed restart list",
                parsed.name
            )));
        }

        Ok(json!({
            "status": "success",
            "message": format!("Container '{}' restarted.", parsed.name)
        }))
    }
}