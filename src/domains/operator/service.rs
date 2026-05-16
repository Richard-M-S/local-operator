use anyhow::{anyhow, Context};
use chrono::Utc;
use serde_json::{json, Value};
use uuid::Uuid;

use crate::{
    op_tasks::{
        models::{OpTask, OpTaskRun, OpTaskRunStatus, OpTaskStatus, TaskArtifact},
        OpTaskRepository,
    },
    services::audit_service::{AuditExecutionSummary, AuditService},
};

use super::models::{
    FailedTaskReviewContext, OperatorDiagnosticArtifact, OperatorDiagnosticEvidence,
    OperatorDiagnosticSource, OperatorEscalationRequestPacket, OperatorFailureClassification,
    OperatorImplementationTaskSet, OperatorPatchPlan, OperatorPatchPlanChange,
    OperatorRecommendationPriority, OperatorRecommendedAction, OperatorReviewFailedTaskInput,
    OperatorTaskDiagnostic, OperatorTaskSpec, OperatorTaskStateQuery, OperatorTaskStateSnapshot,
    CHATGPT_ESCALATION_REQUEST, OPERATOR_IMPLEMENTATION_TASK_SET, OPERATOR_PATCH_PLAN,
    OPERATOR_TASK_DIAGNOSTIC,
};

#[derive(Clone)]
pub struct OperatorMetaService {
    op_tasks: OpTaskRepository,
    audit: AuditService,
}

impl OperatorMetaService {
    pub fn new(op_tasks: OpTaskRepository, audit: AuditService) -> Self {
        Self { op_tasks, audit }
    }

    pub async fn load_failed_run(
        &self,
        profile_id: Uuid,
        run_id: Uuid,
    ) -> anyhow::Result<OpTaskRun> {
        let run = self
            .op_tasks
            .get_task_run(run_id)
            .await?
            .ok_or_else(|| anyhow!("op task run {} not found", run_id))?;

        if run.profile_id != profile_id {
            return Err(anyhow!("op task run {} not found", run_id));
        }
        if run.status != OpTaskRunStatus::Failed {
            return Err(anyhow!(
                "op task run {} is {:?}, not Failed",
                run_id,
                run.status
            ));
        }

        Ok(run)
    }

    pub async fn load_task_definition(
        &self,
        run: &OpTaskRun,
        include_task: bool,
    ) -> anyhow::Result<Option<OpTask>> {
        if !include_task {
            return Ok(None);
        }

        self.op_tasks
            .get_op_task(run.task_id)
            .await?
            .ok_or_else(|| anyhow!("op task {} not found", run.task_id))
            .map(Some)
    }

    pub fn load_artifacts(&self, run: &OpTaskRun, include_artifacts: bool) -> Vec<TaskArtifact> {
        if include_artifacts {
            run.artifacts.clone()
        } else {
            vec![]
        }
    }

    pub async fn load_recent_audit(
        &self,
        run_id: Uuid,
        include_recent_audit: bool,
    ) -> anyhow::Result<Vec<AuditExecutionSummary>> {
        if include_recent_audit {
            self.audit
                .recent_for_run(run_id, 100)
                .await
                .context("failed to load audit entries")
        } else {
            Ok(vec![])
        }
    }

    pub async fn inspect_task_state(
        &self,
        query: OperatorTaskStateQuery,
    ) -> anyhow::Result<OperatorTaskStateSnapshot> {
        let mut tasks = Vec::new();
        let mut runs = Vec::new();
        let mut artifacts = Vec::new();

        if let Some(artifact_id) = query.artifact_id {
            if let Some(artifact) = self.op_tasks.get_artifact(artifact_id).await? {
                artifacts.push(artifact);
            }
        }

        if let Some(run_id) = query.run_id {
            if let Some(run) = self.op_tasks.get_task_run(run_id).await? {
                if query
                    .profile_id
                    .is_none_or(|profile_id| profile_id == run.profile_id)
                {
                    runs.push(run);
                }
            }
        }

        if let Some(task_id) = query.task_id {
            if let Some(task) = self.op_tasks.get_op_task(task_id).await? {
                if query
                    .profile_id
                    .is_none_or(|profile_id| profile_id == task.profile_id)
                {
                    if query.run_id.is_none() {
                        runs.extend(self.op_tasks.list_task_runs_for_task(task.id).await?);
                    }
                    tasks.push(task);
                }
            }
        } else if let Some(profile_id) = query.profile_id {
            tasks.extend(self.op_tasks.list_op_tasks(profile_id).await?);
        }

        if query.artifact_id.is_none() {
            artifacts.extend(self.op_tasks.list_artifacts((&query).into()).await?);
        }

        Ok(OperatorTaskStateSnapshot {
            filters: query,
            tasks,
            runs,
            artifacts,
            note: "Work items are currently embedded in op_task_runs.work_items JSON; promote them to their own table later for mature operator-domain querying.".to_string(),
        })
    }

