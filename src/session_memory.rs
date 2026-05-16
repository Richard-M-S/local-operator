use chrono::Utc;
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::models::session::{ChatMessage, ChatSession, TaskLink, TaskRequest};

#[derive(Clone)]
pub struct SessionMemoryRepository {
    pool: SqlitePool,
}

impl SessionMemoryRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn create_task_request(
        &self,
        mut request: TaskRequest,
    ) -> anyhow::Result<TaskRequest> {
        request.updated_at = Utc::now();

        sqlx::query(
            r#"
            INSERT INTO task_requests (
                id, profile_id, source, user_request, intent, status, op_task_id, run_id,
                primary_artifact_id, created_at, updated_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
            "#,
        )
        .bind(request.id.to_string())
        .bind(request.profile_id.to_string())
        .bind(&request.source)
        .bind(&request.user_request)
        .bind(&request.intent)
        .bind(&request.status)
        .bind(request.op_task_id.map(|id| id.to_string()))
        .bind(request.run_id.map(|id| id.to_string()))
        .bind(request.primary_artifact_id.map(|id| id.to_string()))
        .bind(request.created_at.to_rfc3339())
        .bind(request.updated_at.to_rfc3339())
        .execute(&self.pool)
        .await?;

        Ok(request)
    }

    pub async fn get_task_request(&self, request_id: Uuid) -> anyhow::Result<Option<TaskRequest>> {
        let row = sqlx::query_as::<_, TaskRequestRow>(
            r#"
            SELECT id, profile_id, source, user_request, intent, status, op_task_id, run_id,
                   primary_artifact_id, created_at, updated_at
            FROM task_requests
            WHERE id = ?1
            "#,
        )
        .bind(request_id.to_string())
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(Into::into))
    }

    pub async fn update_task_request(
        &self,
        request_id: Uuid,
        status: &str,
        op_task_id: Option<Uuid>,
        run_id: Option<Uuid>,
        primary_artifact_id: Option<Uuid>,
    ) -> anyhow::Result<()> {
        sqlx::query(
            r#"
            UPDATE task_requests
            SET status = ?1,
                op_task_id = COALESCE(?2, op_task_id),
                run_id = COALESCE(?3, run_id),
                primary_artifact_id = COALESCE(?4, primary_artifact_id),
                updated_at = ?5
            WHERE id = ?6
            "#,
        )
        .bind(status)
        .bind(op_task_id.map(|id| id.to_string()))
        .bind(run_id.map(|id| id.to_string()))
        .bind(primary_artifact_id.map(|id| id.to_string()))
        .bind(Utc::now().to_rfc3339())
        .bind(request_id.to_string())
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn create_chat_session(&self, session: ChatSession) -> anyhow::Result<ChatSession> {
        sqlx::query(
            r#"
            INSERT INTO chat_sessions (
                id, profile_id, external_source, external_conversation_id, last_task_request_id,
                last_run_id, last_artifact_id, created_at, updated_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
            "#,
        )
        .bind(session.id.to_string())
        .bind(session.profile_id.to_string())
        .bind(&session.external_source)
        .bind(&session.external_conversation_id)
        .bind(session.last_task_request_id.map(|id| id.to_string()))
        .bind(session.last_run_id.map(|id| id.to_string()))
        .bind(session.last_artifact_id.map(|id| id.to_string()))
        .bind(session.created_at.to_rfc3339())
        .bind(session.updated_at.to_rfc3339())
        .execute(&self.pool)
        .await?;

        Ok(session)
    }

    pub async fn get_chat_session(&self, session_id: Uuid) -> anyhow::Result<Option<ChatSession>> {
        let row = sqlx::query_as::<_, ChatSessionRow>(
            r#"
            SELECT id, profile_id, external_source, external_conversation_id,
                   last_task_request_id, last_run_id, last_artifact_id, created_at, updated_at
            FROM chat_sessions
            WHERE id = ?1
            "#,
        )
        .bind(session_id.to_string())
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(Into::into))
    }

    pub async fn find_chat_session_by_external(
        &self,
        external_source: &str,
        external_conversation_id: &str,
    ) -> anyhow::Result<Option<ChatSession>> {
        let row = sqlx::query_as::<_, ChatSessionRow>(
            r#"
            SELECT id, profile_id, external_source, external_conversation_id,
                   last_task_request_id, last_run_id, last_artifact_id, created_at, updated_at
            FROM chat_sessions
            WHERE external_source = ?1
              AND external_conversation_id = ?2
            ORDER BY updated_at DESC
            LIMIT 1
            "#,
        )
        .bind(external_source)
        .bind(external_conversation_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(Into::into))
    }

    pub async fn get_or_create_external_chat_session(
        &self,
        profile_id: Uuid,
        external_source: &str,
        external_conversation_id: &str,
    ) -> anyhow::Result<ChatSession> {
        if let Some(session) = self
            .find_chat_session_by_external(external_source, external_conversation_id)
            .await?
        {
            return Ok(session);
        }

        self.create_chat_session(ChatSession::with_external_source(
            profile_id,
            external_source.to_string(),
            external_conversation_id.to_string(),
        ))
        .await
    }

    pub async fn update_chat_session_memory(
        &self,
        session_id: Uuid,
        task_request_id: Option<Uuid>,
        run_id: Option<Uuid>,
        artifact_id: Option<Uuid>,
    ) -> anyhow::Result<()> {
        sqlx::query(
            r#"
            UPDATE chat_sessions
            SET last_task_request_id = COALESCE(?1, last_task_request_id),
                last_run_id = COALESCE(?2, last_run_id),
                last_artifact_id = COALESCE(?3, last_artifact_id),
                updated_at = ?4
            WHERE id = ?5
            "#,
        )
        .bind(task_request_id.map(|id| id.to_string()))
        .bind(run_id.map(|id| id.to_string()))
        .bind(artifact_id.map(|id| id.to_string()))
        .bind(Utc::now().to_rfc3339())
        .bind(session_id.to_string())
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn create_chat_message(&self, message: ChatMessage) -> anyhow::Result<ChatMessage> {
        sqlx::query(
            r#"
            INSERT INTO chat_messages (
                id, session_id, role, content, task_request_id, run_id, artifact_id, created_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
            "#,
        )
        .bind(message.id.to_string())
        .bind(message.session_id.to_string())
        .bind(&message.role)
        .bind(&message.content)
        .bind(message.task_request_id.map(|id| id.to_string()))
        .bind(message.run_id.map(|id| id.to_string()))
        .bind(message.artifact_id.map(|id| id.to_string()))
        .bind(message.created_at.to_rfc3339())
        .execute(&self.pool)
        .await?;

        Ok(message)
    }

    pub async fn list_chat_messages(
        &self,
        session_id: Uuid,
        limit: i64,
    ) -> anyhow::Result<Vec<ChatMessage>> {
        let rows = sqlx::query_as::<_, ChatMessageRow>(
            r#"
            SELECT id, session_id, role, content, task_request_id, run_id, artifact_id, created_at
            FROM chat_messages
            WHERE session_id = ?1
            ORDER BY created_at DESC
            LIMIT ?2
            "#,
        )
        .bind(session_id.to_string())
        .bind(limit.clamp(1, 200))
        .fetch_all(&self.pool)
        .await?;

        let mut messages = rows.into_iter().map(Into::into).collect::<Vec<_>>();
        messages.reverse();
        Ok(messages)
    }

    pub async fn last_artifact_id_for_session(
        &self,
        session_id: Uuid,
    ) -> anyhow::Result<Option<Uuid>> {
        let row = sqlx::query_as::<_, ArtifactIdRow>(
            r#"
            SELECT COALESCE(
                (SELECT last_artifact_id FROM chat_sessions WHERE id = ?1),
                (
                    SELECT artifact_id
                    FROM chat_messages
                    WHERE session_id = ?1
                      AND artifact_id IS NOT NULL
                    ORDER BY created_at DESC
                    LIMIT 1
                )
            ) AS artifact_id
            "#,
        )
        .bind(session_id.to_string())
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.and_then(|row| row.artifact_id.and_then(|id| Uuid::parse_str(&id).ok())))
    }

    pub async fn create_task_link(&self, link: TaskLink) -> anyhow::Result<TaskLink> {
        sqlx::query(
            r#"
            INSERT INTO task_links (
                id, source_type, source_id, target_type, target_id, relationship, created_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
            "#,
        )
        .bind(link.id.to_string())
        .bind(&link.source_type)
        .bind(link.source_id.to_string())
        .bind(&link.target_type)
        .bind(link.target_id.to_string())
        .bind(&link.relationship)
        .bind(link.created_at.to_rfc3339())
        .execute(&self.pool)
        .await?;

        Ok(link)
    }
}

#[derive(sqlx::FromRow)]
struct TaskRequestRow {
    id: String,
    profile_id: String,
    source: String,
    user_request: String,
    intent: Option<String>,
    status: String,
    op_task_id: Option<String>,
    run_id: Option<String>,
    primary_artifact_id: Option<String>,
    created_at: String,
    updated_at: String,
}

impl From<TaskRequestRow> for TaskRequest {
    fn from(row: TaskRequestRow) -> Self {
        Self {
            id: Uuid::parse_str(&row.id).unwrap(),
            profile_id: Uuid::parse_str(&row.profile_id).unwrap(),
            source: row.source,
            user_request: row.user_request,
            intent: row.intent,
            status: row.status,
            op_task_id: row.op_task_id.and_then(|id| Uuid::parse_str(&id).ok()),
            run_id: row.run_id.and_then(|id| Uuid::parse_str(&id).ok()),
            primary_artifact_id: row
                .primary_artifact_id
                .and_then(|id| Uuid::parse_str(&id).ok()),
            created_at: row.created_at.parse().unwrap_or_else(|_| Utc::now()),
            updated_at: row.updated_at.parse().unwrap_or_else(|_| Utc::now()),
        }
    }
}

#[derive(sqlx::FromRow)]
struct ChatSessionRow {
    id: String,
    profile_id: String,
    external_source: Option<String>,
    external_conversation_id: Option<String>,
    last_task_request_id: Option<String>,
    last_run_id: Option<String>,
    last_artifact_id: Option<String>,
    created_at: String,
    updated_at: String,
}

impl From<ChatSessionRow> for ChatSession {
    fn from(row: ChatSessionRow) -> Self {
        Self {
            id: Uuid::parse_str(&row.id).unwrap(),
            profile_id: Uuid::parse_str(&row.profile_id).unwrap(),
            external_source: row.external_source,
            external_conversation_id: row.external_conversation_id,
            last_task_request_id: row
                .last_task_request_id
                .and_then(|id| Uuid::parse_str(&id).ok()),
            last_run_id: row.last_run_id.and_then(|id| Uuid::parse_str(&id).ok()),
            last_artifact_id: row
                .last_artifact_id
                .and_then(|id| Uuid::parse_str(&id).ok()),
            created_at: row.created_at.parse().unwrap_or_else(|_| Utc::now()),
            updated_at: row.updated_at.parse().unwrap_or_else(|_| Utc::now()),
        }
    }
}

#[derive(sqlx::FromRow)]
struct ChatMessageRow {
    id: String,
    session_id: String,
    role: String,
    content: String,
    task_request_id: Option<String>,
    run_id: Option<String>,
    artifact_id: Option<String>,
    created_at: String,
}

impl From<ChatMessageRow> for ChatMessage {
    fn from(row: ChatMessageRow) -> Self {
        Self {
            id: Uuid::parse_str(&row.id).unwrap(),
            session_id: Uuid::parse_str(&row.session_id).unwrap(),
            role: row.role,
            content: row.content,
            task_request_id: row.task_request_id.and_then(|id| Uuid::parse_str(&id).ok()),
            run_id: row.run_id.and_then(|id| Uuid::parse_str(&id).ok()),
            artifact_id: row.artifact_id.and_then(|id| Uuid::parse_str(&id).ok()),
            created_at: row.created_at.parse().unwrap_or_else(|_| Utc::now()),
        }
    }
}

#[derive(sqlx::FromRow)]
struct ArtifactIdRow {
    artifact_id: Option<String>,
}
