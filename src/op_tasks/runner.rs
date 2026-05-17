use crate::adapters::openai_escalation::OpenAiEscalationClient;
use crate::context::{models::ContextKind, ContextService};
use crate::domains::employment::{models::EmploymentOpportunity, repository::EmploymentRepository};
use crate::domains::operator::{
    models::{
        OperatorConvertRecommendationToTasksInput, OperatorGeneratePatchPlanInput,
        OperatorReviewFailedTaskInput, OPERATOR_PATCH_PLAN, OPERATOR_TASK_DIAGNOSTIC,
    },
    OperatorMetaService,
};
use crate::op_tasks::models::{
    OpTask, OpTaskRun, OpTaskRunStatus, ReadUrlInput, SearchWebInput, TaskArtifact,
};
use crate::readers::{models::SearchResultItem, ReaderService};
use crate::services::escalation_safety::{redact_request_for_escalation, RedactionReport};
use crate::services::execution::{ExecutionContext, ModelExecutionService, ToolExecutionService};
use crate::services::llm_router::LlmRouter;
use anyhow::{anyhow, Context};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tracing;
use uuid::Uuid;

#[derive(Clone)]
pub struct OpTaskRunner {
    tool_execution: ToolExecutionService,
    model_execution: ModelExecutionService,
    readers: ReaderService,
    llm_router: LlmRouter,
    employment_repo: EmploymentRepository,
    context: ContextService,
    openai_escalation: Option<OpenAiEscalationClient>,
    operator_meta: OperatorMetaService,
}

impl OpTaskRunner {
    pub fn new(
        tool_execution: ToolExecutionService,
        model_execution: ModelExecutionService,
        readers: ReaderService,
        llm_router: LlmRouter,
        employment_repo: EmploymentRepository,
        context: ContextService,
        openai_escalation: Option<OpenAiEscalationClient>,
        operator_meta: OperatorMetaService,
    ) -> Self {
        Self {
            tool_execution,
            model_execution,
            readers,
            llm_router,
            employment_repo,
            context,
            openai_escalation,
            operator_meta,
        }
    }

    pub async fn execute(&self, task: OpTask, mut run: OpTaskRun) -> anyhow::Result<OpTaskRun> {
        run.status = OpTaskRunStatus::Running;
        run.started_at = Some(Utc::now());

        match task.task_type.as_str() {
            "system.status_report" => self.run_status_report(&task, run).await,
            "reader.read_url" => self.run_read_url(&task, run).await,
            "reader.search_web" => self.run_search_web(&task, run).await,
            "employment.search_opportunities" => {
                self.run_employment_search_opportunities(&task, run).await
            }
            "artifact.summarize" => self.run_artifact_summary(&task, run).await,
            "system.escalate_to_chatgpt" | "operator.escalate_to_chatgpt" => {
                self.run_chatgpt_escalation(task, run).await
            }
            "operator.review_failed_task" => self.run_operator_review_failed_task(&task, run).await,
            "operator.generate_patch_plan" => {
                self.run_operator_generate_patch_plan(&task, run).await
            }
            "operator.convert_recommendation_to_tasks" => {
                self.run_operator_convert_recommendation_to_tasks(&task, run)
                    .await
            }
            _ => {
                let message = format!("unsupported task type: {}", task.task_type);
                if let Ok(step_id) = start_step(&mut run, "unsupported_task") {
                    finish_step_with_error(&mut run, step_id, &message);
                }
                run.status = OpTaskRunStatus::Failed;
                run.completed_at = Some(Utc::now());
                run.summary = Some(message);
                Ok(run)
            }
        }
    }

    async fn run_status_report(
        &self,
        _task: &OpTask,
        mut run: OpTaskRun,
    ) -> anyhow::Result<OpTaskRun> {
        let collect_step_id = start_step(&mut run, "collect_system_status")?;
        let result = match self
            .tool_execution
            .execute(
                "system.get_status",
                json!({}),
                false,
                ExecutionContext::for_work_item(_task.id, run.id, collect_step_id)
                    .with_input_summary("Collect system status"),
            )
            .await
        {
            Ok(result) => result,
            Err(err) => {
                finish_step_with_error(&mut run, collect_step_id, &err.to_string());
                return Err(anyhow!(err.to_string()))
                    .context("failed to execute system.get_status tool");
            }
        };
        finish_step(
            &mut run,
            collect_step_id,
            Some(format!("Collected status with {}.", result.tool)),
            vec![],
        );

        let summary_step_id = start_step(&mut run, "summarize_system_status")?;
        let summary_model = self.llm_router.task_summary_model();
        let mut summary = format!("System status collected by {}", result.tool);
        let mut summary_details = format!("Prepared summary with model purpose task_summary.");

        let prompt = format!(
            "Summarize the following system status output in a short, actionable paragraph:\n\n{}",
            serde_json::to_string_pretty(&result.output)
                .unwrap_or_else(|_| "<unserializable output>".to_string())
        );
        let artifact_id = Uuid::new_v4();

        match self
            .model_execution
            .ask_model(
                &summary_model,
                "You are a system status summarization assistant.",
                &prompt,
                ExecutionContext::for_work_item(_task.id, run.id, summary_step_id)
                    .with_model_purpose("task_summary")
                    .with_input_summary("Summarize system status")
                    .with_output_artifact(artifact_id),
            )
            .await
        {
            Ok(summary_text) => summary = summary_text,
            Err(err) => {
                summary = format!(
                    "System status collected, but LLM summarization failed: {}",
                    err
                );
                summary_details = format!("LLM summarization failed; fallback summary used: {err}");
            }
        }

        run.artifacts.push(TaskArtifact {
            id: artifact_id,
            profile_id: run.profile_id,
            run_id: run.id,
            work_item_id: Some(summary_step_id),
            name: "system_status_report".to_string(),
            artifact_type: "status_report".to_string(),
            location: None,
            created_at: Utc::now(),
            metadata: Some(json!({
                "tool": result.tool,
                "model": summary_model,
                "model_purpose": "task_summary",
                "output": result.output,
            })),
            content_text: Some(summary.clone()),
            content_json: None,
        });
        finish_step(
            &mut run,
            summary_step_id,
            Some(summary_details),
            vec![artifact_id],
        );

        run.summary = Some(summary);
        run.status = OpTaskRunStatus::Succeeded;
        run.completed_at = Some(Utc::now());
        Ok(run)
    }

