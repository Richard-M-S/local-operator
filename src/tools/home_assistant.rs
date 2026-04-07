use async_trait::async_trait;
use serde_json::{json, Value};

use crate::{error::AppError, tools::registry::Tool};

pub struct GetHomeSummaryTool {
    pub enabled: bool,
}

#[async_trait]
impl Tool for GetHomeSummaryTool {
    async fn execute(&self, _args: Value) -> Result<Value, AppError> {
        if !self.enabled {
            return Err(AppError::BadRequest("Home Assistant integration is disabled".into()));
        }

        Ok(json!({
            "front_door_lock": "locked",
            "garage_door": "closed",
            "alarm": "armed_home",
            "porch_light": "off"
        }))
    }
}