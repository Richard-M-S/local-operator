use crate::domains::employment::{models::EmploymentOpportunity, repository::EmploymentRepository};
use crate::op_tasks::models::{
    OpTask, OpTaskRun, OpTaskRunStatus, ReadUrlInput, SearchWebInput, TaskArtifact,
};
use crate::readers::ReaderService;
use crate::services::llm_router::LlmRouter;
use crate::services::llm_service::LlmService;
use crate::tools::registry::ToolRegistry;
use anyhow::{anyhow, Context};
use chrono::Utc;
use serde_json::json;
use tracing;
use uuid::Uuid;

#[derive(Clone)]
pub struct OpTaskRunner {
    tools: ToolRegistry,
    llm: Option<LlmService>,
    readers: ReaderService,
    llm_router: LlmRouter,
    employment_repo: EmploymentRepository,
}

impl OpTaskRunner {
    pub fn new(
        tools: ToolRegistry,
        llm: Option<LlmService>,
        readers: ReaderService,
        llm_router: LlmRouter,
        employment_repo: EmploymentRepository,
    ) -> Self {
        Self {
            tools,
            llm,
            readers,
            llm_router,
            employment_repo,
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
        let result = match self.tools.execute("system.get_status", json!({})).await {
            Ok(result) => result,
            Err(err) => {
                finish_step_with_error(&mut run, collect_step_id, &err.to_string());
                return Err(anyhow!(err)).context("failed to execute system.get_status tool");
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

        if let Some(llm) = &self.llm {
            let prompt = format!(
                "Summarize the following system status output in a short, actionable paragraph:\n\n{}",
                serde_json::to_string_pretty(&result.output)
                    .unwrap_or_else(|_| "<unserializable output>".to_string())
            );

            match llm
                .ask_model(
                    &summary_model,
                    "You are a system status summarization assistant.",
                    &prompt,
                )
                .await
            {
                Ok(summary_text) => summary = summary_text,
                Err(err) => {
                    summary = format!(
                        "System status collected, but LLM summarization failed: {}",
                        err
                    );
                    summary_details =
                        format!("LLM summarization failed; fallback summary used: {err}");
                }
            }
        } else {
            summary_details = "LLM disabled; fallback summary used.".to_string();
        }

        let artifact_id = Uuid::new_v4();
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
        #[derive(serde::Deserialize)]
        struct SearchOpportunitiesInput {
            #[serde(default)]
            create_opportunities: bool,
            #[serde(default)]
            limit: Option<usize>,
        }

        let profile_step_id = start_step(&mut run, "load_profile_criteria")?;
        let input: SearchOpportunitiesInput = match serde_json::from_value(task.input_json.clone())
        {
            Ok(input) => input,
            Err(err) => {
                finish_step_with_error(&mut run, profile_step_id, &err.to_string());
                return Err(anyhow!(err))
                    .context("invalid employment.search_opportunities input_json");
            }
        };
        let limit = input.limit.unwrap_or(10).clamp(1, 25);

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

        let criteria = profile
            .criteria
            .clone()
            .filter(|c| !c.trim().is_empty())
            .unwrap_or_else(|| "jobs".to_string());
        let search_query = employment_search_query(&criteria);
        finish_step(
            &mut run,
            profile_step_id,
            Some(format!("Loaded criteria for {}.", profile.display_name)),
            vec![],
        );

        let search_step_id = start_step(&mut run, "search_opportunities")?;
        update_step_tool_args(
            &mut run,
            search_step_id,
            json!({
                "query": search_query,
                "limit": limit,
            }),
        );
        let results = match self.readers.search_web(search_query.clone(), limit).await {
            Ok(results) => results,
            Err(err) => {
                finish_step_with_error(&mut run, search_step_id, &err.to_string());
                return Err(err).context("failed to search web for opportunities");
            }
        };

        let content_text = if results.results.is_empty() {
            "No job opportunities found for profile criteria.".to_string()
        } else {
            render_search_results_text(&results.query, &results.results)
        };

        let result_count = results.results.len();
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
                "criteria": criteria.clone(),
                "query": search_query,
                "profile_id": run.profile_id.to_string(),
                "result_count": result_count,
                "search_type": "employment_opportunities",
                "create_opportunities": input.create_opportunities,
                "profile_display_name": profile.display_name,
            })),
            content_text: Some(content_text),
            content_json: Some(json!({
                "criteria": criteria,
                "query": results.query,
                "results": results.results,
                "profile_id": run.profile_id.to_string(),
            })),
        });
        finish_step(
            &mut run,
            search_step_id,
            Some(format!("Search completed with {} results.", result_count)),
            vec![artifact_id],
        );

        let create_step_id = start_step(&mut run, "create_opportunities")?;
        push_step_input_artifact(&mut run, create_step_id, artifact_id);
        let mut created_opportunity_ids = Vec::new();
        let mut existing_opportunity_ids = Vec::new();

        if input.create_opportunities && !results.results.is_empty() {
            for (idx, result) in results.results.iter().take(5).enumerate() {
                match self
                    .employment_repo
                    .find_opportunity_by_source_url(run.profile_id, &result.url)
                    .await
                {
                    Ok(Some(existing)) => {
                        existing_opportunity_ids.push(existing.id.to_string());
                        if let Err(e) = self
                            .employment_repo
                            .touch_opportunity_seen_at(existing.id)
                            .await
                        {
                            tracing::warn!("Failed to touch existing opportunity {}: {}", idx, e);
                        }
                        continue;
                    }
                    Ok(None) => {}
                    Err(e) => {
                        tracing::warn!("Failed to check existing opportunity {}: {}", idx, e);
                    }
                }

                let opportunity = EmploymentOpportunity::new_discovered(
                    run.profile_id,
                    result.url.clone(),
                    Some(result.title.clone()),
                    Some(artifact_id),
                );

                let opportunity = EmploymentOpportunity {
                    title: Some(result.title.clone()),
                    description_text: result.snippet.clone(),
                    ..opportunity
                };

                match self.employment_repo.create_opportunity(opportunity).await {
                    Ok(created) => created_opportunity_ids.push(created.id.to_string()),
                    Err(e) => tracing::warn!("Failed to create opportunity {}: {}", idx, e),
                }
            }
        }

        let created_opportunity_count = created_opportunity_ids.len();
        let existing_opportunity_count = existing_opportunity_ids.len();
        if let Some(artifact) = run
            .artifacts
            .iter_mut()
            .find(|artifact| artifact.id == artifact_id)
        {
            let existing_metadata = artifact.metadata.clone();
            artifact.metadata = Some(json!({
                "criteria": existing_metadata
                    .as_ref()
                    .and_then(|metadata| metadata.get("criteria"))
                    .cloned(),
                "query": existing_metadata
                    .as_ref()
                    .and_then(|metadata| metadata.get("query"))
                    .cloned(),
                "profile_display_name": existing_metadata
                    .as_ref()
                    .and_then(|metadata| metadata.get("profile_display_name"))
                    .cloned(),
                "profile_id": run.profile_id.to_string(),
                "result_count": result_count,
                "search_type": "employment_opportunities",
                "create_opportunities": input.create_opportunities,
                "created_opportunity_count": created_opportunity_count,
                "created_opportunity_ids": created_opportunity_ids,
                "existing_opportunity_count": existing_opportunity_count,
                "existing_opportunity_ids": existing_opportunity_ids,
            }));
        }
        finish_step(
            &mut run,
            create_step_id,
            Some(format!(
                "Created {} opportunities and refreshed {} existing opportunities.",
                created_opportunity_count, existing_opportunity_count
            )),
            vec![],
        );

        run.status = OpTaskRunStatus::Succeeded;
        run.completed_at = Some(Utc::now());
        run.summary = Some(format!(
            "Employment search completed with {} results{}.",
            result_count,
            if input.create_opportunities {
                format!(" ({} opportunities created)", created_opportunity_count)
            } else {
                "".to_string()
            }
        ));

        Ok(run)
    }
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
