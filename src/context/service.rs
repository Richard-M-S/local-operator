use crate::context::models::{ContextKind, SavedContext};
use crate::context::repository::ContextRepository;
use anyhow::{anyhow, Result};
use chrono::Utc;
use uuid::Uuid;

#[derive(Clone)]
pub struct ContextService {
    repo: ContextRepository,
}

impl ContextService {
    pub fn new(repo: ContextRepository) -> Self {
        Self { repo }
    }

    pub async fn save_context_note(
        &self,
        kind: ContextKind,
        title: String,
        body: String,
        source_url: Option<String>,
        source_artifact_id: Option<Uuid>,
        tags: Vec<String>,
    ) -> Result<SavedContext> {
        let now = Utc::now();
        let context = SavedContext {
            id: Uuid::new_v4(),
            kind,
            title,
            body,
            source_url,
            source_artifact_id,
            tags,
            created_at: now,
            updated_at: None,
        };

        self.repo.create_context(context).await
    }

    pub async fn get_relevant_context(
        &self,
        query: &str,
        kind: Option<ContextKind>,
    ) -> Result<Vec<SavedContext>> {
        let mut results = if query.is_empty() {
            self.repo.list_contexts().await?
        } else {
            self.repo.search_context_basic(query).await?
        };

        if let Some(kind) = kind {
            results.retain(|item| item.kind == kind);
        }

        Ok(results)
    }

    pub async fn get_context(&self, context_id: Uuid) -> Result<Option<SavedContext>> {
        self.repo.get_context(context_id).await
    }

    #[allow(dead_code)]
    pub async fn list_context_by_kind(&self, kind: ContextKind) -> Result<Vec<SavedContext>> {
        let mut contexts = self.repo.list_contexts().await?;
        contexts.retain(|item| item.kind == kind);
        Ok(contexts)
    }

    #[allow(dead_code)]
    pub async fn attach_context_to_artifact(
        &self,
        context_id: Uuid,
        artifact_id: Uuid,
    ) -> Result<SavedContext> {
        let mut context = self
            .repo
            .get_context(context_id)
            .await?
            .ok_or_else(|| anyhow!("context not found: {}", context_id))?;

        context.source_artifact_id = Some(artifact_id);
        context.updated_at = Some(Utc::now());

        self.repo.update_context(context.clone()).await
    }
}