    pub async fn load_artifact(
        &self,
        profile_id: Uuid,
        artifact_id: Uuid,
        expected_artifact_type: &str,
    ) -> anyhow::Result<TaskArtifact> {
        let artifact = self
            .op_tasks
            .get_artifact(artifact_id)
            .await?
            .ok_or_else(|| anyhow!("artifact {} not found", artifact_id))?;
        if artifact.profile_id != profile_id {
            return Err(anyhow!("artifact {} not found", artifact_id));
        }
        if artifact.artifact_type != expected_artifact_type {
            return Err(anyhow!(
                "artifact {} is {}, not {}",
                artifact_id,
                artifact.artifact_type,
                expected_artifact_type
            ));
        }

        Ok(artifact)
    }

    pub fn build_review_context(
        &self,
        input: OperatorReviewFailedTaskInput,
        run: OpTaskRun,
        task: Option<OpTask>,
        artifacts: Vec<TaskArtifact>,
        audit_entries: Vec<AuditExecutionSummary>,
    ) -> FailedTaskReviewContext {
        FailedTaskReviewContext {
            task,
            run,
            artifacts,
            audit_entries,
            input,
        }
    }

    pub fn build_diagnostic_packet(
        &self,
        context: &FailedTaskReviewContext,
    ) -> OperatorTaskDiagnostic {
        let failure_classification = self.classify_failure(context);
        let summary =
            summarize_failure(failure_classification, context.task.as_ref(), &context.run);
        let evidence = self.build_evidence(context);
        let recommended_actions = self.create_improvement_recommendations(
            failure_classification,
            context.input.escalate_if_needed,
        );
        let next_actions = next_actions_for_failure(context.input.escalate_if_needed);

        OperatorTaskDiagnostic {
            schema_version: "1.0".to_string(),
            diagnostic_type: "failed_task_review".to_string(),
            source: OperatorDiagnosticSource {
                task_id: context.run.task_id,
                run_id: context.run.id,
            },
            failure_classification,
            summary,
            evidence,
            recommended_actions,
            next_actions,
            review_context: review_context_json(context),
            read_only: true,
            actions_executed_by_local_operator: vec![],
        }
    }

    pub fn classify_failure(
        &self,
        context: &FailedTaskReviewContext,
    ) -> OperatorFailureClassification {
        let evidence_text = failure_evidence_text(context);
        let normalized = evidence_text.to_lowercase();

        if contains_any(
            &normalized,
            &[
                "policy denied",
                "requires confirmation",
                "tier 1",
                "tier 2",
                "tier 3",
                "blocked",
            ],
        ) {
            OperatorFailureClassification::PolicyDenied
        } else if contains_any(
            &normalized,
            &[
                "invalid",
                "missing required",
                "failed to parse",
                "input_json",
                "bad request",
                "has no json content",
                "missing source url",
            ],
        ) {
            OperatorFailureClassification::BadTaskInput
        } else if contains_any(
            &normalized,
            &[
                "llm",
                "model",
                "ollama",
                "openai",
                "chat completion",
                "context length",
            ],
        ) {
            OperatorFailureClassification::ModelError
        } else if contains_any(
            &normalized,
            &[
                "tool not found",
                "descriptor_error",
                "reader",
                "search provider",
                "duckduckgo",
                "http",
                "request failed",
                "timeout",
            ],
        ) {
            OperatorFailureClassification::ToolError
        } else if contains_any(
            &normalized,
            &[
                "no search results",
                "generic",
                "not location",
                "irrelevant",
                "bad search",
                "ignored",
            ],
        ) {
            OperatorFailureClassification::BadSearchResults
        } else if contains_any(
            &normalized,
            &[
                "missing context",
                "no saved context",
                "profile criteria",
                "criteria",
            ],
        ) {
            OperatorFailureClassification::MissingContext
        } else if contains_any(
            &normalized,
            &[
                "panic",
                "sqlx",
                "database",
                "internal",
                "unsupported task type",
                "not implemented",
            ],
        ) {
            OperatorFailureClassification::CodeBug
        } else {
            OperatorFailureClassification::Unknown
        }
    }

