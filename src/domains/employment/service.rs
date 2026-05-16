#![allow(dead_code)]

use anyhow::Result;
use chrono::Utc;
use serde_json::json;
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
    services::execution::{ExecutionContext, ModelExecutionService},
    services::llm_router::LlmRouter,
};

#[derive(Clone)]
pub struct EmploymentOpportunityService {
    pub repository: EmploymentRepository,
    op_tasks: OpTaskService,
    model_execution: ModelExecutionService,
    llm_router: LlmRouter,
}

impl EmploymentOpportunityService {
    pub fn new(
        repository: EmploymentRepository,
        op_tasks: OpTaskService,
        model_execution: ModelExecutionService,
        llm_router: LlmRouter,
    ) -> Self {
        Self {
            repository,
            op_tasks,
            model_execution,
            llm_router,
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
        let model = self.llm_router.task_extraction_model();
        let parsed: serde_json::Value = self
            .model_execution
            .parse_job_opportunity(
                &model,
                description_text,
                ExecutionContext::default()
                    .with_model_purpose("employment_parse")
                    .with_input_summary(format!("Parse opportunity {}", opportunity.id)),
            )
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
        let mut opportunity = self
            .get_opportunity(opportunity_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Employment opportunity not found".to_string()))?;

        let profile = self
            .repository
            .get_profile(opportunity.profile_id)
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?
            .ok_or_else(|| AppError::NotFound("Employment profile not found".to_string()))?;
        let criteria = profile.criteria.unwrap_or_else(|| {
            "Score for primary fit and OE fit. Favor confirmed remote work, clear scope, low risk, and strong match to the profile.".to_string()
        });

        let job_json = opportunity_scoring_json(&opportunity);
        let model = self.llm_router.task_reasoning_model();
        let scored = self
            .model_execution
            .score_job_opportunity(
                &model,
                &job_json,
                &criteria,
                ExecutionContext::default()
                    .with_model_purpose("employment_score")
                    .with_input_summary(format!("Score opportunity {}", opportunity.id)),
            )
            .await
            .unwrap_or_else(|_| heuristic_score(&opportunity, &criteria));

        apply_scoring_output(&mut opportunity, scored);

        opportunity.fit_score = opportunity.primary_fit_score.or(opportunity.oe_fit_score);
        opportunity.status = EmploymentOpportunityStatus::Scored;
        opportunity.last_seen_at = Utc::now();

        self.repository
            .update_opportunity(opportunity)
            .await
            .map_err(|e| AppError::Internal(e.to_string()))
    }

    pub async fn generate_cover_letter(
        &self,
        opportunity: &EmploymentOpportunity,
        context: &EmploymentContextBundle,
        direction: &str,
    ) -> Result<String, AppError> {
        let profile = self
            .repository
            .get_profile(opportunity.profile_id)
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?
            .ok_or_else(|| AppError::NotFound("Employment profile not found".to_string()))?;
        let criteria = profile.criteria.unwrap_or_default();
        let profile_context = render_cover_letter_context(context);
        let opportunity_json = opportunity_scoring_json(opportunity);

        let model = self.llm_router.task_writing_model();
        match self
            .model_execution
            .generate_cover_letter(
                &model,
                &opportunity_json,
                &criteria,
                &profile_context,
                direction,
                ExecutionContext::default()
                    .with_model_purpose("employment_cover_letter")
                    .with_input_summary(format!("Generate cover letter for {}", opportunity.id)),
            )
            .await
        {
            Ok(cover_letter) => Ok(cover_letter),
            Err(_) => Ok(fallback_cover_letter(opportunity, &criteria, direction)),
        }
    }
}

fn render_cover_letter_context(context: &EmploymentContextBundle) -> String {
    let sections = [
        ("Career profile", &context.career_profile),
        ("Resume facts", &context.resume_facts),
        ("Project evidence", &context.project_evidence),
        ("Writing preferences", &context.writing_preferences),
        (
            "Salary/location preferences",
            &context.salary_location_preferences,
        ),
        (
            "Role targeting preferences",
            &context.role_targeting_preferences,
        ),
    ];

    sections
        .iter()
        .filter(|(_, items)| !items.is_empty())
        .map(|(label, items)| {
            let body = items
                .iter()
                .take(8)
                .map(|item| format!("- {}: {}", item.title, item.body))
                .collect::<Vec<_>>()
                .join("\n");
            format!("{label}:\n{body}")
        })
        .collect::<Vec<_>>()
        .join("\n\n")
}

fn fallback_cover_letter(
    opportunity: &EmploymentOpportunity,
    criteria: &str,
    direction: &str,
) -> String {
    let title = opportunity.title.as_deref().unwrap_or("the role");
    let company = opportunity.company.as_deref().unwrap_or("your team");
    let direction = direction.trim();

    format!(
        "Hello,\n\nI am interested in {title} at {company}. The role appears aligned with my focus on Salesforce platform architecture, automation, and practical systems improvement.\n\nMy background is strongest where platform ownership, process design, automation, and cross-functional problem solving intersect. I look for opportunities where I can improve reliability, reduce manual work, and make Salesforce easier for teams to operate.\n\n{direction_block}Based on the current profile criteria, I would want to confirm scope, remote expectations, meeting load, and any availability or confidentiality constraints before moving forward.\n\nThank you for your time,\n",
        title = title,
        company = company,
        direction_block = if direction.is_empty() {
            String::new()
        } else {
            format!("{direction}\n\n")
        },
    )
    .replace(
        "Based on the current profile criteria",
        if criteria.trim().is_empty() {
            "Based on the available role information"
        } else {
            "Based on the current profile criteria"
        },
    )
}

fn opportunity_scoring_json(opportunity: &EmploymentOpportunity) -> serde_json::Value {
    json!({
        "id": opportunity.id,
        "title": opportunity.title,
        "company": opportunity.company,
        "location": opportunity.location,
        "remote_type": opportunity.remote_type,
        "salary_min": opportunity.salary_min,
        "salary_max": opportunity.salary_max,
        "description_text": opportunity.description_text,
        "extracted_json": opportunity.extracted_json,
        "source_url": opportunity.source_url,
    })
}

fn apply_scoring_output(opportunity: &mut EmploymentOpportunity, scored: serde_json::Value) {
    opportunity.primary_fit_score = scored
        .get("primary_fit_score")
        .and_then(|value| value.as_i64())
        .map(clamp_score);
    opportunity.oe_fit_score = scored
        .get("oe_fit_score")
        .and_then(|value| value.as_i64())
        .map(clamp_score);
    opportunity.recommended_track = scored
        .get("recommended_track")
        .and_then(|value| value.as_str())
        .map(clean_score_text);
    opportunity.score_reason = scored
        .get("score_reason")
        .and_then(|value| value.as_str())
        .map(clean_score_text);
    opportunity.risk_flags = scored
        .get("risk_flags")
        .and_then(|value| value.as_array())
        .map(|items| {
            items
                .iter()
                .filter_map(|value| value.as_str())
                .map(clean_score_text)
                .filter(|value| is_known_risk_flag(value))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    opportunity.skip_recommendation = scored
        .get("skip_recommendation")
        .and_then(|value| value.as_str())
        .map(clean_score_text)
        .filter(|value| !value.is_empty());

    enforce_oe_remote_requirement(opportunity);
}

fn heuristic_score(opportunity: &EmploymentOpportunity, criteria: &str) -> serde_json::Value {
    let text = format!(
        "{}\n{}\n{}\n{}",
        opportunity.title.clone().unwrap_or_default(),
        opportunity.remote_type.clone().unwrap_or_default(),
        opportunity.description_text.clone().unwrap_or_default(),
        criteria
    )
    .to_lowercase();

    let remote_confirmed = opportunity
        .remote_type
        .as_ref()
        .map(|value| value.eq_ignore_ascii_case("remote"))
        .unwrap_or(false);

    let mut risk_flags = detect_risk_flags(&text);
    if !remote_confirmed && !risk_flags.iter().any(|flag| flag == "on_site_or_hybrid") {
        risk_flags.push("on_site_or_hybrid".to_string());
    }
    if text.contains("unclear") || opportunity.description_text.is_none() {
        risk_flags.push("unclear_scope".to_string());
    }
    risk_flags.sort();
    risk_flags.dedup();

    let primary_fit_score = [
        opportunity.title.as_deref().unwrap_or(""),
        opportunity.description_text.as_deref().unwrap_or(""),
    ]
    .join(" ")
    .to_lowercase()
    .contains("salesforce")
    .then_some(76)
    .unwrap_or(58)
        + if text.contains("architect") || text.contains("automation") {
            12
        } else {
            0
        };

    let oe_fit_score = if remote_confirmed {
        72 - (risk_flags.len() as i64 * 8)
    } else {
        0
    }
    .clamp(0, 100);

    let recommended_track = if !remote_confirmed && primary_fit_score >= 75 {
        "primary"
    } else if risk_flags.iter().any(|flag| {
        matches!(
            flag.as_str(),
            "conflict_risk" | "strict_availability" | "unclear_scope"
        )
    }) {
        "manual_review"
    } else if primary_fit_score >= 75 && oe_fit_score >= 75 {
        "both"
    } else if primary_fit_score >= 75 {
        "primary"
    } else if oe_fit_score >= 75 {
        "oe"
    } else {
        "skip"
    };

    json!({
        "primary_fit_score": primary_fit_score.clamp(0, 100),
        "oe_fit_score": oe_fit_score,
        "recommended_track": recommended_track,
        "score_reason": "Heuristic advisory score. Re-score with the LLM for a fuller criteria-based explanation.",
        "risk_flags": risk_flags,
        "skip_recommendation": if remote_confirmed {
            serde_json::Value::Null
        } else {
            json!("OE reject: remote work is not clearly confirmed.")
        }
    })
}

fn detect_risk_flags(text: &str) -> Vec<String> {
    let mut flags = vec![];
    if text.contains("hybrid") || text.contains("on-site") || text.contains("onsite") {
        flags.push("on_site_or_hybrid".to_string());
    }
    if text.contains("meetings") || text.contains("standup") || text.contains("stakeholder") {
        flags.push("heavy_meetings".to_string());
    }
    if text.contains("on-call") || text.contains("on call") || text.contains("pager") {
        flags.push("on_call".to_string());
    }
    if text.contains("sole owner")
        || text.contains("single owner")
        || text.contains("own end-to-end")
    {
        flags.push("sole_owner".to_string());
    }
    if text.contains("client-facing")
        || text.contains("client facing")
        || text.contains("customer-facing")
    {
        flags.push("client_facing".to_string());
    }
    if text.contains("core hours") || text.contains("strict availability") || text.contains("9am") {
        flags.push("strict_availability".to_string());
    }
    if text.contains("travel") {
        flags.push("heavy_travel".to_string());
    }
    if text.contains("conflict") || text.contains("non-compete") || text.contains("confidential") {
        flags.push("conflict_risk".to_string());
    }
    flags
}

fn enforce_oe_remote_requirement(opportunity: &mut EmploymentOpportunity) {
    let remote_confirmed = opportunity
        .remote_type
        .as_ref()
        .map(|value| value.eq_ignore_ascii_case("remote"))
        .unwrap_or(false);

    if remote_confirmed {
        return;
    }

    opportunity.oe_fit_score = Some(0);
    if !opportunity
        .risk_flags
        .iter()
        .any(|flag| flag == "on_site_or_hybrid")
    {
        opportunity.risk_flags.push("on_site_or_hybrid".to_string());
    }
    opportunity.skip_recommendation =
        Some("OE reject: remote work is not clearly confirmed.".to_string());
    if matches!(
        opportunity.recommended_track.as_deref(),
        Some("oe") | Some("both")
    ) {
        opportunity.recommended_track = Some("manual_review".to_string());
    }
}

fn clamp_score(value: i64) -> i64 {
    value.clamp(0, 100)
}

fn clean_score_text(value: &str) -> String {
    value.trim().to_string()
}

fn is_known_risk_flag(value: &str) -> bool {
    matches!(
        value,
        "on_site_or_hybrid"
            | "heavy_meetings"
            | "on_call"
            | "sole_owner"
            | "client_facing"
            | "strict_availability"
            | "heavy_travel"
            | "conflict_risk"
            | "unclear_scope"
    )
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
