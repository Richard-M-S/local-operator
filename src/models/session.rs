use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A user request for a task execution, tracking the conversation/request lifecycle
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct TaskRequest {
    pub id: Uuid,
    pub profile_id: Uuid,
    pub source: String,
    pub user_request: String,
    pub intent: Option<String>,
    pub status: String,
    pub op_task_id: Option<Uuid>,
    pub run_id: Option<Uuid>,
    pub primary_artifact_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl TaskRequest {
    pub fn new(profile_id: Uuid, source: String, user_request: String) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            profile_id,
            source,
            user_request,
            intent: None,
            status: "pending".to_string(),
            op_task_id: None,
            run_id: None,
            primary_artifact_id: None,
            created_at: now,
            updated_at: now,
        }
    }
}

/// A conversation session, typically representing a multi-turn interaction
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ChatSession {
    pub id: Uuid,
    pub profile_id: Uuid,
    pub external_source: Option<String>,
    pub external_conversation_id: Option<String>,
    pub last_task_request_id: Option<Uuid>,
    pub last_run_id: Option<Uuid>,
    pub last_artifact_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl ChatSession {
    pub fn new(profile_id: Uuid) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            profile_id,
            external_source: None,
            external_conversation_id: None,
            last_task_request_id: None,
            last_run_id: None,
            last_artifact_id: None,
            created_at: now,
            updated_at: now,
        }
    }

    pub fn with_external_source(
        profile_id: Uuid,
        external_source: String,
        external_conversation_id: String,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            profile_id,
            external_source: Some(external_source),
            external_conversation_id: Some(external_conversation_id),
            last_task_request_id: None,
            last_run_id: None,
            last_artifact_id: None,
            created_at: now,
            updated_at: now,
        }
    }
}

/// A single message within a chat session
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ChatMessage {
    pub id: Uuid,
    pub session_id: Uuid,
    pub role: String,
    pub content: String,
    pub task_request_id: Option<Uuid>,
    pub run_id: Option<Uuid>,
    pub artifact_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
}

impl ChatMessage {
    pub fn new(session_id: Uuid, role: String, content: String) -> Self {
        Self {
            id: Uuid::new_v4(),
            session_id,
            role,
            content,
            task_request_id: None,
            run_id: None,
            artifact_id: None,
            created_at: Utc::now(),
        }
    }
}

/// A link between two entities (e.g., task request to op task, message to artifact)
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct TaskLink {
    pub id: Uuid,
    pub source_type: String,
    pub source_id: Uuid,
    pub target_type: String,
    pub target_id: Uuid,
    pub relationship: String,
    pub created_at: DateTime<Utc>,
}

impl TaskLink {
    pub fn new(
        source_type: String,
        source_id: Uuid,
        target_type: String,
        target_id: Uuid,
        relationship: String,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            source_type,
            source_id,
            target_type,
            target_id,
            relationship,
            created_at: Utc::now(),
        }
    }
}
