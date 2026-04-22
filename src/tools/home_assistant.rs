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

    let attributes = raw
        .get("attributes")
        .cloned()
        .unwrap_or_else(|| json!({}));

    let friendly_name = attributes
        .get("friendly_name")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let area = attributes
        .get("area_name")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let domain = entity_id
        .split('.')
        .next()
        .unwrap_or("unknown")
        .to_string();

    Some(HaEntitySummary {
        entity_id,
        state,
        friendly_name,
        area,
        domain,
        attributes,
    })
}

pub struct HomeAssistantSummaryTool {
    client: HomeAssistantClient,
}

impl HomeAssistantSummaryTool {
    pub fn new(client: HomeAssistantClient) -> Self {
        Self { client }
    }
}

#[async_trait]
impl Tool for HomeAssistantSummaryTool {
    fn descriptor(&self) -> ToolDescriptor {
        ToolDescriptor {
            name: "ha.get_summary".to_string(),
            description: "Return a light Home Assistant API status summary".to_string(),
            risk_tier: RiskTier::Tier0,
            requires_confirmation: false,
        }
    }

    async fn execute(&self, _args: Value) -> Result<Value, AppError> {
        let body = self.client.get_root().await?;
        Ok(json!({
            "reachable": true,
            "body": body
        }))
    }
}

pub struct HomeAssistantStatesTool {
    client: HomeAssistantClient,
}

impl HomeAssistantStatesTool {
    pub fn new(client: HomeAssistantClient) -> Self {
        Self { client }
    }
}

#[async_trait]
impl Tool for HomeAssistantStatesTool {
    fn descriptor(&self) -> ToolDescriptor {
        ToolDescriptor {
            name: "ha.get_states".to_string(),
            description: "Return a summarized list of Home Assistant entity states".to_string(),
            risk_tier: RiskTier::Tier0,
            requires_confirmation: false,
        }
    }

    async fn execute(&self, _args: Value) -> Result<Value, AppError> {
        let raw = self.client.get_states().await?;

        let entities: Vec<HaEntitySummary> = raw
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(summarize_entity)
            .collect();

        Ok(json!({
            "count": entities.len(),
            "entities": entities
        }))
    }
}

pub struct HomeAssistantGetEntityTool {
    client: HomeAssistantClient,
}

impl HomeAssistantGetEntityTool {
    pub fn new(client: HomeAssistantClient) -> Self {
        Self { client }
    }
}

#[async_trait]
impl Tool for HomeAssistantGetEntityTool {
    fn descriptor(&self) -> ToolDescriptor {
        ToolDescriptor {
            name: "ha.get_entity".to_string(),
            description: "Return one Home Assistant entity state by entity_id".to_string(),
            risk_tier: RiskTier::Tier0,
            requires_confirmation: false,
        }
    }

    async fn execute(&self, args: Value) -> Result<Value, AppError> {
        let args: HaGetEntityArgs = serde_json::from_value(args)
            .map_err(|e| AppError::BadRequest(format!("invalid args: {e}")))?;

        self.client.get_entity_state(&args.entity_id).await
    }
}

pub struct HomeAssistantSearchEntitiesTool {
    client: HomeAssistantClient,
}

impl HomeAssistantSearchEntitiesTool {
    pub fn new(client: HomeAssistantClient) -> Self {
        Self { client }
    }
}

#[async_trait]
impl Tool for HomeAssistantSearchEntitiesTool {
    fn descriptor(&self) -> ToolDescriptor {
        ToolDescriptor {
            name: "ha.search_entities".to_string(),
            description: "Search Home Assistant entities by id or friendly name".to_string(),
            risk_tier: RiskTier::Tier0,
            requires_confirmation: false,
        }
    }

    async fn execute(&self, args: Value) -> Result<Value, AppError> {
        let args: HaSearchEntitiesArgs = serde_json::from_value(args)
            .map_err(|e| AppError::BadRequest(format!("invalid args: {e}")))?;

        let query = args.query.trim().to_lowercase();
        let limit = args.limit.unwrap_or(25);

        let raw = self.client.get_states().await?;

        let mut matches: Vec<HaEntitySummary> = raw
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(summarize_entity)
            .filter(|e| {
                let friendly = e
                    .friendly_name
                    .as_deref()
                    .unwrap_or("")
                    .to_lowercase();

                e.entity_id.to_lowercase().contains(&query)
                    || friendly.contains(&query)
                    || e.domain.to_lowercase().contains(&query)
            })
            .take(limit)
            .collect();

        matches.sort_by(|a, b| a.entity_id.cmp(&b.entity_id));

        Ok(json!({
            "query": args.query,
            "count": matches.len(),
            "entities": matches
        }))
    }
}