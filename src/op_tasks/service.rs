use crate::{
    context::{models::SavedContext, ContextService},
    error::AppError,
    op_tasks::models::{
        ArtifactContextBodySource, ArtifactSearch, OpTask, OpTaskRun, OpTaskRunStatus,
        OpTaskStatus, OpWorkItem, PromoteArtifactToContextRequest, TaskArtifact,
    },
    op_tasks::{OpTaskRepository, OpTaskRunner},
};
use chrono::Utc;
use serde_json::Value;
use uuid::Uuid;

#[derive(Clone)]
pub struct OpTaskService {
    repo: OpTaskRepository,
    runner: OpTaskRunner,
}

impl OpTaskService {
    pub fn new(repo: OpTaskRepository, runner: OpTaskRunner) -> Self {
        Self { repo, runner }
    }

    pub async fn create_task(
        &self,
        task_type: String,
        name: String,
        description: Option<String>,
        input_json: Value,
        enabled: bool,
    ) -> Result<OpTask, AppError> {
        self.validate_task_input(&task_type, &name, &description)?;

        let task = OpTask {
            id: Uuid::new_v4(),
            task_type: task_type.trim().to_string(),
            name: name.trim().to_string(),
            description,
            input_json,
            status: if enabled {
                OpTaskStatus::Active
            } else {
                OpTaskStatus::Paused
            },
            created_at: Utc::now(),
            updated_at: None,
        };

        self.repo
            .create_op_task(task)
            .await
            .map_err(|e| AppError::Internal(e.to_string()))
    }
    pub async fn get_artifact(&self, artifact_id: Uuid) -> Result<TaskArtifact, AppError> {
        self.repo
            .get_artifact(artifact_id)
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?
            .ok_or_else(|| AppError::NotFound("Op Task artifact not found".to_string()))
    }

    pub async fn list_artifacts(
        &self,
        search: ArtifactSearch,
    ) -> Result<Vec<TaskArtifact>, AppError> {
        self.repo
            .list_artifacts(search)
            .await
            .map_err(|e| AppError::Internal(e.to_string()))
    }

    pub async fn promote_artifact_to_context(
        &self,
        context: &ContextService,
        artifact_id: Uuid,
        request: PromoteArtifactToContextRequest,
    ) -> Result<SavedContext, AppError> {
        let artifact = self.get_artifact(artifact_id).await?;

        let title = request.title.trim().to_string();
        if title.is_empty() {
            return Err(AppError::BadRequest(
                "context title cannot be empty".to_string(),
            ));
        }

        let body = match request.body_source {
            ArtifactContextBodySource::ContentText => {
                artifact.content_text.clone().ok_or_else(|| {
                    AppError::BadRequest("artifact has no content_text to save".to_string())
                })?
            }
            ArtifactContextBodySource::ContentJson => artifact
                .content_json
                .as_ref()
                .map(serde_json::to_string_pretty)
                .transpose()
                .map_err(|err| AppError::Internal(err.to_string()))?
                .ok_or_else(|| {
                    AppError::BadRequest("artifact has no content_json to save".to_string())
                })?,
            ArtifactContextBodySource::Metadata => artifact
                .metadata
                .as_ref()
                .map(serde_json::to_string_pretty)
                .transpose()
                .map_err(|err| AppError::Internal(err.to_string()))?
                .ok_or_else(|| {
                    AppError::BadRequest("artifact has no metadata to save".to_string())
                })?,
        };

        if body.trim().is_empty() {
            return Err(AppError::BadRequest(
                "artifact content is empty".to_string(),
            ));
        }

        let tags = request
            .tags
            .into_iter()
            .map(|tag| tag.trim().to_string())
            .filter(|tag| !tag.is_empty())
            .collect();

        context
            .save_context_note(
                request.kind,
                title,
                body,
                artifact.location.clone(),
                Some(artifact.id),
                tags,
            )
            .await
            .map_err(|err| AppError::Internal(err.to_string()))
    }

