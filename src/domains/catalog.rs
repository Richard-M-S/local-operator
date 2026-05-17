use serde::Serialize;
use serde_json::{json, Value};

use crate::models::tool::RiskTier;

#[derive(Clone, Debug, Serialize)]
pub struct DomainCatalogResponse {
    pub domains: Vec<DomainDescriptor>,
}

#[derive(Clone, Debug, Serialize)]
pub struct DomainDescriptor {
    pub domain: String,
    pub display_name: String,
    pub priority: u8,
    pub description: String,
    pub task_types: Vec<DomainTaskType>,
    pub planner: DomainPlanner,
    pub work_item_types: Vec<String>,
    pub tools: Vec<DomainTool>,
    pub artifact_types: Vec<DomainArtifactType>,
    pub model_purposes: Vec<DomainModelPurpose>,
    pub policy_tiers: Vec<DomainPolicyTier>,
    pub safety_levels: Vec<DomainSafetyLevel>,
    pub continuation_rules: Vec<DomainContinuationRule>,
}

#[derive(Clone, Debug, Serialize)]
pub struct DomainTaskType {
    pub name: String,
    pub status: DomainTaskStatus,
    pub description: String,
    pub input_schema: Value,
}

#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DomainTaskStatus {
    Active,
    Planned,
    Alias,
    #[allow(dead_code)]
    ContinuationRule,
}

