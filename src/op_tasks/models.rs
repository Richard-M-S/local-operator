use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A saved task definition that can be executed later.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct OpTask {
    pub id: Uuid,
    pub task_type: String,
    pub name: String,
    pub description: Option<String>,
    pub status: OpTaskStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: Option<DateTime<Utc>>,
}

/// One execution instance of an `OpTask`.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct OpTaskRun {
    pub id: Uuid,
    pub task_id: Uuid,
    pub status: OpTaskRunStatus,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub work_items: Vec<OpWorkItem>,
    pub artifacts: Vec<TaskArtifact>,
    pub summary: Option<String>,
}

/// One step inside a task run.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct OpWorkItem {
    pub id: Uuid,
    pub run_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub order: u32,
    pub status: OpTaskRunStatus,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub details: Option<String>,
}

/// Output or artifact created during a task run.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct TaskArtifact {
    pub id: Uuid,
    pub run_id: Uuid,
    pub work_item_id: Option<Uuid>,
    pub name: String,
    pub artifact_type: String,
    pub location: Option<String>,
    pub created_at: DateTime<Utc>,
    pub metadata: Option<serde_json::Value>,
}

/// Lifecycle state for a saved task definition.
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub enum OpTaskStatus {
    Draft,
    Active,
    Paused,
    Archived,
}

/// Lifecycle state for a task execution run.
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub enum OpTaskRunStatus {
    Pending,
    Running,
    Succeeded,
    Failed,
    Cancelled,
}
