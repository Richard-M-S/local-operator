use crate::op_tasks::models::{
    ArtifactSearch, OpTask, OpTaskRun, OpTaskRunStatus, OpTaskStatus, TaskArtifact,
};
use sqlx::{FromRow, QueryBuilder, Sqlite, SqlitePool};
use uuid::Uuid;

#[derive(Clone)]
pub struct OpTaskRepository {
    pool: SqlitePool,
}

impl OpTaskRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn create_op_task(&self, task: OpTask) -> anyhow::Result<OpTask> {
        sqlx::query(
            r#"
            INSERT INTO op_tasks (
                id, task_type, name, description, input_json, status, created_at, updated_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
            "#,
        )
        .bind(task.id.to_string())
        .bind(&task.task_type)
        .bind(&task.name)
        .bind(&task.description)
        .bind(task.input_json.to_string())
        .bind(task.status.as_str())
        .bind(task.created_at.to_rfc3339())
        .bind(task.updated_at.map(|dt| dt.to_rfc3339()))
        .execute(&self.pool)
        .await?;

        Ok(task)
    }

    pub async fn get_op_task(&self, task_id: Uuid) -> anyhow::Result<Option<OpTask>> {
        let row = sqlx::query_as::<_, OpTaskRow>(
            r#"
            SELECT id, task_type, name, description, input_json, status, created_at, updated_at
            FROM op_tasks
            WHERE id = ?1
            "#,
        )
        .bind(task_id.to_string())
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(Into::into))
    }

    pub async fn list_op_tasks(&self) -> anyhow::Result<Vec<OpTask>> {
        let rows = sqlx::query_as::<_, OpTaskRow>(
            r#"
            SELECT id, task_type, name, description, input_json, status, created_at, updated_at
            FROM op_tasks
            ORDER BY created_at DESC
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(Into::into).collect())
    }

    pub async fn create_task_run(&self, run: OpTaskRun) -> anyhow::Result<OpTaskRun> {
        let work_items_json = serde_json::to_string(&run.work_items)?;

        sqlx::query(
            r#"
            INSERT INTO op_task_runs (
                id, task_id, status, started_at, completed_at, work_items_json, summary
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
            "#,
        )
        .bind(run.id.to_string())
        .bind(run.task_id.to_string())
        .bind(run.status.as_str())
        .bind(run.started_at.map(|dt| dt.to_rfc3339()))
        .bind(run.completed_at.map(|dt| dt.to_rfc3339()))
        .bind(work_items_json)
        .bind(&run.summary)
        .execute(&self.pool)
        .await?;

        for artifact in &run.artifacts {
            self.save_artifact(artifact.clone()).await?;
        }

        Ok(run)
    }

    pub async fn update_task_run(&self, run: OpTaskRun) -> anyhow::Result<OpTaskRun> {
        let work_items_json = serde_json::to_string(&run.work_items)?;

        sqlx::query(
            r#"
            UPDATE op_task_runs
            SET
                status = ?1,
                started_at = ?2,
                completed_at = ?3,
                work_items_json = ?4,
                summary = ?5
            WHERE id = ?6
            "#,
        )
        .bind(run.status.as_str())
        .bind(run.started_at.map(|dt| dt.to_rfc3339()))
        .bind(run.completed_at.map(|dt| dt.to_rfc3339()))
        .bind(work_items_json)
        .bind(&run.summary)
        .bind(run.id.to_string())
        .execute(&self.pool)
        .await?;

        for artifact in &run.artifacts {
            self.save_artifact(artifact.clone()).await?;
        }

        Ok(run)
    }

    pub async fn get_task_run(&self, run_id: Uuid) -> anyhow::Result<Option<OpTaskRun>> {
        let row = sqlx::query_as::<_, OpTaskRunRow>(
            r#"
            SELECT
                id,
                task_id,
                status,
                started_at,
                completed_at,
                work_items_json,
                summary
            FROM op_task_runs
            WHERE id = ?1
            "#,
        )
        .bind(run_id.to_string())
        .fetch_optional(&self.pool)
        .await?;

        let Some(row) = row else {
            return Ok(None);
        };

        let mut run: OpTaskRun = row.into();
        run.artifacts = self.list_artifacts_for_run(run.id).await?;

        Ok(Some(run))
    }

    pub async fn list_task_runs_for_task(&self, task_id: Uuid) -> anyhow::Result<Vec<OpTaskRun>> {
        let rows = sqlx::query_as::<_, OpTaskRunRow>(
            r#"
            SELECT
                id,
                task_id,
                status,
                started_at,
                completed_at,
                work_items_json,
                summary
            FROM op_task_runs
            WHERE task_id = ?1
            ORDER BY started_at DESC
            "#,
        )
        .bind(task_id.to_string())
        .fetch_all(&self.pool)
        .await?;

        let mut runs = Vec::with_capacity(rows.len());

        for row in rows {
            let mut run: OpTaskRun = row.into();
            run.artifacts = self.list_artifacts_for_run(run.id).await?;
            runs.push(run);
        }

        Ok(runs)
    }

    pub async fn save_artifact(&self, artifact: TaskArtifact) -> anyhow::Result<()> {
        let metadata_json = artifact
            .metadata
            .as_ref()
            .map(serde_json::to_string)
            .transpose()?;
        let content_json = artifact
            .content_json
            .as_ref()
            .map(serde_json::to_string)
            .transpose()?;

        sqlx::query(
            r#"
            INSERT OR REPLACE INTO task_artifacts (
                id, run_id, work_item_id, name, artifact_type, location, created_at, metadata_json, content_text, content_json
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
            "#,
        )
        .bind(artifact.id.to_string())
        .bind(artifact.run_id.to_string())
        .bind(artifact.work_item_id.map(|uuid| uuid.to_string()))
        .bind(&artifact.name)
        .bind(&artifact.artifact_type)
        .bind(&artifact.location)
        .bind(artifact.created_at.to_rfc3339())
        .bind(metadata_json)
        .bind(&artifact.content_text)
        .bind(content_json)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn list_artifacts_for_run(&self, run_id: Uuid) -> anyhow::Result<Vec<TaskArtifact>> {
        let rows = sqlx::query_as::<_, TaskArtifactRow>(
            r#"
            SELECT id, run_id, work_item_id, name, artifact_type, location, created_at, metadata_json, content_text, content_json
            FROM task_artifacts
            WHERE run_id = ?1
            ORDER BY created_at ASC
            "#,
        )
        .bind(run_id.to_string())
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(Into::into).collect())
    }

    pub async fn get_artifact(&self, artifact_id: Uuid) -> anyhow::Result<Option<TaskArtifact>> {
        let row = sqlx::query_as::<_, TaskArtifactRow>(
            r#"
            SELECT
                id,
                run_id,
                work_item_id,
                name,
                artifact_type,
                location,
                created_at,
                metadata_json,
                content_text,
                content_json
            FROM task_artifacts
            WHERE id = ?1
            "#,
        )
        .bind(artifact_id.to_string())
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(Into::into))
    }

    pub async fn list_artifacts(
        &self,
        search: ArtifactSearch,
    ) -> anyhow::Result<Vec<TaskArtifact>> {
        let limit = search.limit.unwrap_or(50).clamp(1, 200);
        let offset = search.offset.unwrap_or(0).max(0);

        let mut query = QueryBuilder::<Sqlite>::new(
            r#"
            SELECT
                a.id,
                a.run_id,
                a.work_item_id,
                a.name,
                a.artifact_type,
                a.location,
                a.created_at,
                a.metadata_json,
                a.content_text,
                a.content_json
            FROM task_artifacts a
            INNER JOIN op_task_runs r ON r.id = a.run_id
            WHERE 1 = 1
            "#,
        );

        if let Some(run_id) = search.run_id {
            query.push(" AND a.run_id = ");
            query.push_bind(run_id.to_string());
        }

        if let Some(task_id) = search.task_id {
            query.push(" AND r.task_id = ");
            query.push_bind(task_id.to_string());
        }

        if let Some(artifact_type) = search.artifact_type {
            query.push(" AND a.artifact_type = ");
            query.push_bind(artifact_type);
        }

        if let Some(source_url) = search.source_url {
            query.push(" AND a.location = ");
            query.push_bind(source_url);
        }

        query.push(" ORDER BY a.created_at DESC LIMIT ");
        query.push_bind(limit);
        query.push(" OFFSET ");
        query.push_bind(offset);

        let rows = query
            .build_query_as::<TaskArtifactRow>()
            .fetch_all(&self.pool)
            .await?;

        Ok(rows.into_iter().map(Into::into).collect())
    }
}

