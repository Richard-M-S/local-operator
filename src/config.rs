use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct AppConfig {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub docker: DockerConfig,
    pub homeassistant: HomeAssistantConfig,
    pub policy: PolicyConfig,
    pub llm: LlmConfig,
    #[serde(default)]
    pub openai_escalation: OpenAiEscalationConfig,
    pub llm_router: LlmRouterConfig,
    #[serde(default)]
    pub search: SearchConfig,
    #[serde(default)]
    pub auth: AuthConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LlmRouterConfig {
    pub fast_model: String,
    pub default_model: String,
    pub coder_model: String,
    pub deep_model: String,
    pub task_summary_model: String,
    pub task_extraction_model: String,
    pub task_reasoning_model: String,
    pub task_writing_model: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AuthConfig {
    pub enabled: bool,
    pub token_env: String,
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            token_env: "OPERATOR_API_TOKEN".to_string(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DatabaseConfig {
    pub url: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DockerConfig {
    pub enabled: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct HomeAssistantConfig {
    pub enabled: bool,
    pub base_url: String,
    pub token_env: String,
    pub timeout_seconds: u64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PolicyConfig {
    pub allow_tier1_without_confirm: bool,
    pub allow_tier2_without_confirm: bool,
    pub block_tier3: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LlmConfig {
    pub enabled: bool,
    #[allow(dead_code)]
    pub provider: String,
    pub base_url: String,
    #[allow(dead_code)]
    pub model: String,
    pub timeout_seconds: u64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct OpenAiEscalationConfig {
    pub enabled: bool,
    pub api_key_env: String,
    pub base_url: String,
    pub model: String,
    pub timeout_seconds: u64,
}

impl Default for OpenAiEscalationConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            api_key_env: "OPENAI_API_KEY".to_string(),
            base_url: "https://api.openai.com/v1".to_string(),
            model: "gpt-4o-mini".to_string(),
            timeout_seconds: 120,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct SearchConfig {
    pub provider: String,
}

impl Default for SearchConfig {
    fn default() -> Self {
        Self {
            provider: "duckduckgo_html".to_string(),
        }
    }
}

impl AppConfig {
    pub fn load() -> anyhow::Result<Self> {
        let cfg = config::Config::builder()
            .add_source(config::File::with_name("config/default"))
            .add_source(config::Environment::default().separator("__"))
            .build()?;

        Ok(cfg.try_deserialize()?)
    }
}
