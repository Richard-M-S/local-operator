use serde_json::Value;
use uuid::Uuid;

use crate::{
    error::AppError,
    models::tool::{RiskTier, ToolExecutionResult},
    tools::registry::ToolRegistry,
};

use super::{
    audit_service::{AuditService, ExecutionAuditRecord},
    llm_service::LlmService,
    policy_engine::PolicyEngine,
};

#[derive(Clone, Debug, Default)]
pub struct ExecutionContext {
    pub task_id: Option<Uuid>,
    pub run_id: Option<Uuid>,
    pub work_item_id: Option<Uuid>,
    pub model_purpose: Option<String>,
    pub input_summary: Option<String>,
    pub output_artifact_ids: Vec<Uuid>,
}

impl ExecutionContext {
    pub fn for_work_item(task_id: Uuid, run_id: Uuid, work_item_id: Uuid) -> Self {
        Self {
            task_id: Some(task_id),
            run_id: Some(run_id),
            work_item_id: Some(work_item_id),
            ..Self::default()
        }
    }

    pub fn with_model_purpose(mut self, purpose: impl Into<String>) -> Self {
        self.model_purpose = Some(purpose.into());
        self
    }

    pub fn with_input_summary(mut self, summary: impl Into<String>) -> Self {
        self.input_summary = Some(summary.into());
        self
    }

    pub fn with_output_artifact(mut self, artifact_id: Uuid) -> Self {
        self.output_artifact_ids.push(artifact_id);
        self
    }
}

#[derive(Clone)]
pub struct ToolExecutionService {
    tools: ToolRegistry,
    policy: PolicyEngine,
    audit: AuditService,
}

impl ToolExecutionService {
    pub fn new(tools: ToolRegistry, policy: PolicyEngine, audit: AuditService) -> Self {
        Self {
            tools,
            policy,
            audit,
        }
    }

    pub async fn execute(
        &self,
        tool_name: &str,
        args: Value,
        confirm: bool,
        context: ExecutionContext,
    ) -> Result<ToolExecutionResult, AppError> {
        let descriptor = match self.tools.describe(tool_name).await {
            Ok(descriptor) => descriptor,
            Err(err) => {
                self.audit_tool_attempt(
                    tool_name,
                    args,
                    context,
                    "descriptor_error",
                    None,
                    false,
                    false,
                    Some(err.to_string()),
                )
                .await;
                return Err(err);
            }
        };
        let risk_tier = risk_tier_value(descriptor.risk_tier);

        if let Err(err) = self
            .policy
            .check_tool_execution(descriptor.risk_tier, confirm)
        {
            let message = err.to_string();
            self.audit_tool_attempt(
                tool_name,
                args,
                context,
                "denied",
                Some(risk_tier),
                false,
                false,
                Some(message.clone()),
            )
            .await;
            return Err(err);
        }

        match self.tools.execute(tool_name, args.clone()).await {
            Ok(result) => {
                self.audit_tool_attempt(
                    tool_name,
                    args,
                    context,
                    "allowed",
                    Some(risk_tier),
                    true,
                    true,
                    None,
                )
                .await;
                Ok(result)
            }
            Err(err) => {
                let message = err.to_string();
                self.audit_tool_attempt(
                    tool_name,
                    args,
                    context,
                    "allowed",
                    Some(risk_tier),
                    true,
                    false,
                    Some(message.clone()),
                )
                .await;
                Err(err)
            }
        }
    }

    async fn audit_tool_attempt(
        &self,
        tool_name: &str,
        args: Value,
        context: ExecutionContext,
        policy_decision: &str,
        risk_tier: Option<i32>,
        allowed: bool,
        success: bool,
        error: Option<String>,
    ) {
        let _ = self
            .audit
            .record_execution_attempt(ExecutionAuditRecord {
                execution_type: "tool".to_string(),
                name: tool_name.to_string(),
                task_id: context.task_id,
                run_id: context.run_id,
                work_item_id: context.work_item_id,
                model_purpose: None,
                input_summary: context.input_summary,
                args_json: Some(args),
                policy_decision: policy_decision.to_string(),
                risk_tier,
                allowed,
                success,
                error,
                output_artifact_ids: context.output_artifact_ids,
            })
            .await;
    }
}

#[derive(Clone)]
pub struct ModelExecutionService {
    llm: Option<LlmService>,
    audit: AuditService,
}

impl ModelExecutionService {
    pub fn new(llm: Option<LlmService>, audit: AuditService) -> Self {
        Self { llm, audit }
    }

