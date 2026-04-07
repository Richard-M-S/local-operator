use chrono::Utc;
use serde_json::json;

use crate::error::AppError;
use crate::models::api::CommandResponse;
use crate::models::audit::AuditEntry;
use crate::services::{audit_service::AuditService, planner::Planner, policy_engine::PolicyEngine};
use crate::tools::registry::ToolRegistry;

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

    pub async fn run_command(&self, input: &str, confirm: bool) -> Result<CommandResponse, AppError> {
        let plan = Planner::build(input);
        let decision = self.policy.evaluate(plan.risk_tier, confirm);

        if !decision.allowed {
            let entry = AuditEntry {
                request_id: plan.request_id,
                created_at: Utc::now(),
                raw_input: plan.raw_input.clone(),
                parsed_intent: Some(plan.parsed_intent.clone()),
                risk_tier: plan.risk_tier.as_i32(),
                allowed: false,
                actions_json: Some(serde_json::to_string(&plan.actions).unwrap_or_default()),
                results_json: None,
                final_message: decision.reason.clone(),
            };

            let _ = self.audit.save(entry).await;

            return Err(AppError::PolicyDenied(
                decision.reason.unwrap_or_else(|| "Action denied".into()),
            ));
        }

        let mut results = vec![];
        for action in &plan.actions {
            let result = self.tools.execute(&action.tool, action.args.clone()).await?;
            results.push(json!({
                "tool": action.tool,
                "result": result
            }));
        }

        let message = if results.is_empty() {
            "No action matched the request.".to_string()
        } else {
            "Request completed successfully.".to_string()
        };

        let entry = AuditEntry {
            request_id: plan.request_id,
            created_at: Utc::now(),
            raw_input: plan.raw_input.clone(),
            parsed_intent: Some(plan.parsed_intent.clone()),
            risk_tier: plan.risk_tier.as_i32(),
            allowed: true,
            actions_json: Some(serde_json::to_string(&plan.actions).unwrap_or_default()),
            results_json: Some(serde_json::to_string(&results).unwrap_or_default()),
            final_message: Some(message.clone()),
        };

        let _ = self.audit.save(entry).await;

        Ok(CommandResponse {
            request_id: plan.request_id,
            parsed_intent: plan.parsed_intent,
            allowed: true,
            risk_tier: plan.risk_tier.as_i32(),
            message,
            actions: serde_json::to_value(&plan.actions).unwrap_or(json!([])),
            results: json!(results),
        })
    }
}