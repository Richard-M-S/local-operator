use crate::op_tasks::models::{OpTask, OpTaskRun, OpTaskRunStatus, OpTaskStatus, OpWorkItem};
use crate::{
    error::AppError,
    op_tasks::{OpTaskRepository, OpTaskRunner},
};
use chrono::Utc;
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

    pub async fn create_op_task(
        &self,
        task_type: String,
        name: String,
        description: Option<String>,
    ) -> Result<OpTask, AppError> {
        self.validate_task_input(&task_type, &name, &description)?;

        let task = OpTask {
            id: Uuid::new_v4(),
            task_type: task_type.trim().to_string(),
            name: name.trim().to_string(),
            description,
            status: OpTaskStatus::Active,
            created_at: Utc::now(),
            updated_at: None,
        };

        self.repo
            .create_op_task(task)
            .await
            .map_err(|e| AppError::Internal(e.to_string()))
    }

    pub async fn create_task(
        &self,
        task_type: String,
        name: String,
        description: Option<String>,
        enabled: bool,
    ) -> Result<OpTask, AppError> {
        self.validate_task_input(&task_type, &name, &description)?;

        let task = OpTask {
            id: Uuid::new_v4(),
            task_type: task_type.trim().to_string(),
            name: name.trim().to_string(),
            description,
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

        for artifact in &executed_run.artifacts {
            self.repo
                .save_artifact(artifact.clone())
                .await
                .map_err(|e| AppError::Internal(e.to_string()))?;
        }

        Ok(executed_run)
    }

    pub async fn get_task_run_summary(&self, run_id: Uuid) -> Result<Option<OpTaskRun>, AppError> {
        let run = self
            .repo
            .get_task_run(run_id)
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?;

        match run {
            Some(mut run) => {
                run.artifacts = self
                    .repo
                    .list_artifacts_for_run(run.id)
                    .await
                    .map_err(|e| AppError::Internal(e.to_string()))?;
                Ok(Some(run))
            }
            None => Ok(None),
        }
    }

    pub async fn list_task_history(&self, task_id: Uuid) -> Result<Vec<OpTaskRun>, AppError> {
        let mut runs = self
            .repo
            .list_task_runs_for_task(task_id)
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?;

        for run in &mut runs {
            run.artifacts = self
                .repo
                .list_artifacts_for_run(run.id)
                .await
                .map_err(|e| AppError::Internal(e.to_string()))?;
        }

        Ok(runs)
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
}
