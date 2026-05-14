use crate::context::models::{ContextKind, SavedContext};
use sqlx::{FromRow, SqlitePool};
use uuid::Uuid;

#[derive(Clone)]
pub struct ContextRepository {
    pool: SqlitePool,
}

impl ContextRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn create_context(&self, context: SavedContext) -> anyhow::Result<SavedContext> {
        sqlx::query(
            r#"
            INSERT INTO saved_contexts (
                id, profile_id, kind, title, body, source_url, source_artifact_id, tags_json, created_at, updated_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
            "#,
        )
        .bind(context.id.to_string())
        .bind(context.profile_id.to_string())
        .bind(context.kind.as_str())
        .bind(&context.title)
        .bind(&context.body)
        .bind(&context.source_url)
        .bind(context.source_artifact_id.map(|id| id.to_string()))
        .bind(serde_json::to_string(&context.tags)?)
        .bind(context.created_at.to_rfc3339())
        .bind(context.updated_at.map(|dt| dt.to_rfc3339()))
        .execute(&self.pool)
        .await?;

        Ok(context)
    }

    pub async fn get_context(&self, context_id: Uuid) -> anyhow::Result<Option<SavedContext>> {
        let row = sqlx::query_as::<_, SavedContextRow>(
            r#"
            SELECT id, profile_id, kind, title, body, source_url, source_artifact_id, tags_json, created_at, updated_at
            FROM saved_contexts
            WHERE id = ?1
            "#,
        )
        .bind(context_id.to_string())
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(Into::into))
    }

    pub async fn list_contexts(&self, profile_id: Uuid) -> anyhow::Result<Vec<SavedContext>> {
        let rows = sqlx::query_as::<_, SavedContextRow>(
            r#"
            SELECT id, profile_id, kind, title, body, source_url, source_artifact_id, tags_json, created_at, updated_at
            FROM saved_contexts
            WHERE profile_id = ?1
            ORDER BY created_at DESC
            "#,
        )
        .bind(profile_id.to_string())
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(Into::into).collect())
    }

    pub async fn search_context_basic(
        &self,
        profile_id: Uuid,
        query: &str,
    ) -> anyhow::Result<Vec<SavedContext>> {
        let pattern = format!("%{}%", query);
        let rows = sqlx::query_as::<_, SavedContextRow>(
            r#"
            SELECT id, profile_id, kind, title, body, source_url, source_artifact_id, tags_json, created_at, updated_at
            FROM saved_contexts
            WHERE profile_id = ?1
              AND (title LIKE ?2 OR body LIKE ?2 OR tags_json LIKE ?2)
            ORDER BY created_at DESC
            "#,
        )
        .bind(profile_id.to_string())
        .bind(pattern)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(Into::into).collect())
    }

    #[allow(dead_code)]
    pub async fn update_context(&self, context: SavedContext) -> anyhow::Result<SavedContext> {
        sqlx::query(
            r#"
            UPDATE saved_contexts
            SET profile_id = ?1,
                kind = ?2,
                title = ?3,
                body = ?4,
                source_url = ?5,
                source_artifact_id = ?6,
                tags_json = ?7,
                updated_at = ?8
            WHERE id = ?9
            "#,
        )
        .bind(context.profile_id.to_string())
        .bind(context.kind.as_str())
        .bind(&context.title)
        .bind(&context.body)
        .bind(&context.source_url)
        .bind(context.source_artifact_id.map(|id| id.to_string()))
        .bind(serde_json::to_string(&context.tags)?)
        .bind(context.updated_at.map(|dt| dt.to_rfc3339()))
        .bind(context.id.to_string())
        .execute(&self.pool)
        .await?;

        Ok(context)
    }

    #[allow(dead_code)]
    pub async fn delete_context(&self, context_id: Uuid) -> anyhow::Result<()> {
        sqlx::query(
            r#"
            DELETE FROM saved_contexts
            WHERE id = ?1
            "#,
        )
        .bind(context_id.to_string())
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}

#[derive(FromRow)]
struct SavedContextRow {
    id: String,
    profile_id: String,
    kind: String,
    title: String,
    body: String,
    source_url: Option<String>,
    source_artifact_id: Option<String>,
    tags_json: String,
    created_at: String,
    updated_at: Option<String>,
}

impl From<SavedContextRow> for SavedContext {
    fn from(row: SavedContextRow) -> Self {
        let tags = serde_json::from_str(&row.tags_json).unwrap_or_default();

        Self {
            id: Uuid::parse_str(&row.id).unwrap(),
            profile_id: Uuid::parse_str(&row.profile_id).unwrap(),
            kind: parse_context_kind(&row.kind),
            title: row.title,
            body: row.body,
            source_url: row.source_url,
            source_artifact_id: row
                .source_artifact_id
                .and_then(|id| Uuid::parse_str(&id).ok()),
            tags,
            created_at: row.created_at.parse().unwrap(),
            updated_at: row.updated_at.and_then(|value| value.parse().ok()),
        }
    }
}

fn parse_context_kind(value: &str) -> ContextKind {
    match value {
        "career_profile" => ContextKind::CareerProfile,
        "resume_fact" => ContextKind::ResumeFact,
        "project_summary" => ContextKind::ProjectSummary,
        "writing_preference" => ContextKind::WritingPreference,
        "home_assistant_note" => ContextKind::HomeAssistantNote,
        "employment_preference" => ContextKind::EmploymentPreference,
        "document_note" => ContextKind::DocumentNote,
        other => ContextKind::Other(other.to_string()),
    }
}

trait ContextKindAsStr {
    fn as_str(&self) -> String;
}

impl ContextKindAsStr for ContextKind {
    fn as_str(&self) -> String {
        match self {
            ContextKind::CareerProfile => "career_profile".to_string(),
            ContextKind::ResumeFact => "resume_fact".to_string(),
            ContextKind::ProjectSummary => "project_summary".to_string(),
            ContextKind::WritingPreference => "writing_preference".to_string(),
            ContextKind::HomeAssistantNote => "home_assistant_note".to_string(),
            ContextKind::EmploymentPreference => "employment_preference".to_string(),
            ContextKind::DocumentNote => "document_note".to_string(),
            ContextKind::Other(value) => value.clone(),
        }
    }
}
