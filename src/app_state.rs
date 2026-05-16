use crate::adapters::{llm::LlmClient, openai_escalation::OpenAiEscalationClient};
use crate::config::AppConfig;
use crate::context::{ContextRepository, ContextService};
use crate::domains::employment::{
    EmploymentContextService, EmploymentOpportunityService, EmploymentRepository,
};
use crate::domains::operator::OperatorMetaService;
use crate::op_tasks::{OpTaskRepository, OpTaskRunner, OpTaskService, TaskPlanner};
use crate::readers::ReaderService;
use crate::services::{
    audit_service::AuditService,
    execution::{ModelExecutionService, ToolExecutionService},
    llm_router::LlmRouter,
    llm_service::LlmService,
    operator_service::OperatorService,
    policy_engine::PolicyEngine,
};
use crate::session_memory::SessionMemoryRepository;
use crate::tools::registry::ToolRegistry;
use sqlx::SqlitePool;

#[derive(Clone)]
pub struct AppState {
    pub config: AppConfig,
    #[allow(dead_code)]
    pub db: SqlitePool,
    #[allow(dead_code)]
    tools: ToolRegistry,
    #[allow(dead_code)]
    policy: PolicyEngine,
    pub audit: AuditService,
    pub tool_execution: ToolExecutionService,
    pub model_execution: ModelExecutionService,
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
    pub operator_meta: OperatorMetaService,
    pub session_memory: SessionMemoryRepository,
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
        let tool_execution =
            ToolExecutionService::new(tools.clone(), policy.clone(), audit.clone());
        let model_execution = ModelExecutionService::new(llm.clone(), audit.clone());
        let openai_escalation = if config.openai_escalation.enabled {
            Some(OpenAiEscalationClient::new(
                config.openai_escalation.clone(),
            )?)
        } else {
            None
        };

        let readers = ReaderService::new(config.search.clone());

        let op_task_repo = OpTaskRepository::new(db.clone());
        let employment_repo = EmploymentRepository::new(db.clone());
        let session_memory = SessionMemoryRepository::new(db.clone());
        let context_repo = ContextRepository::new(db.clone());
        let context = ContextService::new(context_repo);
        let operator_meta = OperatorMetaService::new(op_task_repo.clone(), audit.clone());

        let op_task_runner = OpTaskRunner::new(
            tool_execution.clone(),
            model_execution.clone(),
            readers.clone(),
            llm_router.clone(),
            employment_repo.clone(),
            context.clone(),
            openai_escalation,
            operator_meta.clone(),
        );
        let task_planner = TaskPlanner::new(llm_router.clone());
        let op_tasks = OpTaskService::new(op_task_repo, op_task_runner, task_planner);

        let operator = OperatorService::new(
            tool_execution.clone(),
            model_execution.clone(),
            llm_router.clone(),
            op_tasks.clone(),
            employment_repo.clone(),
            session_memory.clone(),
        );

        let employment = EmploymentOpportunityService::new(
            employment_repo,
            op_tasks.clone(),
            model_execution.clone(),
            llm_router.clone(),
        );
        let employment_context = EmploymentContextService::new(context.clone());

        Ok(Self {
            config,
            db,
            tools,
            policy,
            audit,
            tool_execution,
            model_execution,
            llm_router,
            operator,
            op_tasks,
            readers,
            context,
            employment,
            employment_context,
            operator_meta,
            session_memory,
        })
    }
}