#[derive(Clone, Debug, Serialize)]
pub struct DomainPlanner {
    pub strategy: String,
    pub planner_module: String,
    pub notes: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct DomainTool {
    pub name: String,
    pub purpose: String,
    pub required_now: bool,
}

#[derive(Clone, Debug, Serialize)]
pub struct DomainArtifactType {
    pub name: String,
    pub description: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct DomainModelPurpose {
    pub purpose: String,
    pub model_route: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct DomainPolicyTier {
    pub operation: String,
    pub risk_tier: RiskTier,
    pub requires_confirmation: bool,
}

#[derive(Clone, Debug, Serialize)]
pub struct DomainSafetyLevel {
    pub level: u8,
    pub name: String,
    pub description: String,
    pub allowed: Vec<String>,
    pub requires_confirmation: bool,
    pub status: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct DomainContinuationRule {
    pub source_artifact_type: String,
    pub continuations: Vec<String>,
}

pub fn catalog_response() -> DomainCatalogResponse {
    DomainCatalogResponse {
        domains: domain_catalog(),
    }
}

pub fn domain_catalog() -> Vec<DomainDescriptor> {
    vec![
        home_operations_domain(),
        research_domain(),
        code_repo_domain(),
        infrastructure_domain(),
        knowledge_domain(),
        calendar_domain(),
        operator_domain(),
    ]
}

pub fn find_domain(domain: &str) -> Option<DomainDescriptor> {
    let normalized = domain.trim().to_lowercase();
    domain_catalog()
        .into_iter()
        .find(|candidate| candidate.domain == normalized)
}

fn home_operations_domain() -> DomainDescriptor {
    DomainDescriptor {
        domain: "home".to_string(),
        display_name: "Home Operations".to_string(),
        priority: 1,
        description: "Home Assistant-backed household status, energy, HVAC, and safety review workflows."
            .to_string(),
        task_types: vec![
            task(
                "home.daily_status_report",
                DomainTaskStatus::Planned,
                "Create a daily operational report from Home Assistant overview data.",
                json!({
                    "type": "object",
                    "properties": {
                        "include_energy": { "type": "boolean", "default": true },
                        "include_security": { "type": "boolean", "default": true },
                        "areas": { "type": "array", "items": { "type": "string" } }
                    }
                }),
            ),
            task(
                "home.energy_hvac_review",
                DomainTaskStatus::Planned,
                "Review energy and HVAC snapshot data and recommend changes.",
                json!({
                    "type": "object",
                    "properties": {
                        "lookback_hours": { "type": "integer", "minimum": 1, "default": 24 },
                        "comfort_goal": { "type": "string" },
                        "cost_goal": { "type": "string" }
                    }
                }),
            ),
            task(
                "home.security_check",
                DomainTaskStatus::Planned,
                "Check security-sensitive home entities and produce a read-only report.",
                json!({
                    "type": "object",
                    "properties": {
                        "areas": { "type": "array", "items": { "type": "string" } },
                        "include_entity_details": { "type": "boolean", "default": false }
                    }
                }),
            ),
        ],
        planner: planner(
            "hardcoded_domain_planner",
            "TaskPlanner::plan_home_*",
            "Start with read-only HA snapshots, then summarize with task_summary_model or task_reasoning_model.",
        ),
        work_item_types: strings(&["ha_snapshot", "model_summary", "policy_review"]),
        tools: tools(&[
            ("ha.get_overview", "Collect home overview state.", true),
            (
                "ha.get_energy_hvac_snapshot",
                "Collect energy and HVAC state.",
                true,
            ),
            ("ha.get_summary", "Collect a short HA summary.", true),
            ("ha.get_states", "Inspect Home Assistant states.", true),
            ("ha.search_entities", "Find relevant HA entities.", true),
        ]),
        artifact_types: artifacts(&[
            ("ha_overview_snapshot", "Saved Home Assistant overview data."),
            (
                "ha_energy_hvac_snapshot",
                "Saved energy and HVAC snapshot data.",
            ),
            ("home_status_report", "Human-readable home operations report."),
        ]),
        model_purposes: models(&[
            ("home_summary", "task_summary_model"),
            ("home_reasoning", "task_reasoning_model"),
        ]),
        policy_tiers: policy(&[
            ("read", RiskTier::Tier0, false),
            ("write", RiskTier::Tier2, true),
            ("security_sensitive", RiskTier::Tier3, true),
        ]),
        safety_levels: vec![],
        continuation_rules: continuations(&[
            (
                "ha_energy_hvac_snapshot",
                &[
                    "summarize_hvac_state",
                    "recommend_automation_changes",
                    "create_home_energy_review_task",
                ],
            ),
            (
                "ha_overview_snapshot",
                &["summarize_home_state", "create_security_check_task"],
            ),
        ]),
    }
}

fn research_domain() -> DomainDescriptor {
    DomainDescriptor {
        domain: "research".to_string(),
        display_name: "Research / Web Briefings".to_string(),
        priority: 2,
        description: "Web search, URL reading, source capture, and brief generation.".to_string(),
        task_types: vec![
            task(
                "reader.search_web",
                DomainTaskStatus::Active,
                "Search the web through the configured SearchProvider.",
                json!({
                    "type": "object",
                    "required": ["query"],
                    "properties": {
                        "query": { "type": "string" },
                        "limit": { "type": "integer", "minimum": 1, "maximum": 25, "default": 10 }
                    }
                }),
            ),
            task(
                "reader.read_url",
                DomainTaskStatus::Active,
                "Read a URL and save extracted page text.",
                json!({
                    "type": "object",
                    "required": ["url"],
                    "properties": {
                        "url": { "type": "string", "format": "uri" }
                    }
                }),
            ),
            task(
                "research.web_brief",
                DomainTaskStatus::Planned,
                "Search, read selected sources, and generate a sourced briefing.",
                json!({
                    "type": "object",
                    "required": ["question"],
                    "properties": {
                        "question": { "type": "string" },
                        "limit": { "type": "integer", "default": 10 },
                        "include_sources": { "type": "boolean", "default": true }
                    }
                }),
            ),
        ],
        planner: planner(
            "hardcoded_domain_planner",
            "TaskPlanner::plan_reader_*",
            "Existing reader tasks are active; briefing should compose search, read_url, and model summary work items.",
        ),
        work_item_types: strings(&["search_web", "read_url", "extract_source_facts", "model_summary"]),
        tools: tools(&[
            ("search_provider.search", "Run the configured web search provider.", true),
            ("reader.read_url", "Fetch and extract readable page text.", true),
        ]),
        artifact_types: artifacts(&[
            ("search_result_set", "Saved search result list."),
            ("readable_web_page", "Saved readable text extracted from a URL."),
            ("web_briefing", "Sourced research briefing."),
        ]),
        model_purposes: models(&[
            ("research_summary", "task_summary_model"),
            ("source_reasoning", "task_reasoning_model"),
        ]),
        policy_tiers: policy(&[
            ("read_public_web", RiskTier::Tier0, false),
            ("external_network_access", RiskTier::Tier1, false),
            ("persist_source_content", RiskTier::Tier1, false),
        ]),
        safety_levels: vec![],
        continuation_rules: continuations(&[
            (
                "search_result_set",
                &["read_selected_urls", "summarize_results", "extract_candidates"],
            ),
            (
                "readable_web_page",
                &["summarize_page", "extract_structured_data", "promote_to_context"],
            ),
        ]),
    }
}

fn code_repo_domain() -> DomainDescriptor {
    DomainDescriptor {
        domain: "code".to_string(),
        display_name: "Code / Repo Operations".to_string(),
        priority: 3,
        description: "Repository inspection, patch planning, implementation notes, and review workflows."
            .to_string(),
        task_types: vec![
            task(
                "code.repo_review",
                DomainTaskStatus::Planned,
                "Review repository state, diffs, or selected files.",
                json!({
                    "type": "object",
                    "properties": {
                        "scope": { "type": "string" },
                        "focus": { "type": "string" },
                        "include_diff": { "type": "boolean", "default": true }
                    }
                }),
            ),
            task(
                "code.generate_patch_plan",
                DomainTaskStatus::Planned,
                "Create an implementation plan without modifying files.",
                json!({
                    "type": "object",
                    "required": ["request"],
                    "properties": {
                        "request": { "type": "string" },
                        "target_paths": { "type": "array", "items": { "type": "string" } }
                    }
                }),
            ),
            task(
                "code.apply_patch",
                DomainTaskStatus::Planned,
                "Apply an approved patch plan through repo-safe editing tools.",
                json!({
                    "type": "object",
                    "required": ["patch_plan_artifact_id"],
                    "properties": {
                        "patch_plan_artifact_id": { "type": "string", "format": "uuid" },
                        "confirm": { "type": "boolean", "default": false }
                    }
                }),
            ),
        ],
        planner: planner(
            "approval_gated_repo_planner",
            "TaskPlanner::plan_code_*",
            "Read-only repo analysis should stay Tier0; file edits require explicit approval and audit links.",
        ),
        work_item_types: strings(&["repo_scan", "diff_analysis", "patch_plan", "apply_patch", "test_run"]),
        tools: tools(&[
            ("repo.read_files", "Read selected repository files.", false),
            ("repo.inspect_diff", "Inspect git diff and status.", false),
            ("repo.apply_patch", "Apply approved source edits.", false),
            ("repo.run_tests", "Run approved validation commands.", false),
        ]),
        artifact_types: artifacts(&[
            ("repo_review", "Repository review findings."),
            ("patch_plan", "Proposed implementation plan."),
            ("code_change_summary", "Summary of files changed and validation."),
        ]),
        model_purposes: models(&[
            ("code_review", "task_reasoning_model"),
            ("patch_planning", "task_reasoning_model"),
            ("change_summary", "task_summary_model"),
        ]),
        policy_tiers: policy(&[
            ("read_repo", RiskTier::Tier0, false),
            ("run_tests", RiskTier::Tier1, false),
            ("write_files", RiskTier::Tier2, true),
            ("destructive_git_operation", RiskTier::Tier3, true),
        ]),
        safety_levels: vec![],
        continuation_rules: continuations(&[
            (
                "patch_plan",
                &["approve_apply_patch", "split_into_code_tasks", "escalate_for_review"],
            ),
            (
                "repo_review",
                &["create_patch_plan", "create_follow_up_tasks", "update_readme"],
            ),
        ]),
    }
}

fn infrastructure_domain() -> DomainDescriptor {
    DomainDescriptor {
        domain: "infrastructure".to_string(),
        display_name: "Local Infrastructure Monitoring".to_string(),
        priority: 4,
        description: "Local machine, container, and service health monitoring.".to_string(),
        task_types: vec![
            task(
                "system.status_report",
                DomainTaskStatus::Active,
                "Collect and summarize local system status.",
                json!({
                    "type": "object",
                    "properties": {
                        "include_docker": { "type": "boolean", "default": true },
                        "include_home": { "type": "boolean", "default": true }
                    }
                }),
            ),
            task(
                "infrastructure.docker_status",
                DomainTaskStatus::Planned,
                "Capture Docker container status and highlight unhealthy services.",
                json!({
                    "type": "object",
                    "properties": {
                        "include_all": { "type": "boolean", "default": true }
                    }
                }),
            ),
        ],
        planner: planner(
            "hardcoded_domain_planner",
            "TaskPlanner::plan_system_status_report",
            "Current status report is active; Docker-specific workflow can reuse docker.list_containers.",
        ),
        work_item_types: strings(&["system_probe", "docker_probe", "health_summary"]),
        tools: tools(&[
            ("system.get_status", "Collect local process and system status.", true),
            ("docker.list_containers", "Inspect local Docker containers.", true),
        ]),
        artifact_types: artifacts(&[
            ("system_status_report", "Local system status summary."),
            ("docker_container_snapshot", "Container state snapshot."),
            ("infrastructure_health_report", "Combined infrastructure health report."),
        ]),
        model_purposes: models(&[
            ("infrastructure_summary", "task_summary_model"),
            ("incident_reasoning", "task_reasoning_model"),
        ]),
        policy_tiers: policy(&[
            ("read_status", RiskTier::Tier0, false),
            ("restart_service", RiskTier::Tier2, true),
            ("delete_or_recreate_service", RiskTier::Tier3, true),
        ]),
        safety_levels: vec![],
        continuation_rules: continuations(&[
            (
                "system_status_report",
                &["explain_issue", "create_monitoring_task", "escalate_failure"],
            ),
            (
                "docker_container_snapshot",
                &["summarize_container_health", "create_remediation_plan"],
            ),
        ]),
    }
}

fn knowledge_domain() -> DomainDescriptor {
    DomainDescriptor {
        domain: "knowledge".to_string(),
        display_name: "Document Intake / Knowledge Management".to_string(),
        priority: 5,
        description: "Artifact summarization, document intake, and promotion into saved context."
            .to_string(),
        task_types: vec![
            task(
                "artifact.summarize",
                DomainTaskStatus::Active,
                "Summarize or continue from a saved artifact.",
                json!({
                    "type": "object",
                    "required": ["user_request", "artifact_name", "artifact_type"],
                    "properties": {
                        "user_request": { "type": "string" },
                        "artifact_name": { "type": "string" },
                        "artifact_type": { "type": "string" },
                        "source_artifact_id": { "type": "string", "format": "uuid" }
                    }
                }),
            ),
            task(
                "knowledge.promote_artifact_to_context",
                DomainTaskStatus::Planned,
                "Save useful artifact content into durable context memory.",
                json!({
                    "type": "object",
                    "required": ["artifact_id", "title"],
                    "properties": {
                        "artifact_id": { "type": "string", "format": "uuid" },
                        "title": { "type": "string" },
                        "tags": { "type": "array", "items": { "type": "string" } }
                    }
                }),
            ),
            task(
                "knowledge.document_intake",
                DomainTaskStatus::Planned,
                "Read, normalize, summarize, and index a document.",
                json!({
                    "type": "object",
                    "properties": {
                        "source_url": { "type": "string" },
                        "content_text": { "type": "string" },
                        "tags": { "type": "array", "items": { "type": "string" } }
                    }
                }),
            ),
        ],
        planner: planner(
            "artifact_first_planner",
            "TaskPlanner::plan_artifact_summary",
            "Existing artifact continuation is active; document intake should produce reusable context artifacts.",
        ),
        work_item_types: strings(&["artifact_load", "document_parse", "model_summary", "context_save"]),
        tools: tools(&[
            ("context.save", "Persist approved knowledge notes.", true),
            ("reader.read_url", "Fetch source documents by URL.", true),
        ]),
        artifact_types: artifacts(&[
            (
                "artifact_continuation_summary",
                "Summary produced from a prior artifact.",
            ),
            ("document_intake_summary", "Normalized document summary."),
            ("saved_context_note", "Context note created from an artifact."),
        ]),
        model_purposes: models(&[
            ("document_summary", "task_summary_model"),
            ("knowledge_extraction", "task_reasoning_model"),
        ]),
        policy_tiers: policy(&[
            ("read_artifact", RiskTier::Tier0, false),
            ("save_context", RiskTier::Tier1, false),
            ("personal_document_intake", RiskTier::Tier2, true),
        ]),
        safety_levels: vec![],
        continuation_rules: continuations(&[
            (
                "readable_web_page",
                &["summarize_page", "extract_structured_data", "save_as_context"],
            ),
            (
                "artifact_continuation_summary",
                &["promote_to_context", "create_follow_up_task"],
            ),
        ]),
    }
}

fn calendar_domain() -> DomainDescriptor {
    DomainDescriptor {
        domain: "calendar".to_string(),
        display_name: "Calendar / Planning".to_string(),
        priority: 6,
        description: "Agenda review, planning, scheduling drafts, and time-block recommendations."
            .to_string(),
        task_types: vec![
            task(
                "calendar.daily_plan",
                DomainTaskStatus::Planned,
                "Create a day plan from calendar context and task priorities.",
                json!({
                    "type": "object",
                    "properties": {
                        "date": { "type": "string", "format": "date" },
                        "include_tasks": { "type": "boolean", "default": true }
                    }
                }),
            ),
            task(
                "calendar.availability_review",
                DomainTaskStatus::Planned,
                "Find useful free windows and scheduling conflicts.",
                json!({
                    "type": "object",
                    "properties": {
                        "date": { "type": "string", "format": "date" },
                        "duration_minutes": { "type": "integer", "minimum": 15 }
                    }
                }),
            ),
            task(
                "calendar.schedule_draft",
                DomainTaskStatus::Planned,
                "Draft a calendar change without applying it automatically.",
                json!({
                    "type": "object",
                    "required": ["request"],
                    "properties": {
                        "request": { "type": "string" },
                        "confirm": { "type": "boolean", "default": false }
                    }
                }),
            ),
        ],
        planner: planner(
            "connector_backed_planner",
            "TaskPlanner::plan_calendar_*",
            "Calendar work should read first, draft changes second, and require confirmation before writes.",
        ),
        work_item_types: strings(&["calendar_read", "availability_analysis", "plan_generation", "calendar_draft"]),
        tools: tools(&[
            ("calendar.list_events", "Read calendar events.", false),
            ("calendar.find_availability", "Find open scheduling windows.", false),
            ("calendar.draft_change", "Prepare a proposed calendar update.", false),
        ]),
        artifact_types: artifacts(&[
            ("calendar_day_snapshot", "Calendar events and availability snapshot."),
            ("daily_plan", "Generated day plan."),
            ("calendar_change_draft", "Proposed calendar change awaiting approval."),
        ]),
        model_purposes: models(&[
            ("planning_summary", "task_summary_model"),
            ("schedule_reasoning", "task_reasoning_model"),
        ]),
        policy_tiers: policy(&[
            ("read_calendar", RiskTier::Tier1, false),
            ("draft_calendar_change", RiskTier::Tier1, false),
            ("write_calendar", RiskTier::Tier2, true),
        ]),
        safety_levels: vec![],
        continuation_rules: continuations(&[
            (
                "calendar_day_snapshot",
                &["create_daily_plan", "find_focus_block", "draft_schedule_change"],
            ),
            (
                "daily_plan",
                &["create_task_list", "draft_calendar_blocks"],
            ),
        ]),
    }
}

fn operator_domain() -> DomainDescriptor {
    DomainDescriptor {
        domain: "operator".to_string(),
        display_name: "Operator / Task Improvement".to_string(),
        priority: 7,
        description: "Local Operator self-improvement loops: failed tasks, escalation, recommendations, specs, and follow-up tasks."
            .to_string(),
        task_types: vec![
            task(
                "operator.escalate_to_chatgpt",
                DomainTaskStatus::Active,
                "Prepare a redacted ChatGPT escalation request or send one through the configured OpenAI provider.",
                json!({
                    "type": "object",
                    "required": ["user_request"],
                    "properties": {
                        "user_request": { "type": "string" },
                        "mode": { "type": "string", "enum": ["manual", "openai"], "default": "manual" },
                        "confirm": { "type": "boolean", "default": false },
                        "desired_output": { "type": "string" },
                        "context_text": { "type": "string" },
                        "context_json": { "type": "object", "additionalProperties": true }
                    }
                }),
            ),
            task(
                "system.escalate_to_chatgpt",
                DomainTaskStatus::Alias,
                "Legacy alias for operator.escalate_to_chatgpt.",
                json!({
                    "type": "object",
                    "description": "Alias schema is the same as operator.escalate_to_chatgpt."
                }),
            ),
            task(
                "operator.review_failed_task",
                DomainTaskStatus::Active,
                "Review one failed run and save a read-only operator_task_diagnostic artifact.",
                json!({
                    "type": "object",
                    "required": ["run_id"],
                    "properties": {
                        "run_id": { "type": "string", "format": "uuid" },
                        "include_task": { "type": "boolean", "default": true },
                        "include_artifacts": { "type": "boolean", "default": true },
                        "include_recent_audit": { "type": "boolean", "default": true },
                        "include_repo_context": { "type": "boolean", "default": false },
                        "escalate_if_needed": { "type": "boolean", "default": false }
                    }
                }),
            ),
            task(
                "operator.review_recent_tasks",
                DomainTaskStatus::Planned,
                "Review recent task runs and identify repeated failures or weak workflows.",
                json!({
                    "type": "object",
                    "properties": {
                        "limit": { "type": "integer", "minimum": 1, "default": 10 },
                        "only_failed": { "type": "boolean", "default": false }
                    }
                }),
            ),
            task(
                "operator.design_task_type",
                DomainTaskStatus::Planned,
                "Design a task type, planner, artifacts, policy, and OpenAPI surface.",
                json!({
                    "type": "object",
                    "required": ["request"],
                    "properties": {
                        "request": { "type": "string" },
                        "domain": { "type": "string" }
                    }
                }),
            ),
            task(
                "operator.design_tool",
                DomainTaskStatus::Planned,
                "Create a tool descriptor and policy proposal.",
                json!({
                    "type": "object",
                    "required": ["tool_goal"],
                    "properties": {
                        "tool_goal": { "type": "string" },
                        "risk_assessment": { "type": "string" }
                    }
                }),
            ),
            task(
                "operator.generate_patch_plan",
                DomainTaskStatus::Active,
                "Generate an operator_patch_plan artifact from an operator_task_diagnostic without editing files.",
                json!({
                    "type": "object",
                    "required": ["artifact_id"],
                    "properties": {
                        "artifact_id": { "type": "string", "format": "uuid" },
                        "source_artifact_id": { "type": "string", "format": "uuid" },
                        "diagnostic_artifact_id": { "type": "string", "format": "uuid" },
                        "title": { "type": "string" }
                    }
                }),
            ),
            task(
                "operator.review_openapi_surface",
                DomainTaskStatus::Planned,
                "Generate or review OpenAPI changes for task-oriented tool use.",
                json!({
                    "type": "object",
                    "properties": {
                        "operation_ids": { "type": "array", "items": { "type": "string" } }
                    }
                }),
            ),
            task(
                "operator.convert_recommendation_to_tasks",
                DomainTaskStatus::Active,
                "Convert an operator_patch_plan into an operator_implementation_task_set artifact. Does not create or run OpTasks.",
                json!({
                    "type": "object",
                    "required": ["artifact_id"],
                    "properties": {
                        "artifact_id": { "type": "string", "format": "uuid" },
                        "source_artifact_id": { "type": "string", "format": "uuid" },
                        "patch_plan_artifact_id": { "type": "string", "format": "uuid" }
                    }
                }),
            ),
            task(
                "operator.update_docs_plan",
                DomainTaskStatus::Planned,
                "Generate a documentation update plan from current capabilities.",
                json!({
                    "type": "object",
                    "properties": {
                        "focus": { "type": "string" }
                    }
                }),
            ),
        ],
        planner: planner(
            "read_only_diagnostic_loop",
            "OperatorTaskPlanner",
            "OperatorTaskPlanner owns read-only diagnosis, patch planning, implementation task-set planning, and escalation request plans. Follow-up task creation and escalation stay approval-gated.",
        ),
        work_item_types: strings(&[
            "failed_run_load",
            "task_context_load",
            "artifact_load",
            "audit_load",
            "failure_classification",
            "diagnostic_artifact",
            "redaction",
            "escalation_request",
            "response_parse",
            "follow_up_task_creation",
        ]),
        tools: tools(&[
            ("op_tasks.get_run", "Read task run state and work items.", true),
            ("op_tasks.get_task", "Read task definition.", true),
            ("op_tasks.list_artifacts", "Read task artifacts.", true),
            ("audit.recent_for_run", "Read audited tool/model attempts for a run.", true),
            ("artifact.save", "Persist diagnostic and escalation artifacts.", true),
            ("task.create", "Create approved follow-up OpTasks from recommendations.", true),
        ]),
        artifact_types: artifacts(&[
            (
                "operator_task_diagnostic",
                "Read-only failed task diagnostic with evidence and recommended actions.",
            ),
            (
                "operator_gap_analysis",
                "Gap analysis for a missing or weak Local Operator capability.",
            ),
            (
                "operator_task_type_spec",
                "Proposed task type contract, inputs, planner, artifacts, and policy.",
            ),
            (
                "operator_tool_spec",
                "Proposed tool descriptor, inputs, outputs, and risk tier.",
            ),
            (
                "operator_patch_plan",
                "Read-only code or configuration patch plan; no files changed.",
            ),
            (
                "operator_test_plan",
                "Validation plan for an operator change or new task workflow.",
            ),
            (
                "operator_openapi_review",
                "Review of task-oriented OpenAPI tool surface and operation guidance.",
            ),
            (
                "operator_implementation_task_set",
                "Proposed follow-up OpTasks generated from recommendations; approval required before creation or execution.",
            ),
            (
                "chatgpt_escalation_request",
                "Redacted request intended for ChatGPT.",
            ),
            (
                "chatgpt_escalation_response",
                "Saved ChatGPT response with recommendations.",
            ),
        ]),
        model_purposes: models(&[
            ("failure_classification", "task_extraction_model"),
            ("root_cause_analysis", "deep_model"),
            ("patch_plan", "coder_model"),
            ("readme_openapi_wording", "task_writing_model"),
            ("final_summary", "task_summary_model"),
            ("escalation_packet", "task_writing_model"),
            ("escalation_redaction", "task_reasoning_model"),
            ("escalation_follow_up", "task_summary_model"),
            ("implementation_task_planning", "task_reasoning_model"),
        ]),
        policy_tiers: policy(&[
            ("read_failed_task", RiskTier::Tier0, false),
            ("technical_escalation", RiskTier::Tier0, false),
            ("personal_or_employment_escalation", RiskTier::Tier2, true),
            ("create_follow_up_tasks", RiskTier::Tier1, true),
            ("create_draft_tasks", RiskTier::Tier1, true),
            ("modify_repo_code_config", RiskTier::Tier3, true),
            ("execute_operational_change", RiskTier::Tier3, true),
            ("secret_bearing_context", RiskTier::Tier3, true),
        ]),
        safety_levels: operator_safety_levels(),
        continuation_rules: continuations(&[
            (
                "operator_task_diagnostic",
                &[
                    "generate_patch_plan",
                    "escalate_to_chatgpt",
                    "convert_recommendation_to_tasks",
                ],
            ),
            (
                "operator_patch_plan",
                &["convert_recommendation_to_tasks", "generate_patch_plan", "summarize_artifact"],
            ),
            (
                "operator_tool_spec",
                &["create_tool_implementation_plan"],
            ),
            (
                "operator_openapi_review",
                &["summarize_artifact"],
            ),
            (
                "operator_implementation_task_set",
                &["approve_create_tasks", "continue_from_task_set"],
            ),
            (
                "chatgpt_escalation_response",
                &["generate_patch_plan", "convert_recommendation_to_tasks", "summarize_artifact"],
            ),
        ]),
    }
}

fn task(
    name: &str,
    status: DomainTaskStatus,
    description: &str,
    input_schema: Value,
) -> DomainTaskType {
    DomainTaskType {
        name: name.to_string(),
        status,
        description: description.to_string(),
        input_schema,
    }
}

fn planner(strategy: &str, planner_module: &str, notes: &str) -> DomainPlanner {
    DomainPlanner {
        strategy: strategy.to_string(),
        planner_module: planner_module.to_string(),
        notes: notes.to_string(),
    }
}

fn tools(items: &[(&str, &str, bool)]) -> Vec<DomainTool> {
    items
        .iter()
        .map(|(name, purpose, required_now)| DomainTool {
            name: (*name).to_string(),
            purpose: (*purpose).to_string(),
            required_now: *required_now,
        })
        .collect()
}

fn artifacts(items: &[(&str, &str)]) -> Vec<DomainArtifactType> {
    items
        .iter()
        .map(|(name, description)| DomainArtifactType {
            name: (*name).to_string(),
            description: (*description).to_string(),
        })
        .collect()
}

fn models(items: &[(&str, &str)]) -> Vec<DomainModelPurpose> {
    items
        .iter()
        .map(|(purpose, model_route)| DomainModelPurpose {
            purpose: (*purpose).to_string(),
            model_route: (*model_route).to_string(),
        })
        .collect()
}

fn policy(items: &[(&str, RiskTier, bool)]) -> Vec<DomainPolicyTier> {
    items
        .iter()
        .map(
            |(operation, risk_tier, requires_confirmation)| DomainPolicyTier {
                operation: (*operation).to_string(),
                risk_tier: *risk_tier,
                requires_confirmation: *requires_confirmation,
            },
        )
        .collect()
}

fn continuations(items: &[(&str, &[&str])]) -> Vec<DomainContinuationRule> {
    items
        .iter()
        .map(
            |(source_artifact_type, continuations)| DomainContinuationRule {
                source_artifact_type: (*source_artifact_type).to_string(),
                continuations: strings(continuations),
            },
        )
        .collect()
}

fn operator_safety_levels() -> Vec<DomainSafetyLevel> {
    vec![
        safety_level(
            1,
            "Diagnose only",
            "Read task state, read artifacts, summarize failures, and suggest fixes. No task creation beyond the diagnostic task itself.",
            &[
                "read_task_state",
                "read_artifacts",
                "summarize_failure",
                "suggest_fix",
            ],
            false,
            "active",
        ),
        safety_level(
            2,
            "Plan only",
            "Generate plans and specs as artifacts. No code, config, or operational changes are applied.",
            &[
                "generate_patch_plan",
                "generate_task_specs",
                "generate_tool_specs",
                "generate_openapi_update_plans",
            ],
            false,
            "active",
        ),
        safety_level(
            3,
            "Create draft tasks",
            "Create draft implementation, docs, or test tasks only after explicit confirmation. Draft tasks are not executed automatically.",
            &[
                "create_draft_implementation_tasks",
                "create_draft_docs_tasks",
                "create_draft_test_tasks",
            ],
            true,
            "active",
        ),
        safety_level(
            4,
            "Modify repo/code/config",
            "Future repo operations such as branch creation, patch writing, test execution, or pull request creation. Not enabled in the operator MVP.",
            &["create_branch", "write_patch", "run_tests", "open_pr"],
            true,
            "blocked_for_now",
        ),
        safety_level(
            5,
            "Execute operational changes",
            "Operational changes such as restarting containers, changing Home Assistant automations, or altering secrets/config. Blocked or requires very high confirmation later.",
            &[
                "restart_containers",
                "change_home_assistant_automations",
                "alter_secrets_or_config",
            ],
            true,
            "blocked_for_now",
        ),
    ]
}

fn safety_level(
    level: u8,
    name: &str,
    description: &str,
    allowed: &[&str],
    requires_confirmation: bool,
    status: &str,
) -> DomainSafetyLevel {
    DomainSafetyLevel {
        level,
        name: name.to_string(),
        description: description.to_string(),
        allowed: strings(allowed),
        requires_confirmation,
        status: status.to_string(),
    }
}

fn strings(items: &[&str]) -> Vec<String> {
    items.iter().map(|item| (*item).to_string()).collect()
}

#[cfg(test)]
mod tests {
    use super::{domain_catalog, find_domain, DomainTaskStatus};

    #[test]
    fn catalog_contains_prioritized_domains() {
        let domains = domain_catalog();
        let ids = domains
            .iter()
            .map(|domain| domain.domain.as_str())
            .collect::<Vec<_>>();

        assert_eq!(
            ids,
            vec![
                "home",
                "research",
                "code",
                "infrastructure",
                "knowledge",
                "calendar",
                "operator"
            ]
        );
    }

    #[test]
    fn operator_domain_exposes_diagnostic_and_escalation_loop() {
        let operator = find_domain("operator").expect("operator domain");

        assert!(operator.task_types.iter().any(|task| {
            task.name == "operator.review_failed_task" && task.status == DomainTaskStatus::Active
        }));
        assert!(operator.task_types.iter().any(|task| {
            task.name == "operator.escalate_to_chatgpt" && task.status == DomainTaskStatus::Active
        }));
        assert!(operator.continuation_rules.iter().any(|rule| {
            rule.source_artifact_type == "chatgpt_escalation_response"
                && rule
                    .continuations
                    .contains(&"convert_recommendation_to_tasks".to_string())
        }));
        assert!(operator
            .safety_levels
            .iter()
            .any(|level| { level.level == 4 && level.status == "blocked_for_now" }));
        assert!(operator
            .safety_levels
            .iter()
            .any(|level| { level.level == 5 && level.status == "blocked_for_now" }));
    }
}