    pub fn create_improvement_recommendations(
        &self,
        classification: OperatorFailureClassification,
        escalate_if_needed: bool,
    ) -> Vec<OperatorRecommendedAction> {
        let mut actions = match classification {
            OperatorFailureClassification::BadSearchResults => vec![recommendation(
                "Improve search query construction and provider diagnostics",
                OperatorRecommendationPriority::High,
                "operator.generate_patch_plan",
            )],
            OperatorFailureClassification::ToolError => vec![recommendation(
                "Review tool/provider descriptor, inputs, and error handling",
                OperatorRecommendationPriority::High,
                "operator.design_tool",
            )],
            OperatorFailureClassification::ModelError => vec![recommendation(
                "Review model prompt, output parsing, and fallback behavior",
                OperatorRecommendationPriority::Medium,
                "operator.generate_patch_plan",
            )],
            OperatorFailureClassification::MissingContext => vec![recommendation(
                "Add or promote required context before retrying the task",
                OperatorRecommendationPriority::Medium,
                "knowledge.document_intake",
            )],
            OperatorFailureClassification::BadTaskInput => vec![recommendation(
                "Tighten task input schema and natural-language intake mapping",
                OperatorRecommendationPriority::High,
                "operator.design_task_type",
            )],
            OperatorFailureClassification::PolicyDenied => vec![recommendation(
                "Retry with explicit confirmation or revise the policy tier",
                OperatorRecommendationPriority::Medium,
                "operator.review_openapi_surface",
            )],
            OperatorFailureClassification::CodeBug => vec![recommendation(
                "Generate a patch plan for the failing code path",
                OperatorRecommendationPriority::High,
                "operator.generate_patch_plan",
            )],
            OperatorFailureClassification::Unknown => vec![recommendation(
                "Escalate the diagnostic for deeper review",
                OperatorRecommendationPriority::Medium,
                "operator.escalate_to_chatgpt",
            )],
        };

        if escalate_if_needed
            && !actions
                .iter()
                .any(|action| action.task_type == "operator.escalate_to_chatgpt")
        {
            actions.push(recommendation(
                "Escalate this diagnostic to ChatGPT for second-pass review",
                OperatorRecommendationPriority::Medium,
                "operator.escalate_to_chatgpt",
            ));
        }

        actions
    }

    pub fn diagnostic_artifact(
        &self,
        diagnostic: &OperatorTaskDiagnostic,
    ) -> anyhow::Result<OperatorDiagnosticArtifact> {
        let content_json = serde_json::to_value(diagnostic)?;
        let content_text = render_operator_diagnostic_text(&content_json);

        Ok(OperatorDiagnosticArtifact {
            name: "Operator failed task diagnostic".to_string(),
            artifact_type: OPERATOR_TASK_DIAGNOSTIC.to_string(),
            metadata: json!({
                "schema_version": diagnostic.schema_version,
                "diagnostic_type": diagnostic.diagnostic_type,
                "source_task_id": diagnostic.source.task_id,
                "source_run_id": diagnostic.source.run_id,
                "failure_classification": diagnostic.failure_classification.as_str(),
                "read_only": true,
            }),
            content_text,
            content_json,
        })
    }