    async fn run_chatgpt_escalation(
        &self,
        task: OpTask,
        mut run: OpTaskRun,
    ) -> anyhow::Result<OpTaskRun> {
        let collect_step_id = start_step(&mut run, "collect_escalation_context")?;
        let input: ChatGptEscalationInput = match serde_json::from_value(task.input_json.clone()) {
            Ok(input) => input,
            Err(err) => {
                finish_step_with_error(&mut run, collect_step_id, &err.to_string());
                return Err(anyhow!(err)).context("invalid system.escalate_to_chatgpt input_json");
            }
        };

        let user_request = clean_optional_text(input.user_request.as_deref())
            .unwrap_or_else(|| "Escalate this Local Operator task to ChatGPT.".to_string());
        let mode = clean_optional_text(input.mode.as_deref()).unwrap_or_else(|| "manual".into());
        if !matches!(mode.as_str(), "manual" | "openai") {
            let err = format!(
                "unsupported ChatGPT escalation mode '{}'; supported modes are manual and openai",
                mode
            );
            finish_step_with_error(&mut run, collect_step_id, &err);
            return Err(anyhow!(err));
        }

        let context_query = clean_optional_text(input.context_query.as_deref())
            .unwrap_or_else(|| user_request.clone());
        let saved_context = match self
            .context
            .get_relevant_context(run.profile_id, &context_query, None)
            .await
        {
            Ok(items) => items
                .into_iter()
                .take(input.context_limit.unwrap_or(8).clamp(1, 25))
                .map(|item| {
                    json!({
                        "id": item.id,
                        "kind": item.kind,
                        "title": item.title,
                        "body": item.body,
                        "source_url": item.source_url,
                        "source_artifact_id": item.source_artifact_id,
                        "tags": item.tags,
                    })
                })
                .collect::<Vec<_>>(),
            Err(err) => {
                finish_step_with_error(&mut run, collect_step_id, &err.to_string());
                return Err(err).context("failed to collect escalation context");
            }
        };

        let collected_context = json!({
            "task": {
                "id": task.id,
                "task_type": task.task_type,
                "name": task.name,
                "description": task.description,
                "input_json": task.input_json,
            },
            "run": {
                "id": run.id,
                "profile_id": run.profile_id,
            },
            "user_request": user_request,
            "escalation_mode": mode,
            "desired_output": input.desired_output,
            "saved_context": saved_context,
            "supplied_context_text": input.context_text,
            "supplied_context_json": input.context_json,
        });
        finish_step(
            &mut run,
            collect_step_id,
            Some("Collected task input, user request, and saved profile context.".to_string()),
            vec![],
        );

        let redact_step_id = start_step(&mut run, "redact_escalation_context")?;
        let prepared = redact_request_for_escalation(None, &collected_context, input.confirm);
        let redaction_report = prepared.redaction_report;
        let redacted_context = prepared.redacted_json;
        let privacy_classification = prepared.privacy_classification;
        let policy_decision = prepared.policy_decision;
        if !policy_decision.allowed {
            finish_step_with_error(&mut run, redact_step_id, &policy_decision.reason);
            return Err(anyhow!(policy_decision.reason.clone()));
        }
        finish_step(
            &mut run,
            redact_step_id,
            Some(format!(
                "Privacy classification {:?}. Redacted {} sensitive keys and {} sensitive text values.",
                privacy_classification,
                redaction_report.redacted_keys,
                redaction_report.redacted_text_values
            )),
            vec![],
        );

        let save_step_id = start_step(&mut run, "save_escalation_request")?;
        let artifact_id = Uuid::new_v4();
        let paste_prompt = render_chatgpt_escalation_prompt(
            &user_request,
            input.desired_output.as_deref(),
            &redacted_context,
            &redaction_report,
        );

        run.artifacts.push(TaskArtifact {
            id: artifact_id,
            profile_id: run.profile_id,
            run_id: run.id,
            work_item_id: Some(save_step_id),
            name: "ChatGPT escalation request".to_string(),
            artifact_type: "chatgpt_escalation_request".to_string(),
            location: None,
            created_at: Utc::now(),
            metadata: Some(json!({
                "escalation_provider": "chatgpt",
                "mode": mode,
                "direction": "request",
                "redaction": redaction_report,
                "privacy_classification": privacy_classification,
                "policy_decision": policy_decision,
                "requires_confirmation": policy_decision.requires_confirmation,
                "confirmed": input.confirm,
                "task_id": task.id,
                "openai_request_sends_only_redacted_content": mode == "openai",
            })),
            content_text: Some(paste_prompt.clone()),
            content_json: Some(json!({
                "mode": mode,
                "provider": "chatgpt",
                "user_request": user_request,
                "desired_output": input.desired_output,
                "redacted_context": redacted_context,
                "redaction_report": redaction_report,
                "privacy_classification": privacy_classification,
                "policy_decision": policy_decision,
                "actions_executed_by_local_operator": [],
                "instructions": [
                    if mode == "manual" {
                        "Paste content_text into ChatGPT manually."
                    } else {
                        "OpenAI API provider sends only this redacted request content."
                    },
                    "Do not execute recommended actions automatically.",
                    "Paste or inspect ChatGPT's answer through the saved response artifact."
                ],
            })),
        });

        let mut output_artifact_ids = vec![artifact_id];
        let mut response_artifact_id = None;
        if mode == "openai" {
            let redacted_request = run
                .artifacts
                .iter()
                .find(|artifact| artifact.id == artifact_id)
                .and_then(|artifact| artifact.content_json.clone())
                .ok_or_else(|| anyhow!("escalation request artifact has no JSON content"))?;
            let openai_result = match self.openai_escalation.as_ref() {
                Some(client) => client.send_redacted_request(&redacted_request).await,
                None => Err(crate::error::AppError::Internal(
                    "OpenAI escalation provider is not enabled in configuration".to_string(),
                )),
            };
            match openai_result {
                Ok(output) => {
                    let id = Uuid::new_v4();
                    run.artifacts.push(TaskArtifact {
                        id,
                        profile_id: run.profile_id,
                        run_id: run.id,
                        work_item_id: Some(save_step_id),
                        name: "ChatGPT escalation response".to_string(),
                        artifact_type: "chatgpt_escalation_response".to_string(),
                        location: None,
                        created_at: Utc::now(),
                        metadata: Some(json!({
                            "escalation_provider": "openai",
                            "direction": "response",
                            "request_artifact_id": artifact_id,
                            "actions_executed_by_local_operator": [],
                            "recommended_actions_are_advisory_only": true,
                        })),
                        content_text: Some(output.output_text.clone()),
                        content_json: Some(json!({
                            "raw_response": output.raw_response,
                            "parsed_response": output.parsed_response,
                            "request_artifact_id": artifact_id,
                            "actions_executed_by_local_operator": [],
                            "recommended_actions_are_advisory_only": true,
                        })),
                    });
                    output_artifact_ids.push(id);
                    response_artifact_id = Some(id);
                }
                Err(err) => {
                    let id = Uuid::new_v4();
                    run.artifacts.push(TaskArtifact {
                        id,
                        profile_id: run.profile_id,
                        run_id: run.id,
                        work_item_id: Some(save_step_id),
                        name: "ChatGPT escalation response error".to_string(),
                        artifact_type: "chatgpt_escalation_response".to_string(),
                        location: None,
                        created_at: Utc::now(),
                        metadata: Some(json!({
                            "escalation_provider": "openai",
                            "direction": "response",
                            "request_artifact_id": artifact_id,
                            "success": false,
                            "actions_executed_by_local_operator": [],
                        })),
                        content_text: Some(format!("OpenAI escalation failed: {}", err)),
                        content_json: Some(json!({
                            "error": err.to_string(),
                            "request_artifact_id": artifact_id,
                            "actions_executed_by_local_operator": [],
                            "recommended_actions_are_advisory_only": true,
                        })),
                    });
                    output_artifact_ids.push(id);
                    response_artifact_id = Some(id);
                }
            }
        }

        finish_step(
            &mut run,
            save_step_id,
            Some(if mode == "openai" {
                "Saved ChatGPT escalation request and OpenAI response artifacts.".to_string()
            } else {
                "Saved manual ChatGPT escalation request artifact.".to_string()
            }),
            output_artifact_ids,
        );

        run.status = OpTaskRunStatus::Succeeded;
        run.completed_at = Some(Utc::now());
        run.summary = Some(if let Some(response_artifact_id) = response_artifact_id {
            format!(
                "OpenAI ChatGPT escalation completed. Request artifact {} and response artifact {} were saved. No recommended actions were executed automatically.",
                artifact_id, response_artifact_id
            )
        } else {
            format!(
                "Manual ChatGPT escalation request prepared. Paste artifact {} into ChatGPT, then save the response back to the request artifact.",
                artifact_id
            )
        });

        Ok(run)
    }

