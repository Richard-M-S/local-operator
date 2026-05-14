#![allow(dead_code)]
use crate::context::models::SavedContext;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

pub const DEFAULT_EMPLOYMENT_PROFILE_ID: &str = "00000000-0000-0000-0000-000000000001";

pub fn default_employment_profile_id() -> Uuid {
    Uuid::parse_str(DEFAULT_EMPLOYMENT_PROFILE_ID).expect("default profile id is a valid uuid")
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EmploymentProfile {
    pub id: Uuid,
    pub display_name: String,
    pub email: Option<String>,
    pub notes: Option<String>,
    pub criteria: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: Option<DateTime<Utc>>,
}

#[derive(Clone, Debug, Default)]
pub struct EmploymentContextBundle {
    pub career_profile: Vec<SavedContext>,
    pub resume_facts: Vec<SavedContext>,
    pub project_evidence: Vec<SavedContext>,
    pub writing_preferences: Vec<SavedContext>,
    pub salary_location_preferences: Vec<SavedContext>,
    pub role_targeting_preferences: Vec<SavedContext>,
}

impl EmploymentContextBundle {
    pub fn is_empty(&self) -> bool {
        self.career_profile.is_empty()
            && self.resume_facts.is_empty()
            && self.project_evidence.is_empty()
            && self.writing_preferences.is_empty()
            && self.salary_location_preferences.is_empty()
            && self.role_targeting_preferences.is_empty()
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EmploymentOpportunity {
    pub id: Uuid,
    pub profile_id: Uuid,
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
    pub risk_flags: Vec<String>,
    pub status: EmploymentOpportunityStatus,
    pub skip_reason: Option<String>,
    pub skip_recommendation: Option<String>,
    pub source_artifact_id: Option<Uuid>,
    pub first_seen_at: DateTime<Utc>,
    pub last_seen_at: DateTime<Utc>,
}

impl EmploymentOpportunity {
    pub fn new_discovered(
        profile_id: Uuid,
        source_url: String,
        source_name: Option<String>,
        source_artifact_id: Option<Uuid>,
    ) -> Self {
        let now = Utc::now();

        Self {
            id: Uuid::new_v4(),
            profile_id,
            source_url,
            source_name,
            title: None,
            company: None,
            location: None,
            remote_type: None,
            salary_min: None,
            salary_max: None,
            description_text: None,
            extracted_json: None,
            fit_score: None,
            primary_fit_score: None,
            oe_fit_score: None,
            recommended_track: None,
            score_reason: None,
            risk_flags: vec![],
            status: EmploymentOpportunityStatus::Discovered,
            skip_reason: None,
            skip_recommendation: None,
            source_artifact_id,
            first_seen_at: now,
            last_seen_at: now,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum EmploymentOpportunityStatus {
    Discovered,
    Parsed,
    Scored,
    QueuedForReview,
    Applied,
    Skipped,
    Rejected,
    Archived,
    Closed,
    Other(String),
}

#[derive(Clone, Debug, Default, Deserialize)]
pub struct EmploymentOpportunitySearch {
    pub profile_id: Option<Uuid>,
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
