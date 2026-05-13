use crate::op_tasks::models::{OpTask, OpTaskRun, OpTaskRunStatus, OpWorkItem, TaskArtifact};
use crate::services::llm_service::LlmService;
use crate::tools::registry::ToolRegistry;
use anyhow::{anyhow, Context};
use chrono::Utc;
use serde_json::json;
use uuid::Uuid;

#[derive(Clone)]
pub struct OpTaskRunner {
    tools: ToolRegistry,
    llm: Option<LlmService>,
}

impl OpTaskRunner {
    pub fn new(tools: ToolRegistry, llm: Option<LlmService>) -> Self {
        Self { tools, llm }
    }

    pub async fn execute(&self, task: OpTask, mut run: OpTaskRun) -> anyhow::Result<OpTaskRun> {
        let started_at = Utc::now();
        run.status = OpTaskRunStatus::Running;
        run.started_at = Some(started_at);

        match task.task_type.as_str() {
            "system.status_report" => self.run_status_report(&mut run).await?,
            _ => {
                let message = format!("unsupported task type: {}", task.task_type);
                run.status = OpTaskRunStatus::Failed;
                run.completed_at = Some(Utc::now());
                run.summary = Some(message);
                return Ok(run);
            }
        }

        run.status = OpTaskRunStatus::Succeeded;
        run.completed_at = Some(Utc::now());
        Ok(run)
    }

    async fn run_status_report(&self, run: &mut OpTaskRun) -> anyhow::Result<()> {
        let now = Utc::now();
        let work_item_id = {
            let item = self.ensure_work_item(run, "status_check", "Collect system status");
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
        if let Some(llm) = &self.llm {
            let prompt = format!(
                "Summarize the following system status output in a short, actionable paragraph:\n\n{}",
                serde_json::to_string_pretty(&result.output)
                    .unwrap_or_else(|_| "<unserializable output>".to_string())
            );

            match llm
                .ask_model(
                    "gpt-3.5-mini",
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
            run_id,
            work_item_id: Some(work_item_id),
            name: "system_status_report".to_string(),
            artifact_type: "status_report".to_string(),
            location: None,
            created_at: Utc::now(),
            metadata: Some(json!({
                "tool": result.tool,
                "output": result.output,
            })),
        });

        run.summary = Some(summary);
        Ok(())
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
