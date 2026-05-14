#![allow(dead_code)]

use anyhow::Result;
use chrono::Utc;
use uuid::Uuid;

use crate::{
    context::{
        models::{ContextKind, SavedContext},
        ContextService,
    },
    domains::employment::models::{
        EmploymentContextBundle, EmploymentOpportunity, EmploymentOpportunitySearch,
        EmploymentOpportunityStatus,
    },
    domains::employment::repository::EmploymentRepository,
    error::AppError,
    op_tasks::service::OpTaskService,
    services::llm_service::LlmService,
};

#[derive(Clone)]
pub struct EmploymentOpportunityService {
    pub repository: EmploymentRepository,
    op_tasks: OpTaskService,
    llm: Option<LlmService>,
}

impl EmploymentOpportunityService {
    pub fn new(
        repository: EmploymentRepository,
        op_tasks: OpTaskService,
        llm: Option<LlmService>,
    ) -> Self {
        Self {
            repository,
            op_tasks,
            llm,
        }
    }

    pub async fn create_opportunity(
        &self,
        opportunity: EmploymentOpportunity,
    ) -> Result<EmploymentOpportunity, AppError> {
        self.repository
            .create_opportunity(opportunity)
            .await
            .map_err(|e| AppError::Internal(e.to_string()))
    }

    pub async fn get_opportunity(
        &self,
        id: Uuid,
    ) -> Result<Option<EmploymentOpportunity>, AppError> {
        self.repository
            .get_opportunity(id)
            .await
            .map_err(|e| AppError::Internal(e.to_string()))
    }

    pub async fn list_opportunities(
        &self,
        search: EmploymentOpportunitySearch,
    ) -> Result<Vec<EmploymentOpportunity>, AppError> {
        self.repository
            .list_opportunities(search)
            .await
            .map_err(|e| AppError::Internal(e.to_string()))
    }

    pub async fn create_from_artifact(
        &self,
        profile_id: Uuid,
        artifact_id: Uuid,
    ) -> Result<EmploymentOpportunity, AppError> {
        let artifact = self.op_tasks.get_artifact(artifact_id).await?;
        if artifact.profile_id != profile_id {
            return Err(AppError::NotFound("Op Task artifact not found".to_string()));
        }

        if artifact.artifact_type != "readable_web_page" {
            return Err(AppError::BadRequest(
                "Artifact is not a readable_web_page".to_string(),
            ));
        }

        let source_url = artifact
            .location
            .clone()
            .ok_or_else(|| AppError::BadRequest("Artifact missing location".to_string()))?;

        if let Some(existing) = self
            .repository
            .list_opportunities(EmploymentOpportunitySearch {
                profile_id: Some(profile_id),
                source_artifact_id: Some(artifact.id),
                limit: Some(1),
                ..Default::default()
            })
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?
            .into_iter()
            .next()
        {
            return Ok(existing);
        }

        if let Some(existing) = self
            .repository
            .find_opportunity_by_source_url(profile_id, &source_url)
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?
        {
            return Ok(existing);
        }

        let title = artifact
            .content_json
            .as_ref()
            .and_then(|json| json.get("title"))
            .and_then(|t| t.as_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| artifact.name.clone());

        let mut opportunity =
            EmploymentOpportunity::new_discovered(profile_id, source_url, None, Some(artifact.id));

        opportunity.title = Some(title);
        opportunity.description_text = artifact.content_text;
        opportunity.status = EmploymentOpportunityStatus::Discovered;

        self.repository
            .create_opportunity(opportunity)
            .await
            .map_err(|e| AppError::Internal(e.to_string()))
    }

