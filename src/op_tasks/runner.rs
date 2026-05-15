use crate::domains::employment::{models::EmploymentOpportunity, repository::EmploymentRepository};
use crate::op_tasks::models::{
    OpTask, OpTaskRun, OpTaskRunStatus, OpWorkItem, ReadUrlInput, SearchWebInput, TaskArtifact,
};
use crate::readers::ReaderService;
use crate::services::llm_router::LlmRouter;
use crate::services::llm_service::LlmService;
use crate::tools::registry::ToolRegistry;
use anyhow::Context;
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
        let started_at = Utc::now();
        run.status = OpTaskRunStatus::Running;
        run.started_at = Some(started_at);

        match task.task_type.as_str() {
            "system.status_report" => self.run_status_report(&task, run).await,
            "reader.read_url" => self.run_read_url(&task, run).await,
            "reader.search_web" => self.run_search_web(&task, run).await,
            "employment.search_opportunities" => {
                self.run_employment_search_opportunities(&task, run).await
            }
            _ => {
                let message = format!("unsupported task type: {}", task.task_type);
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
        let now = Utc::now();
        let work_item_id = {
            let item = self.ensure_work_item(&mut run, "status_check", "Collect system status");
            item.status = OpTaskRunStatus::Running;
            item.started_at = Some(now);
            item.id
        };

        let result = self
            .tools
            .execute("system.get_status", json!({}))
            .await
            .context("failed to execute system.get_status tool")?;

        {
            let item = run
                .work_items
                .iter_mut()
                .find(|wi| wi.id == work_item_id)
                .unwrap();
            item.status = OpTaskRunStatus::Succeeded;
            item.completed_at = Some(Utc::now());
        }

        let mut summary = format!("System status collected by {}", result.tool);
        let summary_model = self.llm_router.task_summary_model();
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
                    )
                }
            }
        }

        let run_id = run.id;
        run.artifacts.push(TaskArtifact {
            id: Uuid::new_v4(),
            profile_id: run.profile_id,
            run_id,
            work_item_id: Some(work_item_id),
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
            content_text: None,
            content_json: None,
        });

        run.summary = Some(summary);
        run.status = OpTaskRunStatus::Succeeded;
        run.completed_at = Some(Utc::now());
        Ok(run)
    }

    async fn run_read_url(&self, task: &OpTask, mut run: OpTaskRun) -> anyhow::Result<OpTaskRun> {
        let input: ReadUrlInput = serde_json::from_value(task.input_json.clone())
            .context("invalid reader.read_url input_json")?;

        let mut work_item = OpWorkItem {
            id: Uuid::new_v4(),
            run_id: run.id,
            name: "read_url".to_string(),
            description: Some("Read and extract text from URL".to_string()),
            order: 1,
            status: OpTaskRunStatus::Running,
            started_at: Some(Utc::now()),
            completed_at: None,
            details: None,
        };

        let result = self
            .readers
            .read_url(input.url.clone())
            .await
            .context("failed to read URL")?;

        work_item.status = OpTaskRunStatus::Succeeded;
        work_item.completed_at = Some(Utc::now());

        let title = result
            .title
            .clone()
            .unwrap_or_else(|| "read_url_result".to_string());
        let cleaned_text = result.cleaned_text.clone();

        let artifact = TaskArtifact {
            id: Uuid::new_v4(),
            profile_id: run.profile_id,
            run_id: run.id,
            work_item_id: Some(work_item.id),
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
        };

        run.work_items.push(work_item);
        run.artifacts.push(artifact);
        run.status = OpTaskRunStatus::Succeeded;
        run.completed_at = Some(Utc::now());
        run.summary = Some("Read URL and extracted readable text.".to_string());

        Ok(run)
    }

    async fn run_search_web(&self, task: &OpTask, mut run: OpTaskRun) -> anyhow::Result<OpTaskRun> {
        let input: SearchWebInput = serde_json::from_value(task.input_json.clone())
            .context("invalid reader.search_web input_json")?;
        let limit = input.limit.unwrap_or(10).clamp(1, 25);

        let mut work_item = OpWorkItem {
            id: Uuid::new_v4(),
            run_id: run.id,
            name: "search_web".to_string(),
            description: Some("Search the web and save result links".to_string()),
            order: 1,
            status: OpTaskRunStatus::Running,
            started_at: Some(Utc::now()),
            completed_at: None,
            details: None,
        };

        let results = self
            .readers
            .search_web(input.query.clone(), limit)
            .await
            .context("failed to search web")?;

        work_item.status = OpTaskRunStatus::Succeeded;
        work_item.completed_at = Some(Utc::now());

        let content_text = if results.results.is_empty() {
            format!("No search results found for '{}'.", results.query)
        } else {
            results
                .results
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
        };

        let result_count = results.results.len();
        let query = results.query.clone();

        let artifact = TaskArtifact {
            id: Uuid::new_v4(),
            profile_id: run.profile_id,
            run_id: run.id,
            work_item_id: Some(work_item.id),
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
        };

        run.work_items.push(work_item);
        run.artifacts.push(artifact);
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
        // Parse input - may include create_opportunities flag
        #[derive(serde::Deserialize)]
        struct SearchOpportunitiesInput {
            #[serde(default)]
            create_opportunities: bool,
            #[serde(default)]
            limit: Option<usize>,
        }

        let input: SearchOpportunitiesInput = serde_json::from_value(task.input_json.clone())
            .context("invalid employment.search_opportunities input_json")?;
        let limit = input.limit.unwrap_or(10).clamp(1, 25);

        // Load profile and criteria
        let profile = self
            .employment_repo
            .get_profile(run.profile_id)
            .await
            .context("failed to load employment profile")?
            .ok_or_else(|| anyhow::anyhow!("employment profile not found"))?;

        let criteria = profile
            .criteria
            .filter(|c| !c.trim().is_empty())
            .unwrap_or_else(|| "jobs".to_string());
        let search_query = employment_search_query(&criteria);

        // Create work item for search
        let mut work_item = OpWorkItem {
            id: Uuid::new_v4(),
            run_id: run.id,
            name: "search_opportunities".to_string(),
            description: Some("Search job opportunities based on profile criteria".to_string()),
            order: 1,
            status: OpTaskRunStatus::Running,
            started_at: Some(Utc::now()),
            completed_at: None,
            details: None,
        };

        // Search web using profile criteria
        let results = self
            .readers
            .search_web(search_query.clone(), limit)
            .await
            .context("failed to search web for opportunities")?;

        work_item.status = OpTaskRunStatus::Succeeded;
        work_item.completed_at = Some(Utc::now());

        // Format content_text: human-readable summary
        let content_text = if results.results.is_empty() {
            format!("No job opportunities found for profile criteria.")
        } else {
            results
                .results
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
        };

        let result_count = results.results.len();
        let artifact_id = Uuid::new_v4();
        let mut created_opportunity_ids = Vec::new();
        let mut existing_opportunity_ids = Vec::new();

        // Optionally create employment opportunities from search results
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

        // Create search_result_set artifact
        let artifact = TaskArtifact {
            id: artifact_id,
            profile_id: run.profile_id,
            run_id: run.id,
            work_item_id: Some(work_item.id),
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
                "created_opportunity_count": created_opportunity_count,
                "created_opportunity_ids": created_opportunity_ids,
                "existing_opportunity_count": existing_opportunity_count,
                "existing_opportunity_ids": existing_opportunity_ids,
                "profile_display_name": profile.display_name,
            })),
            content_text: Some(content_text),
            content_json: Some(json!({
                "criteria": criteria,
                "query": results.query,
                "results": results.results,
                "profile_id": run.profile_id.to_string(),
            })),
        };

        run.work_items.push(work_item);
        run.artifacts.push(artifact.clone());

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

    fn ensure_work_item<'a>(
        &self,
        run: &'a mut OpTaskRun,
        name: &str,
        description: &str,
    ) -> &'a mut OpWorkItem {
        if run.work_items.is_empty() {
            run.work_items.push(OpWorkItem {
                id: Uuid::new_v4(),
                run_id: run.id,
                name: name.to_string(),
                description: Some(description.to_string()),
                order: 1,
                status: OpTaskRunStatus::Pending,
                started_at: None,
                completed_at: None,
                details: None,
            });
        }

        run.work_items.first_mut().unwrap()
    }
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
