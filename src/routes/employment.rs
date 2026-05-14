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
        default_employment_profile_id, EmploymentOpportunity, EmploymentOpportunitySearch,
        EmploymentOpportunityStatus, EmploymentProfile,
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
    pub primary_fit_score: Option<i64>,
    pub oe_fit_score: Option<i64>,
    pub recommended_track: Option<String>,
    pub score_reason: Option<String>,
    #[serde(default)]
    pub risk_flags: Vec<String>,
    pub status: Option<EmploymentOpportunityStatus>,
    pub skip_reason: Option<String>,
    pub skip_recommendation: Option<String>,
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

#[derive(Debug, Deserialize)]
pub struct GenerateCoverLetterRequest {
    #[serde(default)]
    pub direction: String,
}

#[derive(Debug, Serialize)]
pub struct CoverLetterResponse {
    pub cover_letter: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateEmploymentProfileRequest {
    pub display_name: String,
    pub email: Option<String>,
    pub notes: Option<String>,
    pub criteria: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateEmploymentProfileRequest {
    pub display_name: Option<String>,
    pub email: Option<String>,
    pub notes: Option<String>,
    pub criteria: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct EmploymentProfileResponse {
    pub profile: EmploymentProfile,
}

#[derive(Debug, Serialize)]
pub struct EmploymentProfileListResponse {
    pub profiles: Vec<EmploymentProfile>,
}

#[derive(Debug, Serialize)]
pub struct EmploymentOpportunityResponse {
    pub opportunity: EmploymentOpportunity,
}

#[derive(Debug, Serialize)]
pub struct EmploymentOpportunityListResponse {
    pub opportunities: Vec<EmploymentOpportunity>,
}

pub async fn list_profiles(
    State(state): State<AppState>,
) -> Result<Json<EmploymentProfileListResponse>, AppError> {
    let profiles = state
        .employment
        .repository
        .list_profiles()
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

    Ok(Json(EmploymentProfileListResponse { profiles }))
}

pub async fn create_profile(
    State(state): State<AppState>,
    Json(req): Json<CreateEmploymentProfileRequest>,
) -> Result<Json<EmploymentProfileResponse>, AppError> {
    let display_name = req.display_name.trim().to_string();
    if display_name.is_empty() {
        return Err(AppError::BadRequest(
            "profile display_name cannot be empty".to_string(),
        ));
    }

    let now = Utc::now();
    let profile = EmploymentProfile {
        id: Uuid::new_v4(),
        display_name,
        email: req
            .email
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty()),
        notes: req
            .notes
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty()),
        criteria: req
            .criteria
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty()),
        created_at: now,
        updated_at: None,
    };

    let profile = state
        .employment
        .repository
        .create_profile(profile)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

    Ok(Json(EmploymentProfileResponse { profile }))
}

pub async fn get_profile(
    State(state): State<AppState>,
    Path(profile_id): Path<Uuid>,
) -> Result<Json<EmploymentProfileResponse>, AppError> {
    let profile = state
        .employment
        .repository
        .get_profile(profile_id)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?
        .ok_or_else(|| AppError::NotFound("Employment profile not found".to_string()))?;

    Ok(Json(EmploymentProfileResponse { profile }))
}

pub async fn update_profile(
    State(state): State<AppState>,
    Path(profile_id): Path<Uuid>,
    Json(req): Json<UpdateEmploymentProfileRequest>,
) -> Result<Json<EmploymentProfileResponse>, AppError> {
    let mut profile = state
        .employment
        .repository
        .get_profile(profile_id)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?
        .ok_or_else(|| AppError::NotFound("Employment profile not found".to_string()))?;

    if let Some(display_name) = req.display_name {
        let display_name = display_name.trim().to_string();
        if display_name.is_empty() {
            return Err(AppError::BadRequest(
                "profile display_name cannot be empty".to_string(),
            ));
        }
        profile.display_name = display_name;
    }

    if let Some(email) = req.email {
        profile.email = clean_optional_text(email);
    }

    if let Some(notes) = req.notes {
        profile.notes = clean_optional_text(notes);
    }

    if let Some(criteria) = req.criteria {
        profile.criteria = clean_optional_text(criteria);
    }

    profile.updated_at = Some(Utc::now());

    let profile = state
        .employment
        .repository
        .update_profile(profile)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

    Ok(Json(EmploymentProfileResponse { profile }))
}

