use serde_json::json;

use crate::{
    error::AppError,
    models::api::CommandResponse,
    services::llm_service::LlmService,
    tools::registry::ToolRegistry,
};

use super::{audit_service::AuditService, policy_engine::PolicyEngine};

#[derive(Clone)]
pub struct OperatorService {
    tools: ToolRegistry,
    policy: PolicyEngine,
    audit: AuditService,
    llm: Option<LlmService>,
}

impl OperatorService {
    pub fn new(
        tools: ToolRegistry,
        policy: PolicyEngine,
        audit: AuditService,
        llm: Option<LlmService>,
    ) -> Self {
        Self {
            tools,
            policy,
            audit,
            llm,
        }
    }

    pub async fn run_command(
        &self,
        input: &str,
        confirm: bool,
    ) -> Result<CommandResponse, AppError> {
        let normalized = input.trim().to_lowercase();

        if normalized.contains("home")
            || normalized.contains("house")
            || normalized.contains("front door")
            || normalized.contains("lock")
            || normalized.contains("weather")
            || normalized.contains("vacuum")
            || normalized.contains("bottom maid")
        {
            return self.run_home_llm_command(input, confirm).await;
        }

        let tool_name = match normalized.as_str() {
            "status" | "get status" => "system.get_status",
            "docker" | "docker status" | "list containers" => "docker.list_containers",
            "home" | "ha" | "home assistant" => "ha.get_overview",
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
            data: serde_json::to_value(result).map_err(|e| AppError::Internal(e.to_string()))?,
        })
    }

    async fn run_home_llm_command(
        &self,
        input: &str,
        confirm: bool,
    ) -> Result<CommandResponse, AppError> {
        let tool_name = "ha.get_overview";

        let descriptor = self.tools.describe(tool_name).await?;
        self.policy
            .check_tool_execution(descriptor.risk_tier, confirm)?;

        let result = self.tools.execute(tool_name, json!({})).await?;
        let _ = self.audit.record_tool_call(tool_name, true).await;

        let llm = self
            .llm
            .as_ref()
            .ok_or_else(|| AppError::Internal("LLM service is not enabled".to_string()))?;

        let message = llm
            .summarize_home_overview(input, &result.output)
            .await?;

        Ok(CommandResponse {
            ok: true,
            mode: "llm_home_summary".to_string(),
            message,
            data: serde_json::to_value(result).map_err(|e| AppError::Internal(e.to_string()))?,
        })
    }
}