use serde_json::json;

use crate::{
    error::AppError,
    models::api::CommandResponse,
    tools::registry::ToolRegistry,
};

use super::{audit_service::AuditService, policy_engine::PolicyEngine};

#[derive(Clone)]
pub struct OperatorService {
    tools: ToolRegistry,
    policy: PolicyEngine,
    audit: AuditService,
}

impl OperatorService {
    pub fn new(tools: ToolRegistry, policy: PolicyEngine, audit: AuditService) -> Self {
        Self { tools, policy, audit }
    }

    pub async fn run_command(
        &self,
        input: &str,
        confirm: bool,
    ) -> Result<CommandResponse, AppError> {
        let normalized = input.trim().to_lowercase();

        let tool_name = match normalized.as_str() {
            "status" | "get status" => "system.get_status",
            "docker" | "docker status" | "list containers" => "docker.list_containers",
            "home" | "ha" | "home assistant" => "ha.get_summary",
            _ => {
                return Ok(CommandResponse {
                    ok: false,
                    mode: "unresolved".to_string(),
                    message: format!("no command mapping for '{}'", input),
                    data: json!({}),
                });
            }
        };

        let descriptor = self.tools.describe(tool_name).await?;
        self.policy
            .check_tool_execution(descriptor.risk_tier, confirm)?;

        let result = self.tools.execute(tool_name, json!({})).await?;
        let _ = self.audit.record_tool_call(tool_name, true).await;

        Ok(CommandResponse {
            ok: true,
            mode: "tool".to_string(),
            message: format!("executed {}", tool_name),
            data: serde_json::to_value(result)
                .map_err(|e| AppError::Internal(e.to_string()))?,
        })
    }
}