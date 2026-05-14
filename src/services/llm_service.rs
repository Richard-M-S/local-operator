use crate::{adapters::llm::LlmClient, error::AppError};

#[derive(Clone)]
pub struct LlmService {
    client: LlmClient,
}

impl LlmService {
    pub fn new(client: LlmClient) -> Self {
        Self { client }
    }

    pub async fn ask_model(
        &self,
        model: &str,
        system: &str,
        prompt: &str,
    ) -> Result<String, AppError> {
        self.client.chat_with_model(model, system, prompt).await
    }

    pub async fn summarize_home_overview_with_model(
        &self,
        model: &str,
        user_command: &str,
        overview_json: &serde_json::Value,
    ) -> Result<String, AppError> {
        let system = r#"
    You are Local Operator, a home automation assistant.
    Use the provided Home Assistant overview only.
    Do not invent devices or states.
    Be precise about locks, doors, presence, alarms, and garage doors.
    If data is missing or ambiguous, say so.
    "#;

        let prompt = format!(
            r#"
    User request:
    {user_command}

    Current Home Assistant overview JSON:
    {overview_json}

    Respond with a concise, useful answer.
    "#,
            user_command = user_command,
            overview_json = serde_json::to_string_pretty(overview_json).unwrap_or_default()
        );

        self.ask_model(model, system, &prompt).await
    }

    pub async fn parse_job_opportunity(
        &self,
        model: &str,
        job_text: &str,
    ) -> Result<serde_json::Value, AppError> {
        let system = r#"
You are an expert job posting parser. Extract structured information from job descriptions.
Return only valid JSON with the following fields (use null for missing information):
- title: string or null
- company: string or null  
- location: string or null
- remote_type: "Remote" | "Hybrid" | "On-site" | null
- salary_min: number or null
- salary_max: number or null
- description_text: string or null (cleaned up version)
- requirements: array of strings or null
- benefits: array of strings or null
"#;

        let prompt = format!(
            r#"
Parse this job posting text and extract the structured information as JSON:

{job_text}

Return only the JSON object, no additional text.
"#,
            job_text = job_text
        );

        let response = self.ask_model(model, system, &prompt).await?;

        // Try to parse as JSON
        serde_json::from_str(&response)
            .map_err(|e| AppError::Internal(format!("Failed to parse LLM response as JSON: {}", e)))
    }

    pub async fn score_job_opportunity(
        &self,
        model: &str,
        job: &serde_json::Value,
        criteria: &str,
    ) -> Result<serde_json::Value, AppError> {
        let system = r#"
You are an employment fit scoring assistant. Score job opportunities against a user's job-search criteria.
Return only valid JSON with these exact fields:
- primary_fit_score: integer 0-100
- oe_fit_score: integer 0-100
- recommended_track: "primary" | "oe" | "both" | "manual_review" | "skip"
- score_reason: string
- risk_flags: array of strings using only: on_site_or_hybrid, heavy_meetings, on_call, sole_owner, client_facing, strict_availability, heavy_travel, conflict_risk, unclear_scope
- skip_recommendation: string or null

Rules:
- Evaluate primary and OE tracks separately.
- OE requires confirmed remote work. If remote work is not clearly confirmed, set oe_fit_score to 0, include on_site_or_hybrid or unclear_scope, and make skip_recommendation explicitly reject OE consideration.
- The output is advisory only. Do not recommend auto-applying.
- Any conflicts, strict availability, legal terms, confidentiality concerns, unclear scope, or current-obligation risk must be manual_review or skip.
- Prefer manual_review over confident recommendation when evidence is ambiguous.
"#;

        let prompt = format!(
            r#"
User criteria:
{criteria}

Parsed job/opportunity JSON:
{job_json}

Score both tracks and return only the JSON object.
"#,
            criteria = criteria,
            job_json = serde_json::to_string_pretty(job).unwrap_or_default()
        );

        let response = self.ask_model(model, system, &prompt).await?;

        serde_json::from_str(&response).map_err(|e| {
            AppError::Internal(format!("Failed to parse LLM score response as JSON: {}", e))
        })
    }
}
