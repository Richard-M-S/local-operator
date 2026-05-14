use axum::{
    extract::{Path, Query, State},
    Json,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use crate::{
    app_state::AppState,
    domains::employment::models::{
        EmploymentOpportunity, EmploymentOpportunitySearch, EmploymentOpportunityStatus,
    },
    error::AppError,
};

#[derive(Debug, Deserialize)]
pub struct CreateEmploymentOpportunityRequest {
    pub source_url: String,
    pub source_name: Option<String>,
    pub title: Option<String>,
    pub company: Option<String>,
    pub location: Option<String>,
    pub remote_type: Option<String>,
    pub salary_min: Option<i64>,
    pub salary_max: Option<i64>,
    pub description_text: Option<String>,
    pub extracted_json: Option<Value>,
    pub fit_score: Option<i64>,
    pub status: Option<EmploymentOpportunityStatus>,
    pub skip_reason: Option<String>,
    pub source_artifact_id: Option<Uuid>,
}

#[derive(Debug, Deserialize)]
pub struct ListEmploymentOpportunitiesQuery {
    pub status: Option<EmploymentOpportunityStatus>,
    pub company: Option<String>,
    pub title: Option<String>,
    pub remote_type: Option<String>,
    pub min_fit_score: Option<i64>,
    pub source_url: Option<String>,
    pub source_artifact_id: Option<Uuid>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct StatusUpdateRequest {
    pub reason: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct EmploymentOpportunityResponse {
    pub opportunity: EmploymentOpportunity,
}

#[derive(Debug, Serialize)]
pub struct EmploymentOpportunityListResponse {
    pub opportunities: Vec<EmploymentOpportunity>,
}

pub async fn create_opportunity(
    State(state): State<AppState>,
    Json(req): Json<CreateEmploymentOpportunityRequest>,
) -> Result<Json<EmploymentOpportunityResponse>, AppError> {
    let now = Utc::now();

    let opportunity = EmploymentOpportunity {
        id: Uuid::new_v4(),
        source_url: req.source_url,
        source_name: req.source_name,
        title: req.title,
        company: req.company,
        location: req.location,
        remote_type: req.remote_type,
        salary_min: req.salary_min,
        salary_max: req.salary_max,
        description_text: req.description_text,
        extracted_json: req.extracted_json,
        fit_score: req.fit_score,
        status: req
            .status
            .unwrap_or(EmploymentOpportunityStatus::Discovered),
        skip_reason: req.skip_reason,
        source_artifact_id: req.source_artifact_id,
        first_seen_at: now,
        last_seen_at: now,
    };

    let opportunity = state.employment.create_opportunity(opportunity).await?;

    Ok(Json(EmploymentOpportunityResponse { opportunity }))
}

pub async fn list_opportunities(
    State(state): State<AppState>,
    Query(query): Query<ListEmploymentOpportunitiesQuery>,
) -> Result<Json<EmploymentOpportunityListResponse>, AppError> {
    let search = EmploymentOpportunitySearch {
        status: query.status,
        company: query.company,
        title: query.title,
        remote_type: query.remote_type,
        min_fit_score: query.min_fit_score,
        source_url: query.source_url,
        source_artifact_id: query.source_artifact_id,
        limit: query.limit,
        offset: query.offset,
    };

    let opportunities = state.employment.list_opportunities(search).await?;

    Ok(Json(EmploymentOpportunityListResponse { opportunities }))
}

pub async fn get_opportunity(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<EmploymentOpportunityResponse>, AppError> {
    let opportunity = state
        .employment
        .get_opportunity(id)
        .await?
        .ok_or_else(|| AppError::NotFound("Employment opportunity not found".to_string()))?;

    Ok(Json(EmploymentOpportunityResponse { opportunity }))
}

pub async fn create_opportunity_from_artifact(
    State(state): State<AppState>,
    Path(artifact_id): Path<Uuid>,
) -> Result<Json<EmploymentOpportunityResponse>, AppError> {
    let opportunity = state.employment.create_from_artifact(artifact_id).await?;
    Ok(Json(EmploymentOpportunityResponse { opportunity }))
}

pub async fn parse_opportunity(
    State(state): State<AppState>,
    Path(opportunity_id): Path<Uuid>,
) -> Result<Json<EmploymentOpportunityResponse>, AppError> {
    let opportunity = state.employment.parse_opportunity(opportunity_id).await?;
    Ok(Json(EmploymentOpportunityResponse { opportunity }))
}

pub async fn score_opportunity(
    State(state): State<AppState>,
    Path(opportunity_id): Path<Uuid>,
) -> Result<Json<EmploymentOpportunityResponse>, AppError> {
    let opportunity = state.employment.score_opportunity(opportunity_id).await?;
    Ok(Json(EmploymentOpportunityResponse { opportunity }))
}

pub async fn archive_opportunity(
    State(state): State<AppState>,
    Path(opportunity_id): Path<Uuid>,
    Json(req): Json<StatusUpdateRequest>,
) -> Result<Json<EmploymentOpportunityResponse>, AppError> {
    let opportunity = state
        .employment
        .repository
        .update_opportunity_status(
            opportunity_id,
            EmploymentOpportunityStatus::Archived,
            req.reason,
        )
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?
        .ok_or_else(|| AppError::NotFound("opportunity not found".to_string()))?;

    Ok(Json(EmploymentOpportunityResponse { opportunity }))
}

pub async fn reject_opportunity(
    State(state): State<AppState>,
    Path(opportunity_id): Path<Uuid>,
    Json(req): Json<StatusUpdateRequest>,
) -> Result<Json<EmploymentOpportunityResponse>, AppError> {
    let opportunity = state
        .employment
        .repository
        .update_opportunity_status(
            opportunity_id,
            EmploymentOpportunityStatus::Rejected,
            req.reason,
        )
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?
        .ok_or_else(|| AppError::NotFound("opportunity not found".to_string()))?;

    Ok(Json(EmploymentOpportunityResponse { opportunity }))
}

pub async fn restore_opportunity(
    State(state): State<AppState>,
    Path(opportunity_id): Path<Uuid>,
    Json(req): Json<StatusUpdateRequest>,
) -> Result<Json<EmploymentOpportunityResponse>, AppError> {
    let opportunity = state
        .employment
        .repository
        .update_opportunity_status(
            opportunity_id,
            EmploymentOpportunityStatus::QueuedForReview,
            req.reason,
        )
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?
        .ok_or_else(|| AppError::NotFound("opportunity not found".to_string()))?;

    Ok(Json(EmploymentOpportunityResponse { opportunity }))
}