    pub fn patch_plan_artifact(
        &self,
        source_artifact: &TaskArtifact,
        title: impl Into<String>,
    ) -> anyhow::Result<OperatorDiagnosticArtifact> {
        let diagnostic = source_artifact
            .content_json
            .as_ref()
            .ok_or_else(|| anyhow!("operator_task_diagnostic has no content_json"))?;
        let title = title.into();
        let source_summary = diagnostic
            .get("summary")
            .and_then(|value| value.as_str())
            .unwrap_or("Review the diagnostic and prepare a focused patch plan.");
        let failure_classification = diagnostic
            .get("failure_classification")
            .and_then(|value| value.as_str())
            .unwrap_or("unknown");
        let files_to_change = infer_patch_files(diagnostic, failure_classification);
        let changes = infer_patch_changes(&files_to_change, diagnostic, failure_classification);
        let plan = OperatorPatchPlan {
            schema_version: "1.0".to_string(),
            source_artifact_id: Some(source_artifact.id),
            title,
            summary: source_summary.to_string(),
            files_to_change,
            changes,
            risks: vec![
                "May change existing default behavior.".to_string(),
                "Patch plan is inferred from diagnostic evidence and needs human review."
                    .to_string(),
            ],
            tests: vec![
                "Existing behavior covered by current tests still passes.".to_string(),
                "New or changed behavior is covered by focused tests.".to_string(),
                "cargo fmt, cargo check, and cargo test pass.".to_string(),
            ],
            read_only: true,
            actions_executed_by_local_operator: vec![],
        };
        let content_json = serde_json::to_value(&plan)?;
        let content_text = format!(
            "Operator patch plan\n\n{}\n\n{} proposed change(s).",
            plan.summary,
            plan.changes.len()
        );

        Ok(OperatorDiagnosticArtifact {
            name: "Operator patch plan".to_string(),
            artifact_type: OPERATOR_PATCH_PLAN.to_string(),
            metadata: json!({
                "schema_version": plan.schema_version,
                "source_artifact_id": source_artifact.id,
                "read_only": true,
            }),
            content_text,
            content_json,
        })
    }

    pub fn implementation_task_set_artifact(
        &self,
        profile_id: Uuid,
        source_artifact: &TaskArtifact,
    ) -> anyhow::Result<OperatorDiagnosticArtifact> {
        let patch_plan = source_artifact
            .content_json
            .as_ref()
            .ok_or_else(|| anyhow!("operator_patch_plan has no content_json"))?;
        let task_set = OperatorImplementationTaskSet {
            schema_version: "1.0".to_string(),
            source_artifact_id: source_artifact.id,
            tasks: implementation_tasks_from_patch_plan(profile_id, source_artifact.id, patch_plan),
            approval_required: true,
            read_only: true,
            actions_executed_by_local_operator: vec![],
        };
        let content_json = serde_json::to_value(&task_set)?;
        let content_text = format!(
            "Operator implementation task set\n\n{} proposed task(s). Approval is required before creating or running tasks.",
            task_set.tasks.len()
        );

        Ok(OperatorDiagnosticArtifact {
            name: "Operator implementation task set".to_string(),
            artifact_type: OPERATOR_IMPLEMENTATION_TASK_SET.to_string(),
            metadata: json!({
                "schema_version": task_set.schema_version,
                "source_artifact_id": task_set.source_artifact_id,
                "profile_id": profile_id,
                "approval_required": true,
                "read_only": true,
            }),
            content_text,
            content_json,
        })
    }

    #[allow(dead_code)]
    pub fn create_escalation_request_artifact(
        &self,
        profile_id: Uuid,
        run_id: Uuid,
        work_item_id: Option<Uuid>,
        diagnostic: &OperatorTaskDiagnostic,
    ) -> anyhow::Result<TaskArtifact> {
        let packet = OperatorEscalationRequestPacket {
            schema_version: "1.0".to_string(),
            created_at: Utc::now(),
            diagnostic: serde_json::to_value(diagnostic)?,
            desired_output:
                "Return structured findings, recommended actions, and follow-up task specs."
                    .to_string(),
            actions_executed_by_local_operator: vec![],
        };
        let content_json = serde_json::to_value(packet)?;

        Ok(TaskArtifact {
            id: Uuid::new_v4(),
            profile_id,
            run_id,
            work_item_id,
            name: "Operator ChatGPT escalation request".to_string(),
            artifact_type: CHATGPT_ESCALATION_REQUEST.to_string(),
            location: None,
            created_at: Utc::now(),
            metadata: Some(json!({
                "escalation_provider": "chatgpt",
                "direction": "request",
                "source": "operator_task_diagnostic",
                "read_only": true,
                "actions_executed_by_local_operator": [],
            })),
            content_text: Some(serde_json::to_string_pretty(&content_json)?),
            content_json: Some(content_json),
        })
    }