    async fn run_operator_review_failed_task(
        &self,
        task: &OpTask,
        mut run: OpTaskRun,
    ) -> anyhow::Result<OpTaskRun> {
        let load_run_step_id = start_step(&mut run, "load_failed_run")?;
        let input: OperatorReviewFailedTaskInput =
            match serde_json::from_value(task.input_json.clone()) {
                Ok(input) => input,
                Err(err) => {
                    finish_step_with_error(&mut run, load_run_step_id, &err.to_string());
                    return Err(anyhow!(err))
                        .context("invalid operator.review_failed_task input_json");
                }
            };

        let reviewed_run = match self
            .operator_meta
            .load_failed_run(run.profile_id, input.run_id)
            .await
        {
            Ok(reviewed_run) => reviewed_run,
            Err(err) => {
                finish_step_with_error(&mut run, load_run_step_id, &err.to_string());
                return Err(err).context("failed to load failed run");
            }
        };
        finish_step(
            &mut run,
            load_run_step_id,
            Some(format!("Loaded failed run {}.", reviewed_run.id)),
            vec![],
        );

        let load_task_step_id = start_step(&mut run, "load_task_definition")?;
        let reviewed_task = match self
            .operator_meta
            .load_task_definition(&reviewed_run, input.include_task)
            .await
        {
            Ok(task) => task,
            Err(err) => {
                finish_step_with_error(&mut run, load_task_step_id, &err.to_string());
                return Err(err).context("failed to load reviewed task");
            }
        };
        finish_step(
            &mut run,
            load_task_step_id,
            Some(if reviewed_task.is_some() {
                "Loaded task definition.".to_string()
            } else {
                "Task definition omitted by input.".to_string()
            }),
            vec![],
        );

        let load_artifacts_step_id = start_step(&mut run, "load_run_artifacts")?;
        let reviewed_artifacts = self
            .operator_meta
            .load_artifacts(&reviewed_run, input.include_artifacts);
        for artifact in &reviewed_artifacts {
            push_step_input_artifact(&mut run, load_artifacts_step_id, artifact.id);
        }
        finish_step(
            &mut run,
            load_artifacts_step_id,
            Some(format!(
                "Loaded {} artifact(s) from failed run.",
                reviewed_artifacts.len()
            )),
            vec![],
        );

        let audit_step_id = start_step(&mut run, "load_recent_audit")?;
        let audit_entries = match self
            .operator_meta
            .load_recent_audit(reviewed_run.id, input.include_recent_audit)
            .await
        {
            Ok(entries) => entries,
            Err(err) => {
                finish_step_with_error(&mut run, audit_step_id, &err.to_string());
                return Err(err).context("failed to load audit entries");
            }
        };
        finish_step(
            &mut run,
            audit_step_id,
            Some(format!(
                "Loaded {} audit entr{}.",
                audit_entries.len(),
                if audit_entries.len() == 1 { "y" } else { "ies" }
            )),
            vec![],
        );

        let classify_step_id = start_step(&mut run, "classify_failure")?;
        let review_context = self.operator_meta.build_review_context(
            input.clone(),
            reviewed_run,
            reviewed_task,
            reviewed_artifacts,
            audit_entries,
        );
        let diagnostic = self.operator_meta.build_diagnostic_packet(&review_context);
        finish_step(
            &mut run,
            classify_step_id,
            Some(format!(
                "Classified failed run as {}.",
                diagnostic.failure_classification.as_str()
            )),
            vec![],
        );

        let analyze_step_id = start_step(&mut run, "analyze_root_cause")?;
        finish_step(
            &mut run,
            analyze_step_id,
            Some(format!(
                "Analyzed likely root cause and recommendation evidence for {}.",
                diagnostic.failure_classification.as_str()
            )),
            vec![],
        );

        let save_step_id = start_step(&mut run, "save_diagnostic_artifact")?;
        let diagnostic_artifact = self.operator_meta.diagnostic_artifact(&diagnostic)?;
        let artifact_id = Uuid::new_v4();
        run.artifacts.push(TaskArtifact {
            id: artifact_id,
            profile_id: run.profile_id,
            run_id: run.id,
            work_item_id: Some(save_step_id),
            name: diagnostic_artifact.name,
            artifact_type: diagnostic_artifact.artifact_type,
            location: None,
            created_at: Utc::now(),
            metadata: Some(diagnostic_artifact.metadata),
            content_text: Some(diagnostic_artifact.content_text),
            content_json: Some(diagnostic_artifact.content_json),
        });
        finish_step(
            &mut run,
            save_step_id,
            Some("Saved operator_task_diagnostic artifact.".to_string()),
            vec![artifact_id],
        );

        let summary_step_id = start_step(&mut run, "summarize_operator_review")?;
        finish_step(
            &mut run,
            summary_step_id,
            Some(format!(
                "Prepared final summary for failed run {}.",
                diagnostic.source.run_id
            )),
            vec![],
        );

        run.status = OpTaskRunStatus::Succeeded;
        run.completed_at = Some(Utc::now());
        run.summary = Some(format!(
            "Reviewed failed run {} and saved diagnostic artifact {}.",
            diagnostic.source.run_id, artifact_id
        ));

        Ok(run)
    }

    async fn run_operator_generate_patch_plan(
        &self,
        task: &OpTask,
        mut run: OpTaskRun,
    ) -> anyhow::Result<OpTaskRun> {
        let load_step_id = start_step(&mut run, "load_diagnostic_artifact")?;
        let input: OperatorGeneratePatchPlanInput =
            match serde_json::from_value(task.input_json.clone()) {
                Ok(input) => input,
                Err(err) => {
                    finish_step_with_error(&mut run, load_step_id, &err.to_string());
                    return Err(anyhow!(err))
                        .context("invalid operator.generate_patch_plan input_json");
                }
            };

        let diagnostic_artifact = match self
            .operator_meta
            .load_artifact(run.profile_id, input.artifact_id, OPERATOR_TASK_DIAGNOSTIC)
            .await
        {
            Ok(artifact) => artifact,
            Err(err) => {
                finish_step_with_error(&mut run, load_step_id, &err.to_string());
                return Err(err).context("failed to load diagnostic artifact");
            }
        };
        push_step_input_artifact(&mut run, load_step_id, diagnostic_artifact.id);
        finish_step(
            &mut run,
            load_step_id,
            Some(format!(
                "Loaded diagnostic artifact {}.",
                diagnostic_artifact.id
            )),
            vec![],
        );

        let build_step_id = start_step(&mut run, "build_patch_plan")?;
        let patch_plan_artifact = match self.operator_meta.patch_plan_artifact(
            &diagnostic_artifact,
            input
                .title
                .unwrap_or_else(|| "Operator patch plan".to_string()),
        ) {
            Ok(artifact) => artifact,
            Err(err) => {
                finish_step_with_error(&mut run, build_step_id, &err.to_string());
                return Err(err).context("failed to build patch plan");
            }
        };
        finish_step(
            &mut run,
            build_step_id,
            Some("Built read-only operator patch plan.".to_string()),
            vec![],
        );

        let save_step_id = start_step(&mut run, "save_patch_plan_artifact")?;
        let artifact_id = Uuid::new_v4();
        run.artifacts.push(TaskArtifact {
            id: artifact_id,
            profile_id: run.profile_id,
            run_id: run.id,
            work_item_id: Some(save_step_id),
            name: patch_plan_artifact.name,
            artifact_type: patch_plan_artifact.artifact_type,
            location: None,
            created_at: Utc::now(),
            metadata: Some(patch_plan_artifact.metadata),
            content_text: Some(patch_plan_artifact.content_text),
            content_json: Some(patch_plan_artifact.content_json),
        });
        finish_step(
            &mut run,
            save_step_id,
            Some("Saved operator_patch_plan artifact.".to_string()),
            vec![artifact_id],
        );

        let summary_step_id = start_step(&mut run, "summarize_patch_plan")?;
        finish_step(
            &mut run,
            summary_step_id,
            Some(format!(
                "Prepared final summary for patch plan artifact {}.",
                artifact_id
            )),
            vec![],
        );

        run.status = OpTaskRunStatus::Succeeded;
        run.completed_at = Some(Utc::now());
        run.summary = Some(format!(
            "Generated patch plan artifact {} from diagnostic artifact {}.",
            artifact_id, diagnostic_artifact.id
        ));

        Ok(run)
    }