#[derive(FromRow)]
struct OpTaskRow {
    id: String,
    task_type: String,
    name: String,
    description: Option<String>,
    input_json: String,
    status: String,
    created_at: String,
    updated_at: Option<String>,
}

impl From<OpTaskRow> for OpTask {
    fn from(row: OpTaskRow) -> Self {
        Self {
            id: Uuid::parse_str(&row.id).unwrap(),
            task_type: row.task_type,
            name: row.name,
            description: row.description,
            input_json: serde_json::from_str(&row.input_json)
                .unwrap_or_else(|_| serde_json::json!({})),
            status: parse_task_status(&row.status),
            created_at: row.created_at.parse().unwrap(),
            updated_at: row.updated_at.and_then(|value| value.parse().ok()),
        }
    }
}

#[derive(FromRow)]
struct OpTaskRunRow {
    id: String,
    task_id: String,
    status: String,
    started_at: Option<String>,
    completed_at: Option<String>,
    work_items_json: Option<String>,
    summary: Option<String>,
}

impl From<OpTaskRunRow> for OpTaskRun {
    fn from(row: OpTaskRunRow) -> Self {
        let work_items = row
            .work_items_json
            .map(|json| serde_json::from_str(&json).unwrap_or_default())
            .unwrap_or_default();

        Self {
            id: Uuid::parse_str(&row.id).unwrap(),
            task_id: Uuid::parse_str(&row.task_id).unwrap(),
            status: parse_task_run_status(&row.status),
            started_at: row.started_at.and_then(|value| value.parse().ok()),
            completed_at: row.completed_at.and_then(|value| value.parse().ok()),
            work_items,
            artifacts: vec![],
            summary: row.summary,
        }
    }
}

