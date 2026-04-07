use std::{collections::HashMap, sync::Arc};

use async_trait::async_trait;
use serde_json::Value;

use crate::{config::AppConfig, error::AppError};

#[async_trait]
pub trait Tool: Send + Sync {
    async fn execute(&self, args: Value) -> Result<Value, AppError>;
}

#[derive(Clone)]
pub struct ToolRegistry {
    tools: Arc<HashMap<String, Arc<dyn Tool>>>,
}

impl ToolRegistry {
    pub async fn new(config: AppConfig) -> anyhow::Result<Self> {
        let mut tools: HashMap<String, Arc<dyn Tool>> = HashMap::new();

        tools.insert(
            "system.get_status".into(),
            Arc::new(super::system::GetSystemStatusTool {}),
        );

        tools.insert(
            "docker.list_containers".into(),
            Arc::new(super::docker::ListContainersTool {}),
        );

        tools.insert(
            "docker.restart_container".into(),
            Arc::new(super::docker::RestartContainerTool {
                allowed: config.docker.allowed_restart_containers.clone(),
            }),
        );

        tools.insert(
            "ha.get_summary".into(),
            Arc::new(super::home_assistant::GetHomeSummaryTool {
                enabled: config.homeassistant.enabled,
            }),
        );

        Ok(Self {
            tools: Arc::new(tools),
        })
    }

    pub async fn execute(&self, name: &str, args: Value) -> Result<Value, AppError> {
        let tool = self
            .tools
            .get(name)
            .ok_or_else(|| AppError::NotFound(format!("tool '{}' not found", name)))?;

        tool.execute(args).await
    }
}