    async fn run_operator_convert_recommendation_to_tasks(
        &self,
        task: &OpTask,
        mut run: OpTaskRun,
    ) -> anyhow::Result<OpTaskRun> {
        let load_step_id = start_step(&mut run, "load_patch_plan_artifact")?;
        let input: OperatorConvertRecommendationToTasksInput =
            match serde_json::from_value(task.input_json.clone()) {
                Ok(input) => input,
                Err(err) => {
                    finish_step_with_error(&mut run, load_step_id, &err.to_string());
                    return Err(anyhow!(err))
                        .context("invalid operator.convert_recommendation_to_tasks input_json");
                }
            };

        let patch_plan_artifact = match self
            .operator_meta
            .load_artifact(run.profile_id, input.artifact_id, OPERATOR_PATCH_PLAN)
            .await
        {
            Ok(artifact) => artifact,
            Err(err) => {
                finish_step_with_error(&mut run, load_step_id, &err.to_string());
                return Err(err).context("failed to load patch plan artifact");
            }
        };
        push_step_input_artifact(&mut run, load_step_id, patch_plan_artifact.id);
        finish_step(
            &mut run,
            load_step_id,
            Some(format!(
                "Loaded patch plan artifact {}.",
                patch_plan_artifact.id
            )),
            vec![],
        );

        let build_step_id = start_step(&mut run, "build_implementation_task_set")?;
        let task_set_artifact = match self
            .operator_meta
            .implementation_task_set_artifact(run.profile_id, &patch_plan_artifact)
        {
            Ok(artifact) => artifact,
            Err(err) => {
                finish_step_with_error(&mut run, build_step_id, &err.to_string());
                return Err(err).context("failed to build implementation task set");
            }
        };
        finish_step(
            &mut run,
            build_step_id,
            Some("Built read-only implementation task set.".to_string()),
            vec![],
        );

        let save_step_id = start_step(&mut run, "save_implementation_task_set_artifact")?;
        let artifact_id = Uuid::new_v4();
        run.artifacts.push(TaskArtifact {
            id: artifact_id,
            profile_id: run.profile_id,
            run_id: run.id,
            work_item_id: Some(save_step_id),
            name: task_set_artifact.name,
            artifact_type: task_set_artifact.artifact_type,
            location: None,
            created_at: Utc::now(),
            metadata: Some(task_set_artifact.metadata),
            content_text: Some(task_set_artifact.content_text),
            content_json: Some(task_set_artifact.content_json),
        });
        finish_step(
            &mut run,
            save_step_id,
            Some("Saved operator_implementation_task_set artifact.".to_string()),
            vec![artifact_id],
        );

        let summary_step_id = start_step(&mut run, "summarize_implementation_task_set")?;
        finish_step(
            &mut run,
            summary_step_id,
            Some(format!(
                "Prepared final summary for implementation task set artifact {}.",
                artifact_id
            )),
            vec![],
        );

        run.status = OpTaskRunStatus::Succeeded;
        run.completed_at = Some(Utc::now());
        run.summary = Some(format!(
            "Generated implementation task set artifact {} from patch plan artifact {}. No OpTasks were created.",
            artifact_id, patch_plan_artifact.id
        ));

        Ok(run)
    }

    async fn run_artifact_summary(
        &self,
        task: &OpTask,
        mut run: OpTaskRun,
    ) -> anyhow::Result<OpTaskRun> {
        let step_id = start_step(&mut run, "summarize_artifact")?;
        let input: ArtifactSummaryInput = match serde_json::from_value(task.input_json.clone()) {
            Ok(input) => input,
            Err(err) => {
                finish_step_with_error(&mut run, step_id, &err.to_string());
                return Err(anyhow!(err)).context("invalid artifact.summarize input_json");
            }
        };
        if let Some(source_artifact_id) = input.source_artifact_id {
            push_step_input_artifact(&mut run, step_id, source_artifact_id);
        }

        let content = input
            .content_text
            .clone()
            .or_else(|| {
                input
                    .content_json
                    .as_ref()
                    .and_then(|value| serde_json::to_string_pretty(value).ok())
            })
            .unwrap_or_else(|| "Artifact has no saved content.".to_string());
        let prompt = format!(
            "User continuation request: {}\n\nArtifact name: {}\nArtifact type: {}\n\nArtifact content:\n{}",
            input.user_request,
            input.artifact_name,
            input.artifact_type,
            truncate_chars(&content, 12000)
        );
        let system = "You are Local Operator. Continue from the supplied artifact. Be concise, concrete, and preserve useful IDs, URLs, scores, and next steps.";
        let summary = self
            .model_execution
            .ask_model(
                &self.llm_router.task_summary_model(),
                system,
                &prompt,
                ExecutionContext::for_work_item(task.id, run.id, step_id)
                    .with_model_purpose("artifact_continuation")
                    .with_input_summary(format!(
                        "Continue from {} artifact {}",
                        input.artifact_type,
                        input
                            .source_artifact_id
                            .map(|id| id.to_string())
                            .unwrap_or_else(|| "unknown".to_string())
                    )),
            )
            .await
            .unwrap_or_else(|_| {
                format!(
                    "Continuation from artifact '{}':\n\n{}",
                    input.artifact_name,
                    truncate_chars(&content, 3000)
                )
            });

        let artifact_id = Uuid::new_v4();
        run.artifacts.push(TaskArtifact {
            id: artifact_id,
            profile_id: run.profile_id,
            run_id: run.id,
            work_item_id: Some(step_id),
            name: format!("Continuation summary for {}", input.artifact_name),
            artifact_type: "artifact_continuation_summary".to_string(),
            location: None,
            created_at: Utc::now(),
            metadata: Some(json!({
                "source_artifact_id": input.source_artifact_id.map(|id| id.to_string()),
                "source_artifact_type": input.artifact_type,
                "user_request": input.user_request,
            })),
            content_text: Some(summary.clone()),
            content_json: Some(json!({
                "summary": summary,
            })),
        });
        finish_step(
            &mut run,
            step_id,
            Some("Created artifact continuation summary.".to_string()),
            vec![artifact_id],
        );
        run.status = OpTaskRunStatus::Succeeded;
        run.completed_at = Some(Utc::now());
        run.summary = Some("Artifact continuation summary completed.".to_string());

        Ok(run)
    }

    async fn run_read_url(&self, task: &OpTask, mut run: OpTaskRun) -> anyhow::Result<OpTaskRun> {
        let step_id = start_step(&mut run, "read_url")?;
        let input: ReadUrlInput = match serde_json::from_value(task.input_json.clone()) {
            Ok(input) => input,
            Err(err) => {
                finish_step_with_error(&mut run, step_id, &err.to_string());
                return Err(anyhow!(err)).context("invalid reader.read_url input_json");
            }
        };

        let result = match self.readers.read_url(input.url.clone()).await {
            Ok(result) => result,
            Err(err) => {
                finish_step_with_error(&mut run, step_id, &err.to_string());
                return Err(err).context("failed to read URL");
            }
        };

        let title = result
            .title
            .clone()
            .unwrap_or_else(|| "read_url_result".to_string());
        let cleaned_text = result.cleaned_text.clone();
        let artifact_id = Uuid::new_v4();

        run.artifacts.push(TaskArtifact {
            id: artifact_id,
            profile_id: run.profile_id,
            run_id: run.id,
            work_item_id: Some(step_id),
            name: title,
            artifact_type: "readable_web_page".to_string(),
            location: Some(input.url.clone()),
            created_at: Utc::now(),
            metadata: Some(json!({
                "source_url": input.url.clone(),
                "title": result.title.clone(),
                "detected_type": result.detected_type.clone(),
                "text_length": result.cleaned_text.len()
            })),
            content_text: Some(cleaned_text.clone()),
            content_json: Some(json!({
                "raw_text": result.raw_text,
                "cleaned_text": cleaned_text,
                "title": result.title,
                "source_url": input.url
            })),
        });

        finish_step(
            &mut run,
            step_id,
            Some("Read URL and extracted readable text.".to_string()),
            vec![artifact_id],
        );
        run.status = OpTaskRunStatus::Succeeded;
        run.completed_at = Some(Utc::now());
        run.summary = Some("Read URL and extracted readable text.".to_string());

        Ok(run)
    }

    async fn run_search_web(&self, task: &OpTask, mut run: OpTaskRun) -> anyhow::Result<OpTaskRun> {
        let step_id = start_step(&mut run, "search_web")?;
        let input: SearchWebInput = match serde_json::from_value(task.input_json.clone()) {
            Ok(input) => input,
            Err(err) => {
                finish_step_with_error(&mut run, step_id, &err.to_string());
                return Err(anyhow!(err)).context("invalid reader.search_web input_json");
            }
        };
        let limit = input.limit.unwrap_or(10).clamp(1, 25);
        update_step_tool_args(
            &mut run,
            step_id,
            json!({
                "query": input.query,
                "limit": limit,
            }),
        );

        let results = match self.readers.search_web(input.query.clone(), limit).await {
            Ok(results) => results,
            Err(err) => {
                finish_step_with_error(&mut run, step_id, &err.to_string());
                return Err(err).context("failed to search web");
            }
        };

        let content_text = render_search_results_text(&results.query, &results.results);
        let result_count = results.results.len();
        let query = results.query.clone();
        let artifact_id = Uuid::new_v4();

        run.artifacts.push(TaskArtifact {
            id: artifact_id,
            profile_id: run.profile_id,
            run_id: run.id,
            work_item_id: Some(step_id),
            name: format!("Search results: {}", query),
            artifact_type: "search_result_set".to_string(),
            location: None,
            created_at: Utc::now(),
            metadata: Some(json!({
                "query": query,
                "profile_id": run.profile_id.to_string(),
                "result_count": result_count,
                "model_purpose": task.input_json.get("model_purpose").cloned(),
                "priority": task.input_json.get("priority").cloned(),
            })),
            content_text: Some(content_text),
            content_json: Some(json!({
                "query": results.query,
                "results": results.results,
            })),
        });

        finish_step(
            &mut run,
            step_id,
            Some(format!("Search completed with {} results.", result_count)),
            vec![artifact_id],
        );
        run.status = OpTaskRunStatus::Succeeded;
        run.completed_at = Some(Utc::now());
        run.summary = Some(format!(
            "Search completed for '{}' with {} results.",
            input.query, result_count
        ));

        Ok(run)
    }

