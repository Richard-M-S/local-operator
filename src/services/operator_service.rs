use serde_json::json;
use uuid::Uuid;

use crate::{
    domains::employment::{
        models::default_employment_profile_id, repository::EmploymentRepository,
    },
    error::AppError,
    models::api::{ChatResponse, CommandResponse},
    op_tasks::service::OpTaskService,
    services::llm_router::LlmRouter,
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
    llm_router: LlmRouter,
    op_tasks: OpTaskService,
    employment_repo: EmploymentRepository,
}

impl OperatorService {
    pub fn new(
        tools: ToolRegistry,
        policy: PolicyEngine,
        audit: AuditService,
        llm: Option<LlmService>,
        llm_router: LlmRouter,
        op_tasks: OpTaskService,
        employment_repo: EmploymentRepository,
    ) -> Self {
        Self {
            tools,
            policy,
            audit,
            llm,
            llm_router,
            op_tasks,
            employment_repo,
        }
    }

    pub async fn run_chat(
        &self,
        message: &str,
        include_home: bool,
        profile_id: Option<Uuid>,
    ) -> Result<ChatResponse, AppError> {
        if let Some(url) = extract_read_url(message) {
            return self
                .run_reader_url_chat_task(
                    profile_id.unwrap_or_else(default_employment_profile_id),
                    &url,
                )
                .await;
        }

        if let Some(query) = self
            .extract_search_query(
                message,
                profile_id.unwrap_or_else(default_employment_profile_id),
            )
            .await?
        {
            return self
                .run_search_web_chat_task(
                    profile_id.unwrap_or_else(default_employment_profile_id),
                    &query,
                )
                .await;
        }

        let llm = self
            .llm
            .as_ref()
            .ok_or_else(|| AppError::Internal("LLM service is not enabled".to_string()))?;

        let decision = self.llm_router.route(message);
        let use_home_context = include_home && decision.needs_home_context;

        if use_home_context {
            let tool_name = "ha.get_overview";

            let descriptor = self.tools.describe(tool_name).await?;
            self.policy
                .check_tool_execution(descriptor.risk_tier, false)?;

            let result = self.tools.execute(tool_name, json!({})).await?;
            let _ = self.audit.record_tool_call(tool_name, true).await;

            let response = llm
                .summarize_home_overview_with_model(&decision.model, message, &result.output)
                .await?;

            return Ok(ChatResponse {
                ok: true,
                mode: format!("chat_home_context::{:?}", decision.route),
                message: response,
                data: json!({
                    "route_decision": decision,
                    "home": result
                }),
            });
        }

        let system = r#"
    You are Local Operator, a local assistant running on the user's server.
    Be concise, direct, and practical.
    Do not claim access to Home Assistant unless home context was included.
    "#;

        let response = llm.ask_model(&decision.model, system, message).await?;

        Ok(ChatResponse {
            ok: true,
            mode: format!("chat::{:?}", decision.route),
            message: response,
            data: json!({
                "route_decision": decision
            }),
        })
    }

    async fn run_reader_url_chat_task(
        &self,
        profile_id: Uuid,
        url: &str,
    ) -> Result<ChatResponse, AppError> {
        let task = self
            .op_tasks
            .create_task(
                profile_id,
                "reader.read_url".to_string(),
                "Chat Read URL".to_string(),
                Some("Created from Local Operator chat.".to_string()),
                json!({
                    "url": url,
                    "priority": "normal",
                    "model_purpose": "task_extraction",
                    "source": "operator_chat"
                }),
                true,
            )
            .await?;

        let run = self.op_tasks.run_task(task.id).await?;
        let artifact = run.artifacts.first();
        let ok = matches!(
            run.status,
            crate::op_tasks::models::OpTaskRunStatus::Succeeded
        );
        let message = if let Some(artifact) = artifact {
            format!("Read URL and saved artifact '{}'.", artifact.name)
        } else {
            run.summary
                .clone()
                .unwrap_or_else(|| "Reader task completed without an artifact.".to_string())
        };

        Ok(ChatResponse {
            ok,
            mode: "chat_task::reader.read_url".to_string(),
            message,
            data: json!({
                "task": task,
                "run": run,
                "artifact": artifact,
            }),
        })
    }

