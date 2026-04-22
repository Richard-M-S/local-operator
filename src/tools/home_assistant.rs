use async_trait::async_trait;
use serde_json::{json, Value};

use crate::{
    adapters::home_assistant::HomeAssistantClient,
    error::AppError,
    models::{
        home_assistant::{HaEntitySummary, HaGetEntityArgs, HaSearchEntitiesArgs},
        tool::{RiskTier, ToolDescriptor},
    },
};

use super::registry::Tool;

fn summarize_entity(raw: &Value) -> Option<HaEntitySummary> {
    let entity_id = raw.get("entity_id")?.as_str()?.to_string();
    let state = raw.get("state")?.as_str()?.to_string();

    let attributes = raw.get("attributes").cloned().unwrap_or(json!({}));

    let friendly_name = attributes
        .get("friendly_name")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let domain = entity_id.split('.').next()?.to_string();

    Some(HaEntitySummary {
        entity_id,
        state,
        friendly_name,
        domain,
        attributes,
    })
}

pub struct HaSummaryTool {
    client: HomeAssistantClient,
}

impl HaSummaryTool {
    pub fn new(client: HomeAssistantClient) -> Self {
        Self { client }
    }
}

#[async_trait]
impl Tool for HaSummaryTool {
    fn descriptor(&self) -> ToolDescriptor {
        ToolDescriptor {
            name: "ha.get_summary".into(),
            description: "Basic HA connectivity check".into(),
            risk_tier: RiskTier::Tier0,
            requires_confirmation: false,
        }
    }

    async fn execute(&self, _: Value) -> Result<Value, AppError> {
        let res = self.client.get_root().await?;
        Ok(json!({ "ok": true, "body": res }))
    }
}

pub struct HaStatesTool {
    client: HomeAssistantClient,
}

impl HaStatesTool {
    pub fn new(client: HomeAssistantClient) -> Self {
        Self { client }
    }
}

#[async_trait]
impl Tool for HaStatesTool {
    fn descriptor(&self) -> ToolDescriptor {
        ToolDescriptor {
            name: "ha.get_states".into(),
            description: "List HA entities".into(),
            risk_tier: RiskTier::Tier0,
            requires_confirmation: false,
        }
    }

    async fn execute(&self, _: Value) -> Result<Value, AppError> {
        let raw = self.client.get_states().await?;

        let entities: Vec<_> = raw
            .as_array()
            .unwrap_or(&vec![])
            .iter()
            .filter_map(summarize_entity)
            .collect();

        Ok(json!({ "count": entities.len(), "entities": entities }))
    }
}

pub struct HaGetEntityTool {
    client: HomeAssistantClient,
}

impl HaGetEntityTool {
    pub fn new(client: HomeAssistantClient) -> Self {
        Self { client }
    }
}

#[async_trait]
impl Tool for HaGetEntityTool {
    fn descriptor(&self) -> ToolDescriptor {
        ToolDescriptor {
            name: "ha.get_entity".into(),
            description: "Get one entity".into(),
            risk_tier: RiskTier::Tier0,
            requires_confirmation: false,
        }
    }

    async fn execute(&self, args: Value) -> Result<Value, AppError> {
        let args: HaGetEntityArgs = serde_json::from_value(args)?;
        self.client.get_entity_state(&args.entity_id).await
    }
}

pub struct HaSearchTool {
    client: HomeAssistantClient,
}

impl HaSearchTool {
    pub fn new(client: HomeAssistantClient) -> Self {
        Self { client }
    }
}

#[async_trait]
impl Tool for HaSearchTool {
    fn descriptor(&self) -> ToolDescriptor {
        ToolDescriptor {
            name: "ha.search_entities".into(),
            description: "Search entities".into(),
            risk_tier: RiskTier::Tier0,
            requires_confirmation: false,
        }
    }

    async fn execute(&self, args: Value) -> Result<Value, AppError> {
        let args: HaSearchEntitiesArgs = serde_json::from_value(args)?;

        let raw = self.client.get_states().await?;

        let query = args.query.to_lowercase();

        let results: Vec<_> = raw
            .as_array()
            .unwrap_or(&vec![])
            .iter()
            .filter_map(summarize_entity)
            .filter(|e| {
                e.entity_id.to_lowercase().contains(&query)
                    || e.friendly_name
                        .as_deref()
                        .unwrap_or("")
                        .to_lowercase()
                        .contains(&query)
            })
            .take(args.limit.unwrap_or(20))
            .collect();

        Ok(json!({ "results": results }))
    }
}