    #[allow(dead_code)]
    pub fn recommendation_task_specs(
        &self,
        profile_id: Uuid,
        source_artifact_id: Uuid,
        recommendations: &[OperatorRecommendedAction],
    ) -> Vec<OperatorTaskSpec> {
        recommendations
            .iter()
            .map(|recommendation| OperatorTaskSpec {
                task_type: recommendation.task_type.clone(),
                name: format!("Follow up: {}", recommendation.title),
                input_json: json!({
                    "profile_id": profile_id,
                    "source_artifact_id": source_artifact_id,
                    "user_request": recommendation.title,
                    "model_purpose": "operator_follow_up",
                }),
            })
            .collect()
    }

    #[allow(dead_code)]
    pub async fn convert_recommendations_into_tasks(
        &self,
        profile_id: Uuid,
        source_artifact_id: Uuid,
        recommendations: &[OperatorRecommendedAction],
        approved: bool,
    ) -> anyhow::Result<Vec<OpTask>> {
        if !approved {
            return Ok(vec![]);
        }

        let mut tasks = Vec::new();
        for spec in self.recommendation_task_specs(profile_id, source_artifact_id, recommendations)
        {
            let task = OpTask {
                id: Uuid::new_v4(),
                profile_id,
                task_type: spec.task_type,
                name: spec.name,
                description: Some(format!(
                    "Created from operator diagnostic artifact {}.",
                    source_artifact_id
                )),
                input_json: spec.input_json,
                status: OpTaskStatus::Paused,
                created_at: Utc::now(),
                updated_at: None,
            };
            tasks.push(self.op_tasks.create_op_task(task).await?);
        }

        Ok(tasks)
    }

    fn build_evidence(&self, context: &FailedTaskReviewContext) -> Vec<OperatorDiagnosticEvidence> {
        let mut evidence = Vec::new();

        if let Some(summary) = &context.run.summary {
            evidence.push(OperatorDiagnosticEvidence {
                kind: "run_summary".to_string(),
                artifact_id: None,
                work_item_id: None,
                detail: truncate_chars(summary, 500),
                metadata: None,
            });
        }

        for item in context
            .run
            .work_items
            .iter()
            .filter(|item| item.error.is_some())
            .take(5)
        {
            evidence.push(OperatorDiagnosticEvidence {
                kind: "work_item_error".to_string(),
                artifact_id: None,
                work_item_id: Some(item.id),
                detail: item.error.clone().unwrap_or_default(),
                metadata: Some(json!({
                    "work_item": item.name,
                    "step_type": item.step_type,
                })),
            });
        }

        if let Some(task) = &context.task {
            evidence.push(OperatorDiagnosticEvidence {
                kind: "task_input".to_string(),
                artifact_id: None,
                work_item_id: None,
                detail: truncate_chars(&task.input_json.to_string(), 700),
                metadata: Some(json!({
                    "task_type": task.task_type,
                    "task_id": task.id,
                })),
            });
        }

        for artifact in context.artifacts.iter().take(5) {
            evidence.push(OperatorDiagnosticEvidence {
                kind: "artifact".to_string(),
                artifact_id: Some(artifact.id),
                work_item_id: artifact.work_item_id,
                detail: artifact
                    .content_text
                    .as_deref()
                    .map(|value| truncate_chars(value, 500))
                    .or_else(|| {
                        artifact
                            .content_json
                            .as_ref()
                            .map(|value| truncate_chars(&value.to_string(), 500))
                    })
                    .unwrap_or_else(|| "artifact has no saved content".to_string()),
                metadata: Some(json!({
                    "artifact_type": artifact.artifact_type,
                    "name": artifact.name,
                })),
            });
        }

        for entry in context
            .audit_entries
            .iter()
            .filter(|entry| entry.error.is_some())
            .take(5)
        {
            evidence.push(OperatorDiagnosticEvidence {
                kind: "audit".to_string(),
                artifact_id: None,
                work_item_id: entry.work_item_id,
                detail: entry.error.clone().unwrap_or_default(),
                metadata: Some(json!({
                    "execution_type": entry.execution_type,
                    "name": entry.name,
                    "policy_decision": entry.policy_decision,
                })),
            });
        }

        evidence
    }
}