    async fn run_search_web_chat_task(
        &self,
        profile_id: Uuid,
        query: &str,
    ) -> Result<ChatResponse, AppError> {
        let task = self
            .op_tasks
            .create_task(
                profile_id,
                "reader.search_web".to_string(),
                "Chat Web Search".to_string(),
                Some("Created from Local Operator chat.".to_string()),
                json!({
                    "query": query,
                    "limit": 10,
                    "priority": "normal",
                    "model_purpose": "task_extraction",
                    "source": "operator_chat"
                }),
                true,
            )
            .await?;

        let run = self.op_tasks.run_task(task.id).await?;
        let artifact = run.artifacts.first();
        let ok = matches!(
            run.status,
            crate::op_tasks::models::OpTaskRunStatus::Succeeded
        );
        let message = run
            .summary
            .clone()
            .unwrap_or_else(|| "Search task completed.".to_string());

        Ok(ChatResponse {
            ok,
            mode: "chat_task::reader.search_web".to_string(),
            message,
            data: json!({
                "task": task,
                "run": run,
                "artifact": artifact,
            }),
        })
    }

    async fn extract_search_query(
        &self,
        message: &str,
        profile_id: Uuid,
    ) -> Result<Option<String>, AppError> {
        let normalized = message.to_lowercase();
        let wants_search = normalized.contains("search")
            || normalized.contains("find jobs")
            || normalized.contains("look for jobs");

        if !wants_search {
            return Ok(None);
        }

        let mut query = message
            .trim()
            .trim_end_matches(|ch: char| matches!(ch, '.' | '!' | '?'))
            .to_string();

        for prefix in ["search for", "search", "find", "look for"] {
            if query.to_lowercase().starts_with(prefix) {
                query = query[prefix.len()..].trim().to_string();
                break;
            }
        }

        if normalized.contains("my criteria") || normalized.contains("profile criteria") {
            if let Some(profile) = self
                .employment_repo
                .get_profile(profile_id)
                .await
                .map_err(|err| AppError::Internal(err.to_string()))?
            {
                if let Some(criteria) = profile.criteria.filter(|value| !value.trim().is_empty()) {
                    query = format!("jobs {}", criteria.trim());
                }
            }
        }

        if query.to_lowercase().contains("create artifacts") {
            query = query
                .replace("and create artifacts if you can", "")
                .replace("create artifacts if you can", "")
                .replace("and create artifacts", "")
                .replace("create artifacts", "")
                .trim()
                .to_string();
        }

        if query.is_empty() {
            return Ok(None);
        }

        if !query.to_lowercase().contains("job") {
            query = format!("{} jobs", query);
        }

        Ok(Some(query))
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

        let decision = self.llm_router.route(input);

        let message = llm
            .summarize_home_overview_with_model(&decision.model, input, &result.output)
            .await?;

        Ok(CommandResponse {
            ok: true,
            mode: "llm_home_summary".to_string(),
            message,
            data: serde_json::to_value(result).map_err(|e| AppError::Internal(e.to_string()))?,
        })
    }
}

fn extract_read_url(message: &str) -> Option<String> {
    let normalized = message.to_lowercase();
    let wants_read = normalized.contains("read")
        || normalized.contains("fetch")
        || normalized.contains("open")
        || normalized.contains("url");

    if !wants_read {
        return None;
    }

    message
        .split_whitespace()
        .find(|part| part.starts_with("http://") || part.starts_with("https://"))
        .map(|part| {
            part.trim_matches(|ch: char| matches!(ch, ',' | '.' | ')' | ']' | '}' | '"' | '\''))
                .to_string()
        })
        .filter(|url| !url.is_empty())
}
