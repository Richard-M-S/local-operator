use async_trait::async_trait;
use serde_json::{json, Value};

use crate::{error::AppError, tools::registry::Tool};

pub struct GetSystemStatusTool;

#[async_trait]
impl Tool for GetSystemStatusTool {
    async fn execute(&self, _args: Value) -> Result<Value, AppError> {
        Ok(json!({
            "hostname": "operator-host",
            "status": "ok",
            "cpu": "stub",
            "memory": "stub",
            "disk": "stub"
        }))
    }
}