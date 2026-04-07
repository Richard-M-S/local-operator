use crate::config::AppConfig;
use crate::services::{audit_service::AuditService, operator_service::OperatorService, policy_engine::PolicyEngine};
use crate::tools::registry::ToolRegistry;
use sqlx::SqlitePool;

#[derive(Clone)]
pub struct AppState {
    pub config: AppConfig,
    pub db: SqlitePool,
    pub tools: ToolRegistry,
    pub policy: PolicyEngine,
    pub audit: AuditService,
    pub operator: OperatorService,
}

impl AppState {
    pub async fn new(config: AppConfig, db: SqlitePool) -> anyhow::Result<Self> {
        let tools = ToolRegistry::new(config.clone()).await?;
        let policy = PolicyEngine::new(config.policy.clone());
        let audit = AuditService::new(db.clone());
        let operator = OperatorService::new(tools.clone(), policy.clone(), audit.clone());

        Ok(Self {
            config,
            db,
            tools,
            policy,
            audit,
            operator,
        })
    }
}