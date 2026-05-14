#![allow(dead_code)]

use anyhow::Result;

use crate::{
    context::{
        models::{ContextKind, SavedContext},
        ContextService,
    },
    domains::employment::models::EmploymentContextBundle,
};

#[derive(Clone)]
pub struct EmploymentContextService {
    context: ContextService,
}

impl EmploymentContextService {
    pub fn new(context: ContextService) -> Self {
        Self { context }
    }

    pub async fn load_application_context(&self) -> Result<EmploymentContextBundle> {
        let career_profile = self
            .context
            .get_relevant_context("", Some(ContextKind::CareerProfile))
            .await?;
        let resume_facts = self
            .context
            .get_relevant_context("", Some(ContextKind::ResumeFact))
            .await?;
        let project_evidence = self
            .context
            .get_relevant_context("", Some(ContextKind::ProjectSummary))
            .await?;
        let writing_preferences = self
            .context
            .get_relevant_context("", Some(ContextKind::WritingPreference))
            .await?;
        let employment_preferences = self
            .context
            .get_relevant_context("", Some(ContextKind::EmploymentPreference))
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