fn failure_evidence_text(context: &FailedTaskReviewContext) -> String {
    let mut parts = Vec::new();
    if let Some(task) = &context.task {
        parts.push(task.task_type.clone());
        parts.push(task.input_json.to_string());
    }
    if let Some(summary) = &context.run.summary {
        parts.push(summary.clone());
    }
    for item in &context.run.work_items {
        parts.push(item.name.clone());
        parts.push(item.step_type.clone());
        if let Some(details) = &item.details {
            parts.push(details.clone());
        }
        if let Some(error) = &item.error {
            parts.push(error.clone());
        }
    }
    for artifact in &context.artifacts {
        parts.push(artifact.name.clone());
        parts.push(artifact.artifact_type.clone());
        if let Some(text) = &artifact.content_text {
            parts.push(truncate_chars(text, 2000));
        }
        if let Some(json) = &artifact.content_json {
            parts.push(truncate_chars(&json.to_string(), 2000));
        }
    }
    for entry in &context.audit_entries {
        parts.push(entry.raw_input.clone());
        if let Some(message) = &entry.final_message {
            parts.push(message.clone());
        }
        if let Some(error) = &entry.error {
            parts.push(error.clone());
        }
        if let Some(policy_decision) = &entry.policy_decision {
            parts.push(policy_decision.clone());
        }
    }

    parts.join("\n")
}

fn summarize_failure(
    classification: OperatorFailureClassification,
    task: Option<&OpTask>,
    run: &OpTaskRun,
) -> String {
    let task_type = task
        .map(|task| task.task_type.as_str())
        .unwrap_or("unknown task type");
    let run_summary = run
        .summary
        .as_deref()
        .map(|summary| truncate_chars(summary, 240))
        .unwrap_or_else(|| "No run summary was saved.".to_string());

    match classification {
        OperatorFailureClassification::PolicyDenied => format!(
            "{} failed because a policy check denied or required confirmation. {}",
            task_type, run_summary
        ),
        OperatorFailureClassification::BadTaskInput => format!(
            "{} failed because task input or required artifact content was invalid or incomplete. {}",
            task_type, run_summary
        ),
        OperatorFailureClassification::ModelError => format!(
            "{} failed during model execution or parsing. {}",
            task_type, run_summary
        ),
        OperatorFailureClassification::ToolError => format!(
            "{} failed while calling a tool, reader, provider, or external service. {}",
            task_type, run_summary
        ),
        OperatorFailureClassification::BadSearchResults => format!(
            "{} produced weak or irrelevant search results. {}",
            task_type, run_summary
        ),
        OperatorFailureClassification::MissingContext => format!(
            "{} appears to be missing profile, saved context, or source context. {}",
            task_type, run_summary
        ),
        OperatorFailureClassification::CodeBug => format!(
            "{} likely hit an implementation bug or unsupported code path. {}",
            task_type, run_summary
        ),
        OperatorFailureClassification::Unknown => format!(
            "{} failed, but the available evidence did not identify a precise class. {}",
            task_type, run_summary
        ),
    }
}

fn review_context_json(context: &FailedTaskReviewContext) -> Value {
    json!({
        "task": context.task.as_ref().map(task_diagnostic_json),
        "run": run_diagnostic_json(&context.run),
        "work_items": context
            .run
            .work_items
            .iter()
            .map(work_item_diagnostic_json)
            .collect::<Vec<_>>(),
        "artifacts": context
            .artifacts
            .iter()
            .map(artifact_diagnostic_json)
            .collect::<Vec<_>>(),
        "audit": context.audit_entries,
        "include_repo_context": context.input.include_repo_context,
        "repo_context": Value::Null,
    })
}

fn next_actions_for_failure(escalate_if_needed: bool) -> Vec<String> {
    let mut actions = vec![
        "generate_patch_plan".to_string(),
        "create_followup_tasks".to_string(),
    ];
    if escalate_if_needed {
        actions.insert(1, "escalate_to_chatgpt".to_string());
    } else {
        actions.push("escalate_to_chatgpt".to_string());
    }
    actions
}

fn recommendation(
    title: &str,
    priority: OperatorRecommendationPriority,
    task_type: &str,
) -> OperatorRecommendedAction {
    OperatorRecommendedAction {
        title: title.to_string(),
        priority,
        task_type: task_type.to_string(),
    }
}

