use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ContextKind {
    CareerProfile,
    ResumeFact,
    ProjectSummary,
    WritingPreference,
    HomeAssistantNote,
    EmploymentPreference,
    DocumentNote,
    Other(String),
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[allow(dead_code)]
pub enum ContextSource {
    Url(String),
    Artifact(Uuid),
    Inline,
    Other(String),
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SavedContext {
    pub id: Uuid,
    pub profile_id: Uuid,
    pub kind: ContextKind,
    pub title: String,
    pub body: String,
    pub source_url: Option<String>,
    pub source_artifact_id: Option<Uuid>,
    pub tags: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: Option<DateTime<Utc>>,
}

impl SavedContext {
    #[allow(dead_code)]
    pub fn source(&self) -> Option<ContextSource> {
        if let Some(id) = self.source_artifact_id {
            return Some(ContextSource::Artifact(id));
        }

        if let Some(url) = &self.source_url {
            return Some(ContextSource::Url(url.clone()));
        }

        None
    }
}