    pub async fn ask_model(
        &self,
        model: &str,
        system: &str,
        prompt: &str,
        context: ExecutionContext,
    ) -> Result<String, AppError> {
        let Some(llm) = &self.llm else {
            let err = AppError::Internal("LLM service is not enabled".to_string());
            self.audit_model_attempt(model, context, false, Some(err.to_string()))
                .await;
            return Err(err);
        };

        match llm.ask_model(model, system, prompt).await {
            Ok(response) => {
                self.audit_model_attempt(model, context, true, None).await;
                Ok(response)
            }
            Err(err) => {
                let message = err.to_string();
                self.audit_model_attempt(model, context, false, Some(message.clone()))
                    .await;
                Err(err)
            }
        }
    }

    pub async fn summarize_home_overview_with_model(
        &self,
        model: &str,
        user_command: &str,
        overview_json: &serde_json::Value,
        context: ExecutionContext,
    ) -> Result<String, AppError> {
        let Some(llm) = &self.llm else {
            let err = AppError::Internal("LLM service is not enabled".to_string());
            self.audit_model_attempt(model, context, false, Some(err.to_string()))
                .await;
            return Err(err);
        };

        match llm
            .summarize_home_overview_with_model(model, user_command, overview_json)
            .await
        {
            Ok(response) => {
                self.audit_model_attempt(model, context, true, None).await;
                Ok(response)
            }
            Err(err) => {
                let message = err.to_string();
                self.audit_model_attempt(model, context, false, Some(message.clone()))
                    .await;
                Err(err)
            }
        }
    }

    pub async fn parse_job_opportunity(
        &self,
        model: &str,
        job_text: &str,
        context: ExecutionContext,
    ) -> Result<serde_json::Value, AppError> {
        let Some(llm) = &self.llm else {
            let err = AppError::Internal("LLM service not available".to_string());
            self.audit_model_attempt(model, context, false, Some(err.to_string()))
                .await;
            return Err(err);
        };

        match llm.parse_job_opportunity(model, job_text).await {
            Ok(value) => {
                self.audit_model_attempt(model, context, true, None).await;
                Ok(value)
            }
            Err(err) => {
                let message = err.to_string();
                self.audit_model_attempt(model, context, false, Some(message.clone()))
                    .await;
                Err(err)
            }
        }
    }

    pub async fn score_job_opportunity(
        &self,
        model: &str,
        job: &serde_json::Value,
        criteria: &str,
        context: ExecutionContext,
    ) -> Result<serde_json::Value, AppError> {
        let Some(llm) = &self.llm else {
            let err = AppError::Internal("LLM service not available".to_string());
            self.audit_model_attempt(model, context, false, Some(err.to_string()))
                .await;
            return Err(err);
        };

        match llm.score_job_opportunity(model, job, criteria).await {
            Ok(value) => {
                self.audit_model_attempt(model, context, true, None).await;
                Ok(value)
            }
            Err(err) => {
                let message = err.to_string();
                self.audit_model_attempt(model, context, false, Some(message.clone()))
                    .await;
                Err(err)
            }
        }
    }

    pub async fn generate_cover_letter(
        &self,
        model: &str,
        opportunity_json: &serde_json::Value,
        profile_criteria: &str,
        profile_context: &str,
        direction: &str,
        context: ExecutionContext,
    ) -> Result<String, AppError> {
        let Some(llm) = &self.llm else {
            let err = AppError::Internal("LLM service not available".to_string());
            self.audit_model_attempt(model, context, false, Some(err.to_string()))
                .await;
            return Err(err);
        };

        match llm
            .generate_cover_letter(
                model,
                opportunity_json,
                profile_criteria,
                profile_context,
                direction,
            )
            .await
        {
            Ok(value) => {
                self.audit_model_attempt(model, context, true, None).await;
                Ok(value)
            }
            Err(err) => {
                let message = err.to_string();
                self.audit_model_attempt(model, context, false, Some(message.clone()))
                    .await;
                Err(err)
            }
        }
    }

    async fn audit_model_attempt(
        &self,
        model: &str,
        context: ExecutionContext,
        success: bool,
        error: Option<String>,
    ) {
        let _ = self
            .audit
            .record_execution_attempt(ExecutionAuditRecord {
                execution_type: "model".to_string(),
                name: model.to_string(),
                task_id: context.task_id,
                run_id: context.run_id,
                work_item_id: context.work_item_id,
                model_purpose: context.model_purpose,
                input_summary: context.input_summary,
                args_json: None,
                policy_decision: "allowed_low_risk".to_string(),
                risk_tier: Some(0),
                allowed: true,
                success,
                error,
                output_artifact_ids: context.output_artifact_ids,
            })
            .await;
    }
}

fn risk_tier_value(risk_tier: RiskTier) -> i32 {
    match risk_tier {
        RiskTier::Tier0 => 0,
        RiskTier::Tier1 => 1,
        RiskTier::Tier2 => 2,
        RiskTier::Tier3 => 3,
    }
}