fn infer_patch_files(diagnostic: &Value, failure_classification: &str) -> Vec<String> {
    let text = diagnostic.to_string().to_lowercase();
    let mut files = Vec::new();

    if text.contains("employment") || text.contains("job") || text.contains("opportunit") {
        files.extend([
            "src/op_tasks/runner.rs",
            "src/services/operator_service.rs",
            "src/routes/openapi.rs",
        ]);
    } else if failure_classification == "tool_error" {
        files.extend(["src/tools/registry.rs", "src/services/execution.rs"]);
    } else if failure_classification == "policy_denied" {
        files.extend(["src/services/policy_engine.rs", "src/routes/openapi.rs"]);
    } else if failure_classification == "model_error" {
        files.extend(["src/op_tasks/runner.rs", "src/services/llm_service.rs"]);
    } else {
        files.extend([
            "src/op_tasks/runner.rs",
            "src/op_tasks/planner.rs",
            "src/routes/openapi.rs",
        ]);
    }

    files.into_iter().map(str::to_string).collect::<Vec<_>>()
}

fn infer_patch_changes(
    files: &[String],
    diagnostic: &Value,
    failure_classification: &str,
) -> Vec<OperatorPatchPlanChange> {
    let summary = diagnostic
        .get("summary")
        .and_then(|value| value.as_str())
        .unwrap_or("Review the failure and adjust the implementation.");

    files
        .iter()
        .map(|file| OperatorPatchPlanChange {
            file: file.clone(),
            change: if file.ends_with("openapi.rs") {
                "Update OpenAPI descriptions/examples for the corrected task behavior.".to_string()
            } else if file.ends_with("operator_service.rs") {
                "Adjust natural-language task intake so task inputs preserve user-specific overrides.".to_string()
            } else if file.ends_with("runner.rs") {
                format!(
                    "Fix task execution behavior for {}. Diagnostic summary: {}",
                    failure_classification, summary
                )
            } else if file.ends_with("planner.rs") {
                "Update planned work items so the task remains inspectable.".to_string()
            } else if file.ends_with("policy_engine.rs") {
                "Review policy tier handling and confirmation requirements.".to_string()
            } else if file.ends_with("execution.rs") {
                "Improve audited execution error normalization and context capture.".to_string()
            } else {
                "Review and adjust this file according to the diagnostic.".to_string()
            },
        })
        .collect()
}

fn implementation_tasks_from_patch_plan(
    profile_id: Uuid,
    patch_plan_artifact_id: Uuid,
    patch_plan: &Value,
) -> Vec<OperatorTaskSpec> {
    let title = patch_plan
        .get("title")
        .and_then(|value| value.as_str())
        .unwrap_or("Implement operator patch plan");
    let mut tasks = vec![OperatorTaskSpec {
        task_type: "code.patch_plan".to_string(),
        name: title.to_string(),
        input_json: json!({
            "profile_id": profile_id,
            "patch_plan_artifact_id": patch_plan_artifact_id,
        }),
    }];

    let files_to_change = patch_plan
        .get("files_to_change")
        .and_then(|value| value.as_array())
        .cloned()
        .unwrap_or_default();
    if files_to_change.iter().any(|value| {
        value
            .as_str()
            .is_some_and(|path| path.contains("README") || path.contains("openapi"))
    }) {
        tasks.push(OperatorTaskSpec {
            task_type: "docs.update_readme_plan".to_string(),
            name: "Document task-oriented behavior changes".to_string(),
            input_json: json!({
                "profile_id": profile_id,
                "patch_plan_artifact_id": patch_plan_artifact_id,
            }),
        });
    }

    tasks
}

fn task_diagnostic_json(task: &OpTask) -> Value {
    json!({
        "id": task.id,
        "profile_id": task.profile_id,
        "task_type": task.task_type,
        "name": task.name,
        "description": task.description,
        "input_json": task.input_json,
        "status": task.status,
        "created_at": task.created_at,
        "updated_at": task.updated_at,
    })
}

fn run_diagnostic_json(run: &OpTaskRun) -> Value {
    json!({
        "id": run.id,
        "profile_id": run.profile_id,
        "task_id": run.task_id,
        "status": run.status,
        "started_at": run.started_at,
        "completed_at": run.completed_at,
        "summary": run.summary,
    })
}

