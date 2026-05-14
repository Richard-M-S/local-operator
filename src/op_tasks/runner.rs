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
use uuid::Uuid;

#[derive(Clone)]
pub struct OpTaskRunner {
    tools: ToolRegistry,
    llm: Option<LlmService>,
    readers: ReaderService,
    llm_router: LlmRouter,
}

impl OpTaskRunner {
    pub fn new(
        tools: ToolRegistry,
        llm: Option<LlmService>,
        readers: ReaderService,
        llm_router: LlmRouter,
    ) -> Self {
        Self {
            tools,
            llm,
            readers,
            llm_router,
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
            artifact_type: "search_results".to_string(),
            location: None,
            created_at: Utc::now(),
            metadata: Some(json!({
                "query": query,
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