#[derive(FromRow)]
struct TaskArtifactRow {
    id: String,
    run_id: String,
    work_item_id: Option<String>,
    name: String,
    artifact_type: String,
    location: Option<String>,
    created_at: String,
    metadata_json: Option<String>,
    content_text: Option<String>,
    content_json: Option<String>,
}

impl From<TaskArtifactRow> for TaskArtifact {
    fn from(row: TaskArtifactRow) -> Self {
        Self {
            id: Uuid::parse_str(&row.id).unwrap(),
            run_id: Uuid::parse_str(&row.run_id).unwrap(),
            work_item_id: row
                .work_item_id
                .and_then(|value| Uuid::parse_str(&value).ok()),
            name: row.name,
            artifact_type: row.artifact_type,
            location: row.location,
            created_at: row.created_at.parse().unwrap(),
            metadata: row
                .metadata_json
                .and_then(|value| serde_json::from_str(&value).ok()),
            content_text: row.content_text,
            content_json: row
                .content_json
                .and_then(|value| serde_json::from_str(&value).ok()),
        }
    }
}

fn parse_task_status(value: &str) -> OpTaskStatus {
    match value {
        "Draft" | "draft" => OpTaskStatus::Draft,
        "Active" | "active" => OpTaskStatus::Active,
        "Paused" | "paused" => OpTaskStatus::Paused,
        "Archived" | "archived" => OpTaskStatus::Archived,
        _ => OpTaskStatus::Draft,
    }
}

fn parse_task_run_status(value: &str) -> OpTaskRunStatus {
    match value {
        "Pending" | "pending" => OpTaskRunStatus::Pending,
        "Running" | "running" => OpTaskRunStatus::Running,
        "Succeeded" | "succeeded" => OpTaskRunStatus::Succeeded,
        "Failed" | "failed" => OpTaskRunStatus::Failed,
        "Cancelled" | "cancelled" => OpTaskRunStatus::Cancelled,
        _ => OpTaskRunStatus::Pending,
    }
}

trait StatusAsStr {
    fn as_str(&self) -> &'static str;
}

impl StatusAsStr for OpTaskStatus {
    fn as_str(&self) -> &'static str {
        match self {
            OpTaskStatus::Draft => "Draft",
            OpTaskStatus::Active => "Active",
            OpTaskStatus::Paused => "Paused",
            OpTaskStatus::Archived => "Archived",
        }
    }
}

impl StatusAsStr for OpTaskRunStatus {
    fn as_str(&self) -> &'static str {
        match self {
            OpTaskRunStatus::Pending => "Pending",
            OpTaskRunStatus::Running => "Running",
            OpTaskRunStatus::Succeeded => "Succeeded",
            OpTaskRunStatus::Failed => "Failed",
            OpTaskRunStatus::Cancelled => "Cancelled",
        }
    }
}
