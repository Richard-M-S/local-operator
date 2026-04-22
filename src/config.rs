use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct AppConfig {
    pub server: ServerConfig,
    pub logging: LoggingConfig,
    pub database: DatabaseConfig,
    pub docker: DockerConfig,
    pub homeassistant: HomeAssistantConfig,
    pub policy: PolicyConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LoggingConfig {
    pub level: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DatabaseConfig {
    pub url: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DockerConfig {
    pub enabled: bool,
    pub allowed_restart_containers: Vec<String>,
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

impl AppConfig {
    pub fn load() -> anyhow::Result<Self> {
        let builder = config::Config::builder()
            .add_source(config::File::with_name("config/default"))
            .add_source(config::Environment::default().separator("__"));

        Ok(builder.build()?.try_deserialize()?)
    }
}