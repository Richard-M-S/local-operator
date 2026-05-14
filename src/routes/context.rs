use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    app_state::AppState,
    context::models::{ContextKind, SavedContext},
    domains::employment::models::default_employment_profile_id,
    error::AppError,
};

#[derive(Debug, Deserialize)]
pub struct CreateContextRequest {
    pub kind: ContextKind,
    pub title: String,
    pub body: String,
    pub source_url: Option<String>,
    pub source_artifact_id: Option<Uuid>,
    #[serde(default)]
    pub tags: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct ListContextQuery {
    pub kind: Option<ContextKind>,
    pub tag: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct SearchContextQuery {
    #[serde(default)]
    pub q: String,
    pub kind: Option<ContextKind>,
}

#[derive(Serialize)]
pub struct ContextResponse {
    pub context: SavedContext,
}

#[derive(Serialize)]
pub struct ListContextResponse {
    pub contexts: Vec<SavedContext>,
}

pub async fn create(
    State(state): State<AppState>,
    Json(req): Json<CreateContextRequest>,
) -> Result<Json<ContextResponse>, AppError> {
    create_for_profile_id(state, default_employment_profile_id(), req).await
}

pub async fn create_for_profile(
    State(state): State<AppState>,
    Path(profile_id): Path<Uuid>,
    Json(req): Json<CreateContextRequest>,
) -> Result<Json<ContextResponse>, AppError> {
    create_for_profile_id(state, profile_id, req).await
}

async fn create_for_profile_id(
    state: AppState,
    profile_id: Uuid,
    req: CreateContextRequest,
) -> Result<Json<ContextResponse>, AppError> {
    let title = req.title.trim().to_string();
    if title.is_empty() {
        return Err(AppError::BadRequest(
            "context title cannot be empty".to_string(),
        ));
    }

    if req.body.trim().is_empty() {
        return Err(AppError::BadRequest(
            "context body cannot be empty".to_string(),
        ));
    }

    let tags = clean_tags(req.tags);
    let source_url = req
        .source_url
        .map(|url| url.trim().to_string())
        .filter(|url| !url.is_empty());

    let context = state
        .context
        .save_context_note(
            profile_id,
            req.kind,
            title,
            req.body,
            source_url,
            req.source_artifact_id,
            tags,
        )
        .await
        .map_err(|err| AppError::Internal(err.to_string()))?;

    Ok(Json(ContextResponse { context }))
}

pub async fn list(
    State(state): State<AppState>,
    Query(query): Query<ListContextQuery>,
) -> Result<Json<ListContextResponse>, AppError> {
    list_for_profile_id(state, default_employment_profile_id(), query).await
}

pub async fn list_for_profile(
    State(state): State<AppState>,
    Path(profile_id): Path<Uuid>,
    Query(query): Query<ListContextQuery>,
) -> Result<Json<ListContextResponse>, AppError> {
    list_for_profile_id(state, profile_id, query).await
}

async fn list_for_profile_id(
    state: AppState,
    profile_id: Uuid,
    query: ListContextQuery,
) -> Result<Json<ListContextResponse>, AppError> {
    let mut contexts = state
        .context
        .get_relevant_context(profile_id, "", query.kind)
        .await
        .map_err(|err| AppError::Internal(err.to_string()))?;

    if let Some(tag) = query
        .tag
        .as_ref()
        .map(|tag| tag.trim())
        .filter(|tag| !tag.is_empty())
    {
        contexts.retain(|context| context.tags.iter().any(|item| item == tag));
    }

    let contexts = paginate(contexts, query.limit, query.offset);

    Ok(Json(ListContextResponse { contexts }))
}

pub async fn get(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<ContextResponse>, AppError> {
    let context = state
        .context
        .get_context(id)
        .await
        .map_err(|err| AppError::Internal(err.to_string()))?
        .ok_or_else(|| AppError::NotFound(format!("context {} not found", id)))?;

    Ok(Json(ContextResponse { context }))
}

pub async fn search(
    State(state): State<AppState>,
    Query(query): Query<SearchContextQuery>,
) -> Result<Json<ListContextResponse>, AppError> {
    search_for_profile_id(state, default_employment_profile_id(), query).await
}

pub async fn search_for_profile(
    State(state): State<AppState>,
    Path(profile_id): Path<Uuid>,
    Query(query): Query<SearchContextQuery>,
) -> Result<Json<ListContextResponse>, AppError> {
    search_for_profile_id(state, profile_id, query).await
}

async fn search_for_profile_id(
    state: AppState,
    profile_id: Uuid,
    query: SearchContextQuery,
) -> Result<Json<ListContextResponse>, AppError> {
    let contexts = state
        .context
        .get_relevant_context(profile_id, query.q.trim(), query.kind)
        .await
        .map_err(|err| AppError::Internal(err.to_string()))?;

    Ok(Json(ListContextResponse { contexts }))
}

fn clean_tags(tags: Vec<String>) -> Vec<String> {
    tags.into_iter()
        .map(|tag| tag.trim().to_string())
        .filter(|tag| !tag.is_empty())
        .collect()
}

fn paginate<T>(items: Vec<T>, limit: Option<i64>, offset: Option<i64>) -> Vec<T> {
    let limit = limit.unwrap_or(50).clamp(1, 200) as usize;
    let offset = offset.unwrap_or(0).max(0) as usize;

    items.into_iter().skip(offset).take(limit).collect()
}