    async fn run_employment_search_opportunities(
        &self,
        task: &OpTask,
        mut run: OpTaskRun,
    ) -> anyhow::Result<OpTaskRun> {
        let profile_step_id = start_step(&mut run, "load_profile_context")?;
        let input: EmploymentSearchInput = match serde_json::from_value(task.input_json.clone()) {
            Ok(input) => input,
            Err(err) => {
                finish_step_with_error(&mut run, profile_step_id, &err.to_string());
                return Err(anyhow!(err))
                    .context("invalid employment.search_opportunities input_json");
            }
        };
        let limit = input.limit.unwrap_or(10).clamp(1, 25);
        let read_limit = limit.min(8);
        let min_score = input.min_score.unwrap_or(0).clamp(0, 100);

        let profile = match self.employment_repo.get_profile(run.profile_id).await {
            Ok(Some(profile)) => profile,
            Ok(None) => {
                let err = "employment profile not found";
                finish_step_with_error(&mut run, profile_step_id, err);
                return Err(anyhow!(err));
            }
            Err(err) => {
                finish_step_with_error(&mut run, profile_step_id, &err.to_string());
                return Err(err).context("failed to load employment profile");
            }
        };
        let context_summary = match self.load_employment_context_summary(run.profile_id).await {
            Ok(context_summary) => context_summary,
            Err(err) => {
                finish_step_with_error(&mut run, profile_step_id, &err.to_string());
                return Err(err).context("failed to load employment context");
            }
        };

        let criteria = profile
            .criteria
            .clone()
            .filter(|c| !c.trim().is_empty())
            .unwrap_or_else(|| "jobs".to_string());
        finish_step(
            &mut run,
            profile_step_id,
            Some(format!(
                "Loaded profile {} and {} context notes.",
                profile.display_name,
                context_summary.len()
            )),
            vec![],
        );

        let build_query_step_id = start_step(&mut run, "build_search_query")?;
        let search_query = build_employment_search_query(&input, &criteria);
        update_step_tool_args(
            &mut run,
            build_query_step_id,
            json!({
                "query": search_query,
                "limit": limit,
                "query_override": input.query_override,
                "user_request": input.user_request,
                "location": input.location,
                "remote_preference": input.remote_preference,
                "min_score": min_score,
            }),
        );
        finish_step(
            &mut run,
            build_query_step_id,
            Some(format!("Built search query '{}'.", search_query)),
            vec![],
        );

        let search_step_id = start_step(&mut run, "run_search")?;
        update_step_tool_args(
            &mut run,
            search_step_id,
            json!({
                "query": search_query,
                "limit": limit,
                "source_artifact_id": input.source_artifact_id,
                "source_artifact_type": input.source_artifact_type,
            }),
        );
        if let Some(source_artifact_id) = input.source_artifact_id {
            push_step_input_artifact(&mut run, search_step_id, source_artifact_id);
        }
        let (results_query, search_results) = if input.seed_search_results.is_empty() {
            match self.readers.search_web(search_query.clone(), limit).await {
                Ok(results) => (results.query, results.results),
                Err(err) => {
                    finish_step_with_error(&mut run, search_step_id, &err.to_string());
                    return Err(err).context("failed to search web for opportunities");
                }
            }
        } else {
            (
                input
                    .source_query
                    .clone()
                    .unwrap_or_else(|| search_query.clone()),
                input
                    .seed_search_results
                    .clone()
                    .into_iter()
                    .take(limit)
                    .collect(),
            )
        };

        let content_text = if search_results.is_empty() {
            "No job opportunities found for profile criteria.".to_string()
        } else {
            render_search_results_text(&results_query, &search_results)
        };

        let result_count = search_results.len();
        let artifact_id = Uuid::new_v4();

        run.artifacts.push(TaskArtifact {
            id: artifact_id,
            profile_id: run.profile_id,
            run_id: run.id,
            work_item_id: Some(search_step_id),
            name: "Employment opportunities search results".to_string(),
            artifact_type: "search_result_set".to_string(),
            location: None,
            created_at: Utc::now(),
            metadata: Some(json!({
                "criteria": criteria,
                "context": context_summary,
                "query": search_query,
                "profile_id": run.profile_id.to_string(),
                "result_count": result_count,
                "search_type": "employment_opportunities",
                "create_opportunities": input.create_opportunities,
                "min_score": min_score,
                "profile_display_name": profile.display_name,
                "source_artifact_id": input.source_artifact_id.map(|id| id.to_string()),
                "source_artifact_type": input.source_artifact_type,
            })),
            content_text: Some(content_text),
            content_json: Some(json!({
                "criteria": profile.criteria,
                "query": results_query,
                "results": search_results,
                "profile_id": run.profile_id.to_string(),
            })),
        });
        finish_step(
            &mut run,
            search_step_id,
            Some(format!("Search completed with {} results.", result_count)),
            vec![artifact_id],
        );

        let read_step_id = start_step(&mut run, "read_result_urls")?;
        push_step_input_artifact(&mut run, read_step_id, artifact_id);
        if let Some(source_artifact_id) = input.source_artifact_id {
            push_step_input_artifact(&mut run, read_step_id, source_artifact_id);
        }
        let mut readable_pages = Vec::new();
        let mut read_failures = Vec::new();

        for seed_page in input.seed_readable_pages.iter().take(read_limit) {
            readable_pages.push(ReadablePageForExtraction {
                artifact_id: seed_page.artifact_id.unwrap_or(artifact_id),
                source_url: seed_page.source_url.clone(),
                title: seed_page.title.clone(),
                snippet: seed_page.snippet.clone(),
                text: seed_page.text.clone(),
            });
        }

        let remaining_read_limit = read_limit.saturating_sub(readable_pages.len());
        for result in search_results.iter().take(remaining_read_limit) {
            if readable_pages
                .iter()
                .any(|page| page.source_url == result.url)
            {
                continue;
            }
            match self.readers.read_url(result.url.clone()).await {
                Ok(page) => {
                    let page_artifact_id = Uuid::new_v4();
                    let page_title = page.title.clone().unwrap_or_else(|| result.title.clone());
                    run.artifacts.push(TaskArtifact {
                        id: page_artifact_id,
                        profile_id: run.profile_id,
                        run_id: run.id,
                        work_item_id: Some(read_step_id),
                        name: page_title,
                        artifact_type: "readable_web_page".to_string(),
                        location: Some(page.source_url.clone()),
                        created_at: Utc::now(),
                        metadata: Some(json!({
                            "source_url": page.source_url,
                            "title": page.title,
                            "detected_type": page.detected_type,
                            "text_length": page.cleaned_text.len(),
                            "search_result_title": result.title,
                        })),
                        content_text: Some(page.cleaned_text.clone()),
                        content_json: Some(json!({
                            "raw_text": page.raw_text,
                            "cleaned_text": page.cleaned_text,
                            "title": page.title,
                            "source_url": page.source_url,
                            "search_result": result,
                        })),
                    });
                    readable_pages.push(ReadablePageForExtraction {
                        artifact_id: page_artifact_id,
                        source_url: result.url.clone(),
                        title: result.title.clone(),
                        snippet: result.snippet.clone(),
                        text: page.cleaned_text,
                    });
                }
                Err(err) => {
                    read_failures.push(json!({
                        "url": result.url,
                        "title": result.title,
                        "error": err.to_string(),
                    }));
                }
            }
        }
        let readable_artifact_ids = readable_pages
            .iter()
            .map(|page| page.artifact_id)
            .collect::<Vec<_>>();
        finish_step(
            &mut run,
            read_step_id,
            Some(format!(
                "Read {} result URLs; {} failed.",
                readable_pages.len(),
                read_failures.len()
            )),
            readable_artifact_ids.clone(),
        );

        let extract_step_id = start_step(&mut run, "extract_candidates")?;
        push_step_input_artifact(&mut run, extract_step_id, artifact_id);
        for readable_artifact_id in &readable_artifact_ids {
            push_step_input_artifact(&mut run, extract_step_id, *readable_artifact_id);
        }
        let candidates_artifact_id = Uuid::new_v4();
        let mut candidates = input
            .seed_candidates
            .clone()
            .into_iter()
            .take(limit)
            .collect::<Vec<_>>();

        for page in &readable_pages {
            if candidates
                .iter()
                .any(|candidate| candidate.source_url == page.source_url)
            {
                continue;
            }
            let parsed = self
                .model_execution
                .parse_job_opportunity(
                    &self.llm_router.task_extraction_model(),
                    &page.text,
                    ExecutionContext::for_work_item(task.id, run.id, extract_step_id)
                        .with_model_purpose("employment_candidate_extract")
                        .with_input_summary(format!("Extract candidate from {}", page.source_url))
                        .with_output_artifact(candidates_artifact_id),
                )
                .await
                .unwrap_or_else(|_| {
                    json!({
                        "title": page.title,
                        "company": null,
                        "location": null,
                        "remote_type": null,
                        "salary_min": null,
                        "salary_max": null,
                        "description_text": page.text.chars().take(4000).collect::<String>(),
                    })
                });
            candidates.push(candidate_from_parsed(
                &parsed,
                &page.source_url,
                Some(page.title.clone()),
                page.snippet.clone(),
                Some(page.artifact_id),
            ));
        }

        for result in &search_results {
            if !candidates
                .iter()
                .any(|candidate: &OpportunityCandidate| candidate.source_url == result.url)
            {
                candidates.push(candidate_from_search_result(result));
            }
        }

        run.artifacts.push(TaskArtifact {
            id: candidates_artifact_id,
            profile_id: run.profile_id,
            run_id: run.id,
            work_item_id: Some(extract_step_id),
            name: "Extracted opportunity candidates".to_string(),
            artifact_type: "extracted_opportunity_candidates".to_string(),
            location: None,
            created_at: Utc::now(),
            metadata: Some(json!({
                "candidate_count": candidates.len(),
                "readable_page_count": readable_pages.len(),
                "read_failures": read_failures,
            })),
            content_text: Some(render_candidates_text(&candidates)),
            content_json: Some(json!({
                "candidates": candidates,
            })),
        });
        finish_step(
            &mut run,
            extract_step_id,
            Some(format!(
                "Extracted {} opportunity candidates.",
                candidates.len()
            )),
            vec![candidates_artifact_id],
        );

        let score_step_id = start_step(&mut run, "score_matches")?;
        push_step_input_artifact(&mut run, score_step_id, candidates_artifact_id);
        let scored_artifact_id = Uuid::new_v4();
        let scoring_criteria = run
            .artifacts
            .iter()
            .find(|artifact| artifact.id == artifact_id)
            .and_then(|artifact| artifact.metadata.as_ref())
            .and_then(|metadata| metadata.get("criteria"))
            .and_then(|value| value.as_str())
            .unwrap_or("Score for fit against the employment profile.");
        let mut scored_matches = input
            .seed_scored_matches
            .clone()
            .into_iter()
            .take(limit)
            .collect::<Vec<_>>();

        for candidate in &candidates {
            if scored_matches
                .iter()
                .any(|scored_match| scored_match.candidate.source_url == candidate.source_url)
            {
                continue;
            }
            let candidate_json = serde_json::to_value(candidate)
                .unwrap_or_else(|_| json!({ "source_url": candidate.source_url }));
            let scored = self
                .model_execution
                .score_job_opportunity(
                    &self.llm_router.task_reasoning_model(),
                    &candidate_json,
                    scoring_criteria,
                    ExecutionContext::for_work_item(task.id, run.id, score_step_id)
                        .with_model_purpose("employment_match_score")
                        .with_input_summary(format!("Score candidate {}", candidate.source_url))
                        .with_output_artifact(scored_artifact_id),
                )
                .await
                .unwrap_or_else(|_| heuristic_candidate_score(candidate));
            scored_matches.push(scored_match_from_output(
                candidate.clone(),
                scored,
                min_score,
            ));
        }

        run.artifacts.push(TaskArtifact {
            id: scored_artifact_id,
            profile_id: run.profile_id,
            run_id: run.id,
            work_item_id: Some(score_step_id),
            name: "Scored opportunity matches".to_string(),
            artifact_type: "scored_opportunity_matches".to_string(),
            location: None,
            created_at: Utc::now(),
            metadata: Some(json!({
                "match_count": scored_matches.len(),
                "min_score": min_score,
                "passing_count": scored_matches.iter().filter(|item| item.passes_min_score).count(),
            })),
            content_text: Some(render_scored_matches_text(&scored_matches)),
            content_json: Some(json!({
                "matches": scored_matches,
                "min_score": min_score,
            })),
        });
        finish_step(
            &mut run,
            score_step_id,
            Some(format!(
                "Scored {} opportunity matches.",
                scored_matches.len()
            )),
            vec![scored_artifact_id],
        );

        let create_step_id = start_step(&mut run, "create_opportunities")?;
        push_step_input_artifact(&mut run, create_step_id, scored_artifact_id);
        let mut created_opportunity_ids = Vec::new();
        let mut existing_opportunity_ids = Vec::new();
        let mut skipped_opportunity_sources = Vec::new();

        if input.create_opportunities {
            for scored_match in scored_matches.iter().filter(|item| item.passes_min_score) {
                match self
                    .employment_repo
                    .find_opportunity_by_source_url(
                        run.profile_id,
                        &scored_match.candidate.source_url,
                    )
                    .await
                {
                    Ok(Some(existing)) => {
                        existing_opportunity_ids.push(existing.id.to_string());
                        if let Err(e) = self
                            .employment_repo
                            .touch_opportunity_seen_at(existing.id)
                            .await
                        {
                            tracing::warn!(
                                "Failed to touch existing opportunity {}: {}",
                                scored_match.candidate.source_url,
                                e
                            );
                        }
                        continue;
                    }
                    Ok(None) => {}
                    Err(e) => {
                        tracing::warn!(
                            "Failed to check existing opportunity {}: {}",
                            scored_match.candidate.source_url,
                            e
                        );
                    }
                }

                let opportunity = EmploymentOpportunity::new_discovered(
                    run.profile_id,
                    scored_match.candidate.source_url.clone(),
                    scored_match.candidate.source_name.clone(),
                    scored_match.candidate.source_artifact_id,
                );

                let opportunity = EmploymentOpportunity {
                    title: scored_match.candidate.title.clone(),
                    company: scored_match.candidate.company.clone(),
                    location: scored_match.candidate.location.clone(),
                    remote_type: scored_match.candidate.remote_type.clone(),
                    salary_min: scored_match.candidate.salary_min,
                    salary_max: scored_match.candidate.salary_max,
                    description_text: scored_match.candidate.description_text.clone(),
                    extracted_json: scored_match.candidate.extracted_json.clone(),
                    fit_score: scored_match.primary_fit_score.or(scored_match.oe_fit_score),
                    primary_fit_score: scored_match.primary_fit_score,
                    oe_fit_score: scored_match.oe_fit_score,
                    recommended_track: scored_match.recommended_track.clone(),
                    score_reason: scored_match.score_reason.clone(),
                    risk_flags: scored_match.risk_flags.clone(),
                    skip_recommendation: scored_match.skip_recommendation.clone(),
                    ..opportunity
                };

                match self.employment_repo.create_opportunity(opportunity).await {
                    Ok(created) => created_opportunity_ids.push(created.id.to_string()),
                    Err(e) => tracing::warn!(
                        "Failed to create opportunity {}: {}",
                        scored_match.candidate.source_url,
                        e
                    ),
                }
            }
        }
        for scored_match in scored_matches.iter().filter(|item| !item.passes_min_score) {
            skipped_opportunity_sources.push(scored_match.candidate.source_url.clone());
        }

        let created_opportunity_count = created_opportunity_ids.len();
        let existing_opportunity_count = existing_opportunity_ids.len();
        let created_summary_artifact_id = Uuid::new_v4();
        run.artifacts.push(TaskArtifact {
            id: created_summary_artifact_id,
            profile_id: run.profile_id,
            run_id: run.id,
            work_item_id: Some(create_step_id),
            name: "Created opportunities summary".to_string(),
            artifact_type: "created_opportunities_summary".to_string(),
            location: None,
            created_at: Utc::now(),
            metadata: Some(json!({
                "create_opportunities": input.create_opportunities,
                "created_opportunity_count": created_opportunity_count,
                "existing_opportunity_count": existing_opportunity_count,
                "skipped_count": skipped_opportunity_sources.len(),
            })),
            content_text: Some(format!(
                "Created {} opportunities, refreshed {}, skipped {} below score threshold.",
                created_opportunity_count,
                existing_opportunity_count,
                skipped_opportunity_sources.len()
            )),
            content_json: Some(json!({
                "created_opportunity_ids": created_opportunity_ids,
                "existing_opportunity_ids": existing_opportunity_ids,
                "skipped_sources": skipped_opportunity_sources,
                "create_opportunities": input.create_opportunities,
                "min_score": min_score,
            })),
        });
        finish_step(
            &mut run,
            create_step_id,
            Some(format!(
                "Created {} opportunities and refreshed {} existing opportunities.",
                created_opportunity_count, existing_opportunity_count
            )),
            vec![created_summary_artifact_id],
        );

        let summary_step_id = start_step(&mut run, "summarize_run")?;
        push_step_input_artifact(&mut run, summary_step_id, artifact_id);
        push_step_input_artifact(&mut run, summary_step_id, candidates_artifact_id);
        push_step_input_artifact(&mut run, summary_step_id, scored_artifact_id);
        push_step_input_artifact(&mut run, summary_step_id, created_summary_artifact_id);
        let passing_count = scored_matches
            .iter()
            .filter(|item| item.passes_min_score)
            .count();
        let summary = format!(
            "Employment search completed with {} raw results, {} readable pages, {} extracted candidates, {} matches at or above score {}, and {} newly created opportunities.",
            result_count,
            readable_pages.len(),
            candidates.len(),
            passing_count,
            min_score,
            created_opportunity_count
        );
        finish_step(&mut run, summary_step_id, Some(summary.clone()), vec![]);

        run.status = OpTaskRunStatus::Succeeded;
        run.completed_at = Some(Utc::now());
        run.summary = Some(summary);

        Ok(run)
    }