    pub async fn parse_opportunity(
        &self,
        opportunity_id: Uuid,
    ) -> Result<EmploymentOpportunity, AppError> {
        // Get the opportunity
        let mut opportunity = self
            .get_opportunity(opportunity_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Employment opportunity not found".to_string()))?;

        // Check if it has description_text
        let description_text = opportunity.description_text.as_ref().ok_or_else(|| {
            AppError::BadRequest("Opportunity has no description_text to parse".to_string())
        })?;

        // Use LLM to parse the job details
        let llm_service = self
            .llm
            .as_ref()
            .ok_or_else(|| AppError::Internal("LLM service not available".to_string()))?;
        let parsed: serde_json::Value = llm_service
            .parse_job_opportunity("qwen2.5:14b", description_text)
            .await?;

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
        self.repository
            .update_opportunity(opportunity)
            .await
            .map_err(|e| AppError::Internal(e.to_string()))
    }

    pub async fn score_opportunity(
        &self,
        opportunity_id: Uuid,
    ) -> Result<EmploymentOpportunity, AppError> {
        // Get the opportunity
        let mut opportunity = self
            .get_opportunity(opportunity_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Employment opportunity not found".to_string()))?;

        // Calculate fit score (simple algorithm for now)
        let mut score = 0i64;

        // Remote work bonus
        if opportunity
            .remote_type
            .as_ref()
            .map(|rt| rt == "Remote")
            .unwrap_or(false)
        {
            score += 50;
        }

        // Salary bonus (higher salary = higher score)
        if let Some(salary_max) = opportunity.salary_max {
            score += (salary_max / 1000).min(100); // Max 100 points for salary
        }

        // Company bonus (if it's a known good company - placeholder)
        if opportunity
            .company
            .as_ref()
            .map(|c| c.to_lowercase().contains("tech"))
            .unwrap_or(false)
        {
            score += 20;
        }

        opportunity.fit_score = Some(score);
        opportunity.status = EmploymentOpportunityStatus::Scored;
        opportunity.last_seen_at = Utc::now();

        // Save the updated opportunity
        self.repository
            .update_opportunity(opportunity)
            .await
            .map_err(|e| AppError::Internal(e.to_string()))
    }
}

#[derive(Clone)]
pub struct EmploymentContextService {
    context: ContextService,
}

impl EmploymentContextService {
    pub fn new(context: ContextService) -> Self {
        Self { context }
    }

    pub async fn load_application_context(&self) -> Result<EmploymentContextBundle> {
        self.load_application_context_for_profile(
            crate::domains::employment::models::default_employment_profile_id(),
        )
        .await
    }

    pub async fn load_application_context_for_profile(
        &self,
        profile_id: Uuid,
    ) -> Result<EmploymentContextBundle> {
        let career_profile = self
            .context
            .get_relevant_context(profile_id, "", Some(ContextKind::CareerProfile))
            .await?;
        let resume_facts = self
            .context
            .get_relevant_context(profile_id, "", Some(ContextKind::ResumeFact))
            .await?;
        let project_evidence = self
            .context
            .get_relevant_context(profile_id, "", Some(ContextKind::ProjectSummary))
            .await?;
        let writing_preferences = self
            .context
            .get_relevant_context(profile_id, "", Some(ContextKind::WritingPreference))
            .await?;
        let employment_preferences = self
            .context
            .get_relevant_context(profile_id, "", Some(ContextKind::EmploymentPreference))
            .await?;

        let salary_location_preferences =
            filter_employment_preferences(&employment_preferences, &SALARY_LOCATION_MARKERS);
        let role_targeting_preferences =
            filter_employment_preferences(&employment_preferences, &ROLE_TARGETING_MARKERS);

        Ok(EmploymentContextBundle {
            career_profile,
            resume_facts,
            project_evidence,
            writing_preferences,
            salary_location_preferences,
            role_targeting_preferences,
        })
    }
}

const SALARY_LOCATION_MARKERS: [&str; 7] = [
    "salary",
    "compensation",
    "location",
    "remote",
    "hybrid",
    "relocation",
    "travel",
];

const ROLE_TARGETING_MARKERS: [&str; 8] = [
    "role",
    "roles",
    "target",
    "targeting",
    "career",
    "architect",
    "platform",
    "salesforce",
];

fn filter_employment_preferences(contexts: &[SavedContext], markers: &[&str]) -> Vec<SavedContext> {
    contexts
        .iter()
        .filter(|context| context_matches_any_marker(context, markers))
        .cloned()
        .collect()
}

fn context_matches_any_marker(context: &SavedContext, markers: &[&str]) -> bool {
    context.tags.iter().any(|tag| {
        markers
            .iter()
            .any(|marker| tag.eq_ignore_ascii_case(marker))
    }) || markers.iter().any(|marker| {
        contains_case_insensitive(&context.title, marker)
            || contains_case_insensitive(&context.body, marker)
    })
}

fn contains_case_insensitive(value: &str, needle: &str) -> bool {
    value.to_lowercase().contains(&needle.to_lowercase())
}