fn work_item_diagnostic_json(item: &crate::op_tasks::models::OpWorkItem) -> Value {
    json!({
        "id": item.id,
        "name": item.name,
        "description": item.description,
        "order": item.order,
        "step_type": item.step_type,
        "model_purpose": item.model_purpose,
        "model_name": item.model_name,
        "tool_name": item.tool_name,
        "tool_args_json": item.tool_args_json,
        "status": item.status,
        "started_at": item.started_at,
        "completed_at": item.completed_at,
        "details": item.details,
        "error": item.error,
        "input_artifact_ids": item.input_artifact_ids,
        "output_artifact_ids": item.output_artifact_ids,
    })
}

fn artifact_diagnostic_json(artifact: &TaskArtifact) -> Value {
    json!({
        "id": artifact.id,
        "run_id": artifact.run_id,
        "work_item_id": artifact.work_item_id,
        "name": artifact.name,
        "artifact_type": artifact.artifact_type,
        "location": artifact.location,
        "created_at": artifact.created_at,
        "metadata": artifact.metadata,
        "content_preview": artifact.content_text
            .as_deref()
            .map(|value| truncate_chars(value, 1200))
            .or_else(|| artifact.content_json.as_ref().map(|value| truncate_chars(&value.to_string(), 1200))),
    })
}

fn render_operator_diagnostic_text(diagnostic: &Value) -> String {
    let classification = diagnostic
        .get("failure_classification")
        .and_then(|value| value.as_str())
        .unwrap_or("unknown");
    let summary = diagnostic
        .get("summary")
        .and_then(|value| value.as_str())
        .unwrap_or("No summary.");
    let actions = diagnostic
        .get("recommended_actions")
        .and_then(|value| value.as_array())
        .map(|items| {
            items
                .iter()
                .enumerate()
                .map(|(index, item)| {
                    format!(
                        "{}. {} ({})",
                        index + 1,
                        item.get("title")
                            .and_then(|value| value.as_str())
                            .unwrap_or("Review failure"),
                        item.get("task_type")
                            .and_then(|value| value.as_str())
                            .unwrap_or("operator.review_failed_task")
                    )
                })
                .collect::<Vec<_>>()
                .join("\n")
        })
        .unwrap_or_default();

    format!(
        "Operator task diagnostic\n\nClassification: {}\n\nSummary: {}\n\nRecommended actions:\n{}",
        classification, summary, actions
    )
}

fn truncate_chars(value: &str, max_chars: usize) -> String {
    let mut output = String::new();
    for (index, ch) in value.chars().enumerate() {
        if index >= max_chars {
            output.push_str("...");
            return output;
        }
        output.push(ch);
    }
    output
}

fn contains_any(text: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| text.contains(needle))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::op_tasks::models::{OpTaskRunStatus, OpWorkItem};

    #[tokio::test]
    async fn classifies_policy_denied_failure() {
        let service = OperatorMetaService::new(
            OpTaskRepository::new(sqlx::SqlitePool::connect_lazy("sqlite::memory:").unwrap()),
            AuditService::new(sqlx::SqlitePool::connect_lazy("sqlite::memory:").unwrap()),
        );
        let run_id = Uuid::new_v4();
        let task_id = Uuid::new_v4();
        let context = FailedTaskReviewContext {
            task: None,
            run: OpTaskRun {
                id: run_id,
                profile_id: Uuid::new_v4(),
                task_id,
                status: OpTaskRunStatus::Failed,
                started_at: None,
                completed_at: None,
                work_items: vec![{
                    let mut item = OpWorkItem::planned(run_id, "policy", "policy", "policy", 1);
                    item.error =
                        Some("policy denied: tier 2 action requires confirmation".to_string());
                    item
                }],
                artifacts: vec![],
                summary: Some("policy denied".to_string()),
            },
            artifacts: vec![],
            audit_entries: vec![],
            input: OperatorReviewFailedTaskInput {
                run_id,
                include_task: true,
                include_artifacts: true,
                include_recent_audit: true,
                include_repo_context: false,
                escalate_if_needed: false,
            },
        };

        assert_eq!(
            service.classify_failure(&context),
            OperatorFailureClassification::PolicyDenied
        );
    }
}
