use async_trait::async_trait;
use serde_json::{json, Value};

use crate::{
    error::AppError,
    models::tool::{RiskTier, ToolDescriptor},
};

use super::registry::Tool;

pub struct SystemStatusTool;

impl SystemStatusTool {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Tool for SystemStatusTool {
    fn descriptor(&self) -> ToolDescriptor {
        ToolDescriptor {
            name: "system.get_status".to_string(),
            description: "Return basic local operator health and runtime status".to_string(),
            risk_tier: RiskTier::Tier0,
            requires_confirmation: false,
        }
    }

    async fn execute(&self, _args: Value) -> Result<Value, AppError> {
        Ok(json!({
            "service": "local-operator",
            "status": "ok"
        }))
    }
}