use crate::{adapters::llm::LlmClient, error::AppError};

#[derive(Clone)]
pub struct LlmService {
    client: LlmClient,
}

impl LlmService {
    pub fn new(client: LlmClient) -> Self {
        Self { client }
    }

    pub async fn ask(&self, system: &str, prompt: &str) -> Result<String, AppError> {
        self.client.chat(system, prompt).await
    }

    pub async fn summarize_home_overview(
        &self,
        user_command: &str,
        overview_json: &serde_json::Value,
    ) -> Result<String, AppError> {
        let system = r#"
You are Local Operator, a home automation assistant.
You summarize Home Assistant state clearly and safely.
Do not invent devices or states.
For security-sensitive items like locks, doors, presence, alarms, and garage doors, be precise.
If data is missing or ambiguous, say so.
"#;

        let prompt = format!(
            r#"
User request:
{user_command}

Current Home Assistant overview JSON:
{overview_json}

Respond with a concise, useful summary.
"#,
            user_command = user_command,
            overview_json = serde_json::to_string_pretty(overview_json).unwrap_or_default()
        );

        self.ask(system, &prompt).await
    }
}