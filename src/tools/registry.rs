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
        HomeAssistantGetEntityTool, HomeAssistantSearchEntitiesTool, HomeAssistantStatesTool,
        HomeAssistantSummaryTool,
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
            let ha_client = HomeAssistantClient::new(
                config.homeassistant.base_url.clone(),
                config.homeassistant.token_env.clone(),
                config.homeassistant.timeout_seconds,
            )?;

            registry
                .register(HomeAssistantSummaryTool::new(ha_client.clone()))
                .await;
            registry
                .register(HomeAssistantStatesTool::new(ha_client.clone()))
                .await;
            registry
                .register(HomeAssistantGetEntityTool::new(ha_client.clone()))
                .await;
            registry
                .register(HomeAssistantSearchEntitiesTool::new(ha_client))
                .await;
        }

        Ok(registry)
    }

    pub async fn register<T>(&self, tool: T)
    where
        T: Tool + 'static,
    {
        let name = tool.descriptor().name.clone();
        self.tools.write().await.insert(name, Arc::new(tool));
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
            tool: name.to_string(),
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
        let tools = self.tools.read().await;
        tools.values().map(|t| t.descriptor()).collect()
    }
}