    async fn load_employment_context_summary(
        &self,
        profile_id: Uuid,
    ) -> anyhow::Result<Vec<Value>> {
        let mut items = Vec::new();

        for kind in [
            ContextKind::CareerProfile,
            ContextKind::EmploymentPreference,
            ContextKind::ResumeFact,
            ContextKind::ProjectSummary,
        ] {
            let contexts = self
                .context
                .get_relevant_context(profile_id, "", Some(kind.clone()))
                .await?;
            for context in contexts.into_iter().take(8) {
                items.push(json!({
                    "kind": context.kind,
                    "title": context.title,
                    "body_preview": context.body.chars().take(500).collect::<String>(),
                    "tags": context.tags,
                }));
            }
        }

        Ok(items)
    }
}

#[derive(Debug, Deserialize)]
struct EmploymentSearchInput {
    #[serde(default)]
    query_override: Option<String>,
    #[serde(default)]
    user_request: Option<String>,
    #[serde(default)]
    location: Option<String>,
    #[serde(default)]
    remote_preference: Option<String>,
    #[serde(default)]
    create_opportunities: bool,
    #[serde(default)]
    limit: Option<usize>,
    #[serde(default)]
    min_score: Option<i64>,
    #[serde(default)]
    source_artifact_id: Option<Uuid>,
    #[serde(default)]
    source_artifact_type: Option<String>,
    #[serde(default)]
    source_query: Option<String>,
    #[serde(default)]
    seed_search_results: Vec<SearchResultItem>,
    #[serde(default)]
    seed_readable_pages: Vec<SeedReadablePage>,
    #[serde(default)]
    seed_candidates: Vec<OpportunityCandidate>,
    #[serde(default)]
    seed_scored_matches: Vec<ScoredOpportunityMatch>,
}

