use std::{collections::HashMap, sync::Arc};

use async_trait::async_trait;
use serde_json::Value;
use tokio::sync::RwLock;

use crate::{
    adapters::home_assistant::HomeAssistantClient,
    config::AppConfig,
    error::AppError,
    models::tool::{ToolDescriptor, ToolExecutionResult},
};

use super::{
    docker::DockerListContainersTool,
    home_assistant::{
        HaGetEntityTool, HaOverviewTool, HaSearchTool, HaStatesTool, HaSummaryTool,
    },
    system::SystemStatusTool,
};

#[async_trait]
pub trait Tool: Send + Sync {
    fn descriptor(&self) -> ToolDescriptor;
    async fn execute(&self, args: Value) -> Result<Value, AppError>;
}

#[derive(Clone, Default)]
pub struct ToolRegistry {
    tools: Arc<RwLock<HashMap<String, Arc<dyn Tool>>>>,
}

impl ToolRegistry {
    pub async fn new(config: AppConfig) -> anyhow::Result<Self> {
        let registry = Self::default();

        registry.register(SystemStatusTool::new()).await;

        if config.docker.enabled {
            registry.register(DockerListContainersTool::new()).await;
        }

        if config.homeassistant.enabled {
            let client = HomeAssistantClient::new(
                config.homeassistant.base_url,
                config.homeassistant.token_env,
                config.homeassistant.timeout_seconds,
            )?;

            registry.register(HaSummaryTool::new(client.clone())).await;
            registry.register(HaStatesTool::new(client.clone())).await;
            registry.register(HaGetEntityTool::new(client.clone())).await;
            registry.register(HaSearchTool::new(client.clone())).await;
            registry.register(HaOverviewTool::new(client)).await;
        }

        Ok(registry)
    }

    pub async fn register<T>(&self, tool: T)
    where
        T: Tool + 'static,
    {
        self.tools
            .write()
            .await
            .insert(tool.descriptor().name.clone(), Arc::new(tool));
    }

    pub async fn execute(
        &self,
        name: &str,
        args: Value,
    ) -> Result<ToolExecutionResult, AppError> {
        let tools = self.tools.read().await;

        let tool = tools
            .get(name)
            .ok_or_else(|| AppError::NotFound(format!("tool not found: {}", name)))?;

        let output = tool.execute(args).await?;

        Ok(ToolExecutionResult {
            tool: name.into(),
            ok: true,
            output,
        })
    }

    pub async fn describe(&self, name: &str) -> Result<ToolDescriptor, AppError> {
        let tools = self.tools.read().await;

        let tool = tools
            .get(name)
            .ok_or_else(|| AppError::NotFound(format!("tool not found: {}", name)))?;

        Ok(tool.descriptor())
    }

    pub async fn list(&self) -> Vec<ToolDescriptor> {
        self.tools
            .read()
            .await
            .values()
            .map(|t| t.descriptor())
            .collect()
    }
}