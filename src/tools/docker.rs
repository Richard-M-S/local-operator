use async_trait::async_trait;
use bollard::container::ListContainersOptions;
use bollard::Docker;
use serde_json::{json, Value};

use crate::{
    error::AppError,
    models::tool::{RiskTier, ToolDescriptor},
};

use super::registry::Tool;

pub struct DockerListContainersTool;

impl DockerListContainersTool {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Tool for DockerListContainersTool {
    fn descriptor(&self) -> ToolDescriptor {
        ToolDescriptor {
            name: "docker.list_containers".to_string(),
            description: "List local Docker containers".to_string(),
            risk_tier: RiskTier::Tier0,
            requires_confirmation: false,
        }
    }

    async fn execute(&self, _args: Value) -> Result<Value, AppError> {
        let docker = Docker::connect_with_local_defaults()
            .map_err(|e| AppError::Internal(format!("docker connect failed: {e}")))?;

        let containers = docker
            .list_containers(Some(ListContainersOptions::<String> {
                all: true,
                ..Default::default()
            }))
            .await
            .map_err(|e| AppError::Internal(format!("docker list failed: {e}")))?;

        let items: Vec<Value> = containers
            .into_iter()
            .map(|c| {
                json!({
                    "id": c.id,
                    "names": c.names,
                    "image": c.image,
                    "state": c.state,
                    "status": c.status
                })
            })
            .collect();

        Ok(json!({
            "count": items.len(),
            "containers": items
        }))
    }
}