    pub async fn get_op_task(&self, task_id: Uuid) -> Result<Option<OpTask>, AppError> {
        self.repo
            .get_op_task(task_id)
            .await
            .map_err(|e| AppError::Internal(e.to_string()))
    }

    pub async fn list_tasks(&self) -> Result<Vec<OpTask>, AppError> {
        self.repo
            .list_op_tasks()
            .await
            .map_err(|e| AppError::Internal(e.to_string()))
    }

    pub async fn run_task(&self, task_id: Uuid) -> Result<OpTaskRun, AppError> {
        self.start_task_run(task_id, vec![]).await
    }

    pub async fn start_task_run(
        &self,
        task_id: Uuid,
        work_items: Vec<OpWorkItem>,
    ) -> Result<OpTaskRun, AppError> {
        let task = self
            .get_op_task(task_id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("task {} not found", task_id)))?;

        if !self.is_task_allowed(&task) {
            return Err(AppError::PolicyDenied(format!(
                "task '{}' is not allowed to run",
                task.name
            )));
        }

        let run_id = Uuid::new_v4();
        let prepared_items = work_items
            .into_iter()
            .enumerate()
            .map(|(index, mut item)| {
                item.order = (index + 1) as u32;
                item.status = OpTaskRunStatus::Pending;
                item.run_id = run_id;
                item
            })
            .collect::<Vec<_>>();

        let run = OpTaskRun {
            id: run_id,
            task_id,
            status: OpTaskRunStatus::Pending,
            started_at: None,
            completed_at: None,
            work_items: prepared_items,
            artifacts: vec![],
            summary: None,
        };

        let run = self
            .repo
            .create_task_run(run)
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?;

        let executed_run = match self.runner.execute(task.clone(), run.clone()).await {
            Ok(executed_run) => executed_run,
            Err(err) => {
                let mut failed_run = run;
                failed_run.status = OpTaskRunStatus::Failed;
                failed_run.completed_at = Some(Utc::now());
                failed_run.summary = Some(err.to_string());

                self.repo
                    .update_task_run(failed_run.clone())
                    .await
                    .map_err(|e| AppError::Internal(e.to_string()))?;

                return Ok(failed_run);
            }
        };

        let executed_run = self
            .repo
            .update_task_run(executed_run)
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?;

        Ok(executed_run)
    }

    pub fn is_task_allowed(&self, task: &OpTask) -> bool {
        matches!(task.status, OpTaskStatus::Active)
    }

    fn validate_task_input(
        &self,
        task_type: &str,
        name: &str,
        description: &Option<String>,
    ) -> Result<(), AppError> {
        let task_type = task_type.trim();
        let name = name.trim();

        if task_type.is_empty() {
            return Err(AppError::BadRequest(
                "task type cannot be empty".to_string(),
            ));
        }

        if task_type.len() > 128 {
            return Err(AppError::BadRequest(
                "task type cannot exceed 128 characters".to_string(),
            ));
        }

        if name.is_empty() {
            return Err(AppError::BadRequest(
                "task name cannot be empty".to_string(),
            ));
        }

        if name.len() > 256 {
            return Err(AppError::BadRequest(
                "task name cannot exceed 256 characters".to_string(),
            ));
        }

        if let Some(desc) = description {
            if desc.len() > 1024 {
                return Err(AppError::BadRequest(
                    "task description cannot exceed 1024 characters".to_string(),
                ));
            }
        }

        Ok(())
    }

    pub async fn get_run(&self, run_id: Uuid) -> Result<OpTaskRun, AppError> {
        self.repo
            .get_task_run(run_id)
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?
            .ok_or_else(|| AppError::NotFound(format!("Op Task run {} not found", run_id)))
    }

    pub async fn list_runs_for_task(&self, task_id: Uuid) -> Result<Vec<OpTaskRun>, AppError> {
        self.get_op_task(task_id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("task {} not found", task_id)))?;

        self.repo
            .list_task_runs_for_task(task_id)
            .await
            .map_err(|e| AppError::Internal(e.to_string()))
    }
}
