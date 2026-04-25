use crate::adapters::llm::LlmClient;
use crate::config::AppConfig;
use crate::services::{
    audit_service::AuditService, llm_service::LlmService, operator_service::OperatorService,
    policy_engine::PolicyEngine,
};
use crate::tools::registry::ToolRegistry;
use sqlx::SqlitePool;

#[derive(Clone)]
pub struct AppState {
    pub config: AppConfig,
    pub db: SqlitePool,
    pub tools: ToolRegistry,
    pub policy: PolicyEngine,
    pub audit: AuditService,
    pub llm: Option<LlmService>,
    pub operator: OperatorService,
}

impl AppState {
    pub async fn new(config: AppConfig, db: SqlitePool) -> anyhow::Result<Self> {
        let tools = ToolRegistry::new(config.clone()).await?;
        let policy = PolicyEngine::new(config.policy.clone());
        let audit = AuditService::new(db.clone());

        let llm = if config.llm.enabled {
            let llm_client = LlmClient::new(config.llm.clone())?;
            Some(LlmService::new(llm_client))
        } else {
            None
        };

        let operator = OperatorService::new(
            tools.clone(),
            policy.clone(),
            audit.clone(),
            llm.clone(),
        );

        Ok(Self {
            config,
            db,
            tools,
            policy,
            audit,
            llm,
            operator,
        })
    }
}