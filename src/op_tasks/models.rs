use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::context::models::ContextKind;

/// A saved task definition that can be executed later.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct OpTask {
    pub id: Uuid,
    pub profile_id: Uuid,
    pub task_type: String,
    pub name: String,
    pub description: Option<String>,
    pub input_json: serde_json::Value,
    pub status: OpTaskStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: Option<DateTime<Utc>>,
}

/// One execution instance of an `OpTask`.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct OpTaskRun {
    pub id: Uuid,
    pub profile_id: Uuid,
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
    pub profile_id: Uuid,
    pub run_id: Uuid,
    pub work_item_id: Option<Uuid>,
    pub name: String,
    pub artifact_type: String,
    pub location: Option<String>,
    pub created_at: DateTime<Utc>,
    pub metadata: Option<serde_json::Value>,
    pub content_text: Option<String>,
    pub content_json: Option<serde_json::Value>,
}

#[derive(Clone, Debug, Default)]
pub struct ArtifactSearch {
    pub profile_id: Option<Uuid>,
    pub run_id: Option<Uuid>,
    pub task_id: Option<Uuid>,
    pub artifact_type: Option<String>,
    pub source_url: Option<String>,
    pub include_content: Option<bool>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct ReadUrlInput {
    pub url: String,
}

#[derive(Debug, Deserialize)]
pub struct SearchWebInput {
    pub query: String,
    pub limit: Option<usize>,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactContextBodySource {
    ContentText,
    ContentJson,
    Metadata,
}

impl Default for ArtifactContextBodySource {
    fn default() -> Self {
        Self::ContentText
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct PromoteArtifactToContextRequest {
    pub kind: ContextKind,
    pub title: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub body_source: ArtifactContextBodySource,
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