pub async fn create_opportunity(
    State(state): State<AppState>,
    Json(req): Json<CreateEmploymentOpportunityRequest>,
) -> Result<Json<EmploymentOpportunityResponse>, AppError> {
    create_opportunity_for_profile_id(state, default_employment_profile_id(), req).await
}

pub async fn create_opportunity_for_profile(
    State(state): State<AppState>,
    Path(profile_id): Path<Uuid>,
    Json(req): Json<CreateEmploymentOpportunityRequest>,
) -> Result<Json<EmploymentOpportunityResponse>, AppError> {
    create_opportunity_for_profile_id(state, profile_id, req).await
}

async fn create_opportunity_for_profile_id(
    state: AppState,
    profile_id: Uuid,
    req: CreateEmploymentOpportunityRequest,
) -> Result<Json<EmploymentOpportunityResponse>, AppError> {
    ensure_profile_exists(&state, profile_id).await?;
    let now = Utc::now();

    let opportunity = EmploymentOpportunity {
        id: Uuid::new_v4(),
        profile_id,
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
        primary_fit_score: req.primary_fit_score,
        oe_fit_score: req.oe_fit_score,
        recommended_track: req.recommended_track,
        score_reason: req.score_reason,
        risk_flags: req.risk_flags,
        status: req
            .status
            .unwrap_or(EmploymentOpportunityStatus::Discovered),
        skip_reason: req.skip_reason,
        skip_recommendation: req.skip_recommendation,
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
    list_opportunities_for_profile_id(state, default_employment_profile_id(), query).await
}

pub async fn list_opportunities_for_profile(
    State(state): State<AppState>,
    Path(profile_id): Path<Uuid>,
    Query(query): Query<ListEmploymentOpportunitiesQuery>,
) -> Result<Json<EmploymentOpportunityListResponse>, AppError> {
    list_opportunities_for_profile_id(state, profile_id, query).await
}

async fn list_opportunities_for_profile_id(
    state: AppState,
    profile_id: Uuid,
    query: ListEmploymentOpportunitiesQuery,
) -> Result<Json<EmploymentOpportunityListResponse>, AppError> {
    ensure_profile_exists(&state, profile_id).await?;
    let search = EmploymentOpportunitySearch {
        profile_id: Some(profile_id),
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

pub async fn get_opportunity_for_profile(
    State(state): State<AppState>,
    Path((profile_id, id)): Path<(Uuid, Uuid)>,
) -> Result<Json<EmploymentOpportunityResponse>, AppError> {
    let opportunity = get_profile_opportunity(&state, profile_id, id).await?;

    Ok(Json(EmploymentOpportunityResponse { opportunity }))
}

pub async fn create_opportunity_from_artifact(
    State(state): State<AppState>,
    Path(artifact_id): Path<Uuid>,
) -> Result<Json<EmploymentOpportunityResponse>, AppError> {
    let opportunity = state
        .employment
        .create_from_artifact(default_employment_profile_id(), artifact_id)
        .await?;
    Ok(Json(EmploymentOpportunityResponse { opportunity }))
}

pub async fn create_opportunity_from_artifact_for_profile(
    State(state): State<AppState>,
    Path((profile_id, artifact_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<EmploymentOpportunityResponse>, AppError> {
    ensure_profile_exists(&state, profile_id).await?;
    let opportunity = state
        .employment
        .create_from_artifact(profile_id, artifact_id)
        .await?;
    Ok(Json(EmploymentOpportunityResponse { opportunity }))
}

pub async fn parse_opportunity(
    State(state): State<AppState>,
    Path(opportunity_id): Path<Uuid>,
) -> Result<Json<EmploymentOpportunityResponse>, AppError> {
    let opportunity = state.employment.parse_opportunity(opportunity_id).await?;
    Ok(Json(EmploymentOpportunityResponse { opportunity }))
}

pub async fn parse_opportunity_for_profile(
    State(state): State<AppState>,
    Path((profile_id, opportunity_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<EmploymentOpportunityResponse>, AppError> {
    get_profile_opportunity(&state, profile_id, opportunity_id).await?;
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

pub async fn score_opportunity_for_profile(
    State(state): State<AppState>,
    Path((profile_id, opportunity_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<EmploymentOpportunityResponse>, AppError> {
    get_profile_opportunity(&state, profile_id, opportunity_id).await?;
    let opportunity = state.employment.score_opportunity(opportunity_id).await?;
    Ok(Json(EmploymentOpportunityResponse { opportunity }))
}

pub async fn generate_cover_letter_for_profile(
    State(state): State<AppState>,
    Path((profile_id, opportunity_id)): Path<(Uuid, Uuid)>,
    Json(req): Json<GenerateCoverLetterRequest>,
) -> Result<Json<CoverLetterResponse>, AppError> {
    let opportunity = get_profile_opportunity(&state, profile_id, opportunity_id).await?;
    let context = state
        .employment_context
        .load_application_context_for_profile(profile_id)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;
    let cover_letter = state
        .employment
        .generate_cover_letter(&opportunity, &context, &req.direction)
        .await?;

    Ok(Json(CoverLetterResponse { cover_letter }))
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

pub async fn archive_opportunity_for_profile(
    State(state): State<AppState>,
    Path((profile_id, opportunity_id)): Path<(Uuid, Uuid)>,
    Json(req): Json<StatusUpdateRequest>,
) -> Result<Json<EmploymentOpportunityResponse>, AppError> {
    get_profile_opportunity(&state, profile_id, opportunity_id).await?;
    update_profile_opportunity_status(
        &state,
        opportunity_id,
        EmploymentOpportunityStatus::Archived,
        req.reason,
    )
    .await
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

pub async fn reject_opportunity_for_profile(
    State(state): State<AppState>,
    Path((profile_id, opportunity_id)): Path<(Uuid, Uuid)>,
    Json(req): Json<StatusUpdateRequest>,
) -> Result<Json<EmploymentOpportunityResponse>, AppError> {
    get_profile_opportunity(&state, profile_id, opportunity_id).await?;
    update_profile_opportunity_status(
        &state,
        opportunity_id,
        EmploymentOpportunityStatus::Rejected,
        req.reason,
    )
    .await
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

pub async fn restore_opportunity_for_profile(
    State(state): State<AppState>,
    Path((profile_id, opportunity_id)): Path<(Uuid, Uuid)>,
    Json(req): Json<StatusUpdateRequest>,
) -> Result<Json<EmploymentOpportunityResponse>, AppError> {
    get_profile_opportunity(&state, profile_id, opportunity_id).await?;
    update_profile_opportunity_status(
        &state,
        opportunity_id,
        EmploymentOpportunityStatus::QueuedForReview,
        req.reason,
    )
    .await
}

async fn ensure_profile_exists(state: &AppState, profile_id: Uuid) -> Result<(), AppError> {
    state
        .employment
        .repository
        .get_profile(profile_id)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?
        .map(|_| ())
        .ok_or_else(|| AppError::NotFound("Employment profile not found".to_string()))
}

async fn get_profile_opportunity(
    state: &AppState,
    profile_id: Uuid,
    opportunity_id: Uuid,
) -> Result<EmploymentOpportunity, AppError> {
    ensure_profile_exists(state, profile_id).await?;
    let opportunity = state
        .employment
        .get_opportunity(opportunity_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Employment opportunity not found".to_string()))?;

    if opportunity.profile_id != profile_id {
        return Err(AppError::NotFound(
            "Employment opportunity not found".to_string(),
        ));
    }

    Ok(opportunity)
}

async fn update_profile_opportunity_status(
    state: &AppState,
    opportunity_id: Uuid,
    status: EmploymentOpportunityStatus,
    reason: Option<String>,
) -> Result<Json<EmploymentOpportunityResponse>, AppError> {
    let opportunity = state
        .employment
        .repository
        .update_opportunity_status(opportunity_id, status, reason)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?
        .ok_or_else(|| AppError::NotFound("opportunity not found".to_string()))?;

    Ok(Json(EmploymentOpportunityResponse { opportunity }))
}

fn clean_optional_text(value: String) -> Option<String> {
    let value = value.trim().to_string();
    if value.is_empty() {
        None
    } else {
        Some(value)
    }
}
