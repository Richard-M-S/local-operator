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
    pub source_artifact_id: Option<Uuid>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
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

    let opportunity = state
        .employment_repo
        .create_opportunity(opportunity)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

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
        source_artifact_id: query.source_artifact_id,
        limit: query.limit,
        offset: query.offset,
    };

    let opportunities = state
        .employment_repo
        .list_opportunities(search)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

    Ok(Json(EmploymentOpportunityListResponse { opportunities }))
}

pub async fn get_opportunity(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<EmploymentOpportunityResponse>, AppError> {
    let opportunity = state
        .employment_repo
        .get_opportunity(id)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?
        .ok_or_else(|| AppError::NotFound("Employment opportunity not found".to_string()))?;

    Ok(Json(EmploymentOpportunityResponse { opportunity }))
}

pub async fn create_opportunity_from_artifact(
    State(state): State<AppState>,
    Path(artifact_id): Path<Uuid>,
) -> Result<Json<EmploymentOpportunityResponse>, AppError> {
    // Get the artifact
    let artifact = state
        .op_tasks
        .get_artifact(artifact_id)
        .await?;

    // Check if it's a readable_web_page artifact
    if artifact.artifact_type != "readable_web_page" {
        return Err(AppError::BadRequest(
            "Artifact is not a readable_web_page".to_string(),
        ));
    }

    // Extract title: content_json.title or artifact.name
    let title = artifact.content_json.as_ref()
        .and_then(|json| json.get("title"))
        .and_then(|t| t.as_str())
        .map(|s| s.to_string())
        .unwrap_or_else(|| artifact.name.clone());

    // Create the opportunity
    let mut opportunity = EmploymentOpportunity::new_discovered(
        artifact.location.ok_or_else(|| {
            AppError::BadRequest("Artifact missing location".to_string())
        })?,
        None, // source_name
        Some(artifact.id),
    );

    // Set additional fields
    opportunity.title = Some(title);
    opportunity.description_text = artifact.content_text;
    opportunity.status = EmploymentOpportunityStatus::Discovered;

    let opportunity = state
        .employment_repo
        .create_opportunity(opportunity)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

    Ok(Json(EmploymentOpportunityResponse { opportunity }))
}

pub async fn parse_opportunity(
    State(state): State<AppState>,
    Path(opportunity_id): Path<Uuid>,
) -> Result<Json<EmploymentOpportunityResponse>, AppError> {
    // Get the opportunity
    let mut opportunity = state
        .employment_repo
        .get_opportunity(opportunity_id)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?
        .ok_or_else(|| AppError::NotFound("Employment opportunity not found".to_string()))?;

    // Check if it has description_text
    let description_text = opportunity.description_text.as_ref()
        .ok_or_else(|| AppError::BadRequest("Opportunity has no description_text to parse".to_string()))?;

    // Use LLM to parse the job details
    let llm_service = state.llm.as_ref()
        .ok_or_else(|| AppError::Internal("LLM service not available".to_string()))?;
    let parsed: serde_json::Value = llm_service.parse_job_opportunity("qwen2.5:14b", description_text).await?;

    // Update opportunity with parsed data
    if let Some(title) = parsed.get("title").and_then(|v| v.as_str()) {
        opportunity.title = Some(title.to_string());
    }
    if let Some(company) = parsed.get("company").and_then(|v| v.as_str()) {
        opportunity.company = Some(company.to_string());
    }
    if let Some(location) = parsed.get("location").and_then(|v| v.as_str()) {
        opportunity.location = Some(location.to_string());
    }
    if let Some(remote_type) = parsed.get("remote_type").and_then(|v| v.as_str()) {
        opportunity.remote_type = Some(remote_type.to_string());
    }
    if let Some(salary_min) = parsed.get("salary_min").and_then(|v| v.as_i64()) {
        opportunity.salary_min = Some(salary_min);
    }
    if let Some(salary_max) = parsed.get("salary_max").and_then(|v| v.as_i64()) {
        opportunity.salary_max = Some(salary_max);
    }
    if let Some(description) = parsed.get("description_text").and_then(|v| v.as_str()) {
        opportunity.description_text = Some(description.to_string());
    }

    // Store the full parsed JSON
    opportunity.extracted_json = Some(parsed);

    // Update status to Parsed
    opportunity.status = EmploymentOpportunityStatus::Parsed;
    opportunity.last_seen_at = Utc::now();

    // Save the updated opportunity
    let opportunity = state
        .employment_repo
        .update_opportunity(opportunity)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

    Ok(Json(EmploymentOpportunityResponse { opportunity }))
}

pub async fn score_opportunity(
    State(state): State<AppState>,
    Path(opportunity_id): Path<Uuid>,
) -> Result<Json<EmploymentOpportunityResponse>, AppError> {
    // Get the opportunity
    let mut opportunity = state
        .employment_repo
        .get_opportunity(opportunity_id)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?
        .ok_or_else(|| AppError::NotFound("Employment opportunity not found".to_string()))?;

    // Calculate fit score (simple algorithm for now)
    let mut score = 0i64;

    // Remote work bonus
    if opportunity.remote_type.as_ref().map(|rt| rt == "Remote").unwrap_or(false) {
        score += 50;
    }

    // Salary bonus (higher salary = higher score)
    if let Some(salary_max) = opportunity.salary_max {
        score += (salary_max / 1000).min(100); // Max 100 points for salary
    }

    // Company bonus (if it's a known good company - placeholder)
    if opportunity.company.as_ref().map(|c| c.to_lowercase().contains("tech")).unwrap_or(false) {
        score += 20;
    }

    opportunity.fit_score = Some(score);
    opportunity.status = EmploymentOpportunityStatus::Scored;
    opportunity.last_seen_at = Utc::now();

    // Save the updated opportunity
    let opportunity = state
        .employment_repo
        .update_opportunity(opportunity)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

    Ok(Json(EmploymentOpportunityResponse { opportunity }))
}
