#![allow(dead_code)]

use crate::context::models::SavedContext;

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
