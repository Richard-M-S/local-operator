use crate::adapters::llm::LlmClient;
use crate::config::AppConfig;
use crate::context::{ContextRepository, ContextService};
use crate::domains::employment::{
    EmploymentContextService, EmploymentOpportunityService, EmploymentRepository,
};
use crate::op_tasks::{OpTaskRepository, OpTaskRunner, OpTaskService};
use crate::readers::ReaderService;
use crate::services::{
    audit_service::AuditService, llm_router::LlmRouter, llm_service::LlmService,
    operator_service::OperatorService, policy_engine::PolicyEngine,
};
use crate::tools::registry::ToolRegistry;
use sqlx::SqlitePool;

#[derive(Clone)]
pub struct AppState {
    pub config: AppConfig,
    #[allow(dead_code)]
    pub db: SqlitePool,
    pub tools: ToolRegistry,
    pub policy: PolicyEngine,
    pub audit: AuditService,
    pub llm: Option<LlmService>,
    #[allow(dead_code)]
    pub llm_router: LlmRouter,
    pub operator: OperatorService,
    pub op_tasks: OpTaskService,
    #[allow(dead_code)]
    pub readers: ReaderService,
    pub context: ContextService,
    pub employment: EmploymentOpportunityService,
    #[allow(dead_code)]
    pub employment_context: EmploymentContextService,
}

impl AppState {
    pub async fn new(config: AppConfig, db: SqlitePool) -> anyhow::Result<Self> {
        let tools = ToolRegistry::new(config.clone()).await?;
        let policy = PolicyEngine::new(config.policy.clone());
        let audit = AuditService::new(db.clone());

        let llm_router = LlmRouter::new(config.llm_router.clone());

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
            llm_router.clone(),
        );

        let readers = ReaderService::new();

        let op_task_repo = OpTaskRepository::new(db.clone());
        let op_task_runner = OpTaskRunner::new(
            tools.clone(),
            llm.clone(),
            readers.clone(),
            config.llm.model.clone(),
        );
        let op_tasks = OpTaskService::new(op_task_repo, op_task_runner);

        let context_repo = ContextRepository::new(db.clone());
        let context = ContextService::new(context_repo);

        let employment_repo = EmploymentRepository::new(db.clone());
        let employment =
            EmploymentOpportunityService::new(employment_repo, op_tasks.clone(), llm.clone());
        let employment_context = EmploymentContextService::new(context.clone());

        Ok(Self {
            config,
            db,
            tools,
            policy,
            audit,
            llm,
            llm_router,
            operator,
            op_tasks,
            readers,
            context,
            employment,
            employment_context,
        })
    }
}
