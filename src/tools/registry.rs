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
        HaEnergyHvacSnapshotTool, HaGetEntityTool, HaOverviewTool, HaSearchTool, HaStatesTool,
        HaSummaryTool,
    },
    system::SystemStatusTool,
};

#[async_trait]
pub trait Tool: Send + Sync {
    fn descriptor(&self) -> ToolDescriptor;
    async fn execute(&self, args: Value) -> Result<Value, AppError>;
}
#[derive(Clone)]
pub enum ToolEnum {
    SystemStatus(SystemStatusTool),
    DockerListContainers(DockerListContainersTool),
    HaSummary(HaSummaryTool),
    HaStates(HaStatesTool),
    HaGetEntity(HaGetEntityTool),
    HaSearch(HaSearchTool),
    HaOverview(HaOverviewTool),
    HaEnergyHvacSnapshot(HaEnergyHvacSnapshotTool),
}

#[async_trait]
impl Tool for ToolEnum {
    fn descriptor(&self) -> ToolDescriptor {
        match self {
            ToolEnum::SystemStatus(t) => t.descriptor(),
            ToolEnum::DockerListContainers(t) => t.descriptor(),
            ToolEnum::HaSummary(t) => t.descriptor(),
            ToolEnum::HaStates(t) => t.descriptor(),
            ToolEnum::HaGetEntity(t) => t.descriptor(),
            ToolEnum::HaSearch(t) => t.descriptor(),
            ToolEnum::HaOverview(t) => t.descriptor(),
            ToolEnum::HaEnergyHvacSnapshot(t) => t.descriptor(),
        }
    }

    async fn execute(&self, args: Value) -> Result<Value, AppError> {
        match self {
            ToolEnum::SystemStatus(t) => t.execute(args).await,
            ToolEnum::DockerListContainers(t) => t.execute(args).await,
            ToolEnum::HaSummary(t) => t.execute(args).await,
            ToolEnum::HaStates(t) => t.execute(args).await,
            ToolEnum::HaGetEntity(t) => t.execute(args).await,
            ToolEnum::HaSearch(t) => t.execute(args).await,
            ToolEnum::HaOverview(t) => t.execute(args).await,
            ToolEnum::HaEnergyHvacSnapshot(t) => t.execute(args).await,
        }
    }
}
#[derive(Clone, Default)]
pub struct ToolRegistry {
    tools: Arc<RwLock<HashMap<String, ToolEnum>>>,
}

impl ToolRegistry {
    pub async fn new(config: AppConfig) -> anyhow::Result<Self> {
        let registry = Self::default();

        registry
            .register(ToolEnum::SystemStatus(SystemStatusTool::new()))
            .await;

        if config.docker.enabled {
            registry
                .register(ToolEnum::DockerListContainers(
                    DockerListContainersTool::new(),
                ))
                .await;
        }

        if config.homeassistant.enabled {
            let client = HomeAssistantClient::new(
                config.homeassistant.base_url,
                config.homeassistant.token_env,
                config.homeassistant.timeout_seconds,
            )?;

            registry
                .register(ToolEnum::HaSummary(HaSummaryTool::new(client.clone())))
                .await;
            registry
                .register(ToolEnum::HaStates(HaStatesTool::new(client.clone())))
                .await;
            registry
                .register(ToolEnum::HaGetEntity(HaGetEntityTool::new(client.clone())))
                .await;
            registry
                .register(ToolEnum::HaSearch(HaSearchTool::new(client.clone())))
                .await;
            registry
                .register(ToolEnum::HaOverview(HaOverviewTool::new(client.clone())))
                .await;
            registry
                .register(ToolEnum::HaEnergyHvacSnapshot(
                    HaEnergyHvacSnapshotTool::new(client.clone()),
                ))
                .await;
        }

        Ok(registry)
    }

    pub async fn register(&self, tool: ToolEnum) {
        let name = tool.descriptor().name.clone();
        self.tools.write().await.insert(name, tool);
    }

    pub async fn execute(&self, name: &str, args: Value) -> Result<ToolExecutionResult, AppError> {
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

    #[allow(dead_code)]
    pub async fn list(&self) -> Vec<ToolDescriptor> {
        self.tools
            .read()
            .await
            .values()
            .map(|t| t.descriptor())
            .collect()
    }
}