#[derive(Debug, Deserialize)]
struct ArtifactSummaryInput {
    user_request: String,
    artifact_name: String,
    artifact_type: String,
    #[serde(default)]
    source_artifact_id: Option<Uuid>,
    #[serde(default)]
    content_text: Option<String>,
    #[serde(default)]
    content_json: Option<Value>,
}

#[derive(Debug, Deserialize)]
struct ChatGptEscalationInput {
    #[serde(default)]
    user_request: Option<String>,
    #[serde(default)]
    mode: Option<String>,
    #[serde(default)]
    confirm: bool,
    #[serde(default)]
    desired_output: Option<String>,
    #[serde(default)]
    context_query: Option<String>,
    #[serde(default)]
    context_limit: Option<usize>,
    #[serde(default)]
    context_text: Option<String>,
    #[serde(default)]
    context_json: Option<Value>,
}

#[derive(Debug, Clone, Deserialize)]
struct SeedReadablePage {
    #[serde(default)]
    artifact_id: Option<Uuid>,
    source_url: String,
    title: String,
    #[serde(default)]
    snippet: Option<String>,
    text: String,
}

#[derive(Debug, Clone)]
struct ReadablePageForExtraction {
    artifact_id: Uuid,
    source_url: String,
    title: String,
    snippet: Option<String>,
    text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct OpportunityCandidate {
    source_url: String,
    source_name: Option<String>,
    title: Option<String>,
    company: Option<String>,
    location: Option<String>,
    remote_type: Option<String>,
    salary_min: Option<i64>,
    salary_max: Option<i64>,
    description_text: Option<String>,
    extracted_json: Option<Value>,
    source_artifact_id: Option<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ScoredOpportunityMatch {
    candidate: OpportunityCandidate,
    primary_fit_score: Option<i64>,
    oe_fit_score: Option<i64>,
    recommended_track: Option<String>,
    score_reason: Option<String>,
    risk_flags: Vec<String>,
    skip_recommendation: Option<String>,
    passes_min_score: bool,
}

fn build_employment_search_query(input: &EmploymentSearchInput, profile_criteria: &str) -> String {
    let mut parts = Vec::new();

    if let Some(query_override) = clean_optional_text(input.query_override.as_deref()) {
        parts.push(query_override);
    } else {
        if let Some(user_request) = clean_optional_text(input.user_request.as_deref()) {
            parts.push(user_request);
        }
        if !profile_criteria.trim().is_empty() {
            parts.push(profile_criteria.trim().to_string());
        }
    }

    if let Some(location) = clean_optional_text(input.location.as_deref()) {
        parts.push(location);
    }
    if let Some(remote_preference) = clean_optional_text(input.remote_preference.as_deref()) {
        parts.push(remote_preference);
    }

    let query = parts.join(" ");
    employment_search_query(&query)
}

fn candidate_from_search_result(result: &SearchResultItem) -> OpportunityCandidate {
    OpportunityCandidate {
        source_url: result.url.clone(),
        source_name: Some(result.title.clone()),
        title: Some(result.title.clone()),
        company: None,
        location: None,
        remote_type: None,
        salary_min: None,
        salary_max: None,
        description_text: result.snippet.clone(),
        extracted_json: None,
        source_artifact_id: None,
    }
}

fn candidate_from_parsed(
    parsed: &Value,
    source_url: &str,
    source_name: Option<String>,
    fallback_description: Option<String>,
    source_artifact_id: Option<Uuid>,
) -> OpportunityCandidate {
    OpportunityCandidate {
        source_url: source_url.to_string(),
        source_name,
        title: parsed
            .get("title")
            .and_then(|value| value.as_str())
            .map(clean_score_text),
        company: parsed
            .get("company")
            .and_then(|value| value.as_str())
            .map(clean_score_text),
        location: parsed
            .get("location")
            .and_then(|value| value.as_str())
            .map(clean_score_text),
        remote_type: parsed
            .get("remote_type")
            .and_then(|value| value.as_str())
            .map(clean_score_text),
        salary_min: parsed.get("salary_min").and_then(|value| value.as_i64()),
        salary_max: parsed.get("salary_max").and_then(|value| value.as_i64()),
        description_text: parsed
            .get("description_text")
            .and_then(|value| value.as_str())
            .map(clean_score_text)
            .filter(|value| !value.is_empty())
            .or(fallback_description),
        extracted_json: Some(parsed.clone()),
        source_artifact_id,
    }
}

fn scored_match_from_output(
    candidate: OpportunityCandidate,
    scored: Value,
    min_score: i64,
) -> ScoredOpportunityMatch {
    let primary_fit_score = scored
        .get("primary_fit_score")
        .and_then(|value| value.as_i64())
        .map(clamp_score);
    let oe_fit_score = scored
        .get("oe_fit_score")
        .and_then(|value| value.as_i64())
        .map(clamp_score);
    let recommended_track = scored
        .get("recommended_track")
        .and_then(|value| value.as_str())
        .map(clean_score_text);
    let score_reason = scored
        .get("score_reason")
        .and_then(|value| value.as_str())
        .map(clean_score_text);
    let risk_flags = scored
        .get("risk_flags")
        .and_then(|value| value.as_array())
        .map(|items| {
            items
                .iter()
                .filter_map(|value| value.as_str())
                .map(clean_score_text)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let skip_recommendation = scored
        .get("skip_recommendation")
        .and_then(|value| value.as_str())
        .map(clean_score_text)
        .filter(|value| !value.is_empty());
    let best_score = primary_fit_score.or(oe_fit_score).unwrap_or(0);

    ScoredOpportunityMatch {
        candidate,
        primary_fit_score,
        oe_fit_score,
        recommended_track,
        score_reason,
        risk_flags,
        skip_recommendation,
        passes_min_score: best_score >= min_score,
    }
}

fn heuristic_candidate_score(candidate: &OpportunityCandidate) -> Value {
    let text = format!(
        "{}\n{}\n{}\n{}",
        candidate.title.clone().unwrap_or_default(),
        candidate.remote_type.clone().unwrap_or_default(),
        candidate.description_text.clone().unwrap_or_default(),
        candidate.location.clone().unwrap_or_default()
    )
    .to_lowercase();
    let remote_confirmed = text.contains("remote") && !text.contains("hybrid");
    let mut primary_fit_score = if text.contains("salesforce") { 74 } else { 55 };
    if text.contains("architect") || text.contains("automation") {
        primary_fit_score += 12;
    }
    let oe_fit_score = if remote_confirmed { 72 } else { 0 };

    json!({
        "primary_fit_score": primary_fit_score.clamp(0, 100),
        "oe_fit_score": oe_fit_score,
        "recommended_track": if primary_fit_score >= 75 {
            "primary"
        } else if oe_fit_score >= 75 {
            "oe"
        } else {
            "manual_review"
        },
        "score_reason": "Heuristic score from extracted candidate text.",
        "risk_flags": if remote_confirmed {
            Vec::<String>::new()
        } else {
            vec!["on_site_or_hybrid".to_string()]
        },
        "skip_recommendation": if remote_confirmed {
            Value::Null
        } else {
            json!("OE reject: remote work is not clearly confirmed.")
        }
    })
}

fn render_candidates_text(candidates: &[OpportunityCandidate]) -> String {
    if candidates.is_empty() {
        return "No opportunity candidates extracted.".to_string();
    }

    candidates
        .iter()
        .enumerate()
        .map(|(index, candidate)| {
            format!(
                "{}. {}\n{}\n{}",
                index + 1,
                candidate.title.as_deref().unwrap_or("Untitled opportunity"),
                candidate.source_url,
                candidate.description_text.as_deref().unwrap_or("")
            )
        })
        .collect::<Vec<_>>()
        .join("\n\n")
}

fn render_scored_matches_text(matches: &[ScoredOpportunityMatch]) -> String {
    if matches.is_empty() {
        return "No opportunity matches scored.".to_string();
    }

    matches
        .iter()
        .enumerate()
        .map(|(index, item)| {
            format!(
                "{}. {} | primary={} oe={} pass={}\n{}\n{}",
                index + 1,
                item.candidate
                    .title
                    .as_deref()
                    .unwrap_or("Untitled opportunity"),
                item.primary_fit_score
                    .map(|value| value.to_string())
                    .unwrap_or_else(|| "n/a".to_string()),
                item.oe_fit_score
                    .map(|value| value.to_string())
                    .unwrap_or_else(|| "n/a".to_string()),
                item.passes_min_score,
                item.candidate.source_url,
                item.score_reason.as_deref().unwrap_or("")
            )
        })
        .collect::<Vec<_>>()
        .join("\n\n")
}

fn clean_optional_text(value: Option<&str>) -> Option<String> {
    value
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn clean_score_text(value: &str) -> String {
    value.trim().to_string()
}

fn clamp_score(value: i64) -> i64 {
    value.clamp(0, 100)
}

fn truncate_chars(value: &str, max_chars: usize) -> String {
    let mut chars = value.chars();
    let truncated = chars.by_ref().take(max_chars).collect::<String>();
    if chars.next().is_some() {
        format!("{}...", truncated)
    } else {
        truncated
    }
}

fn render_chatgpt_escalation_prompt(
    user_request: &str,
    desired_output: Option<&str>,
    redacted_context: &Value,
    redaction_report: &RedactionReport,
) -> String {
    let context = serde_json::to_string_pretty(redacted_context)
        .unwrap_or_else(|_| "<unserializable context>".to_string());
    format!(
        "You are assisting Local Operator with a manual escalation.\n\nUser request:\n{}\n\nDesired output:\n{}\n\nRedaction summary:\n- redacted_keys: {}\n- redacted_text_values: {}\n\nRedacted context JSON:\n{}\n\nPlease answer with structured, actionable output. Preserve relevant IDs, URLs, task names, artifact types, and recommended next steps.",
        user_request,
        desired_output.unwrap_or("Return a concise answer with findings and next steps."),
        redaction_report.redacted_keys,
        redaction_report.redacted_text_values,
        context
    )
}

fn start_step(run: &mut OpTaskRun, name: &str) -> anyhow::Result<Uuid> {
    let item = run
        .work_items
        .iter_mut()
        .find(|item| item.name == name)
        .ok_or_else(|| anyhow!("planned work item '{}' not found", name))?;
    item.status = OpTaskRunStatus::Running;
    item.started_at = Some(Utc::now());
    item.completed_at = None;
    item.error = None;
    Ok(item.id)
}

fn finish_step(
    run: &mut OpTaskRun,
    step_id: Uuid,
    details: Option<String>,
    output_artifact_ids: Vec<Uuid>,
) {
    if let Some(item) = run.work_items.iter_mut().find(|item| item.id == step_id) {
        item.status = OpTaskRunStatus::Succeeded;
        item.completed_at = Some(Utc::now());
        item.details = details;
        item.output_artifact_ids.extend(output_artifact_ids);
    }
}

fn finish_step_with_error(run: &mut OpTaskRun, step_id: Uuid, error: &str) {
    if let Some(item) = run.work_items.iter_mut().find(|item| item.id == step_id) {
        item.status = OpTaskRunStatus::Failed;
        item.completed_at = Some(Utc::now());
        item.error = Some(error.to_string());
    }
}

fn update_step_tool_args(run: &mut OpTaskRun, step_id: Uuid, args: serde_json::Value) {
    if let Some(item) = run.work_items.iter_mut().find(|item| item.id == step_id) {
        item.tool_args_json = Some(args);
    }
}

fn push_step_input_artifact(run: &mut OpTaskRun, step_id: Uuid, artifact_id: Uuid) {
    if let Some(item) = run.work_items.iter_mut().find(|item| item.id == step_id) {
        item.input_artifact_ids.push(artifact_id);
    }
}

fn render_search_results_text(
    query: &str,
    results: &[crate::readers::models::SearchResultItem],
) -> String {
    if results.is_empty() {
        return format!("No search results found for '{}'.", query);
    }

    results
        .iter()
        .enumerate()
        .map(|(index, item)| {
            format!(
                "{}. {}\n{}\n{}",
                index + 1,
                item.title,
                item.url,
                item.snippet.clone().unwrap_or_default()
            )
        })
        .collect::<Vec<_>>()
        .join("\n\n")
}

fn employment_search_query(criteria: &str) -> String {
    let trimmed = criteria.trim();
    if trimmed.is_empty() {
        return "jobs".to_string();
    }

    let lower = trimmed.to_lowercase();
    if lower.contains("job") || lower.contains("opportunit") || lower.contains("role") {
        trimmed.to_string()
    } else {
        format!("jobs {}", trimmed)
    }
}
