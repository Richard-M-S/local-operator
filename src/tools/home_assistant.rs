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

    let attributes = raw.get("attributes").cloned().unwrap_or_else(|| json!({}));

    let friendly_name = attributes
        .get("friendly_name")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let area = attributes
        .get("area_name")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let domain = entity_id.split('.').next().unwrap_or("unknown").to_string();

    Some(HaEntitySummary {
        entity_id,
        state,
        friendly_name,
        area,
        domain,
        attributes,
    })
}

fn state_array(raw: &Value) -> Vec<HaEntitySummary> {
    raw.as_array()
        .map(|items| items.iter().filter_map(summarize_entity).collect())
        .unwrap_or_default()
}

fn is_problem_entity(entity: &HaEntitySummary) -> bool {
    let friendly = friendly_or_id(entity).to_lowercase();

    let device_class = entity
        .attributes
        .get("device_class")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_lowercase();

    // Do not treat enabled automations/scripts as active problems.
    if entity.domain == "automation" || entity.domain == "script" {
        return false;
    }

    entity.state == "on"
        && (device_class == "problem"
            || friendly.contains("jammed")
            || friendly.contains("over-current")
            || friendly.contains("error"))
}

fn is_door_entity(entity: &HaEntitySummary) -> bool {
    let friendly = friendly_or_id(entity).to_lowercase();

    let device_class = entity
        .attributes
        .get("device_class")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_lowercase();

    if entity.domain != "binary_sensor" {
        return false;
    }

    if friendly.contains("jammed")
        || friendly.contains("keypad")
        || friendly.contains("disabled")
        || friendly.contains("alert")
        || friendly.contains("battery")
    {
        return false;
    }

    device_class == "door" || entity.entity_id == "binary_sensor.front_door"
}

fn interpret_binary_door_state(entity: &HaEntitySummary) -> String {
    let friendly = friendly_or_id(entity).to_lowercase();

    if friendly.contains("is closed") {
        return match entity.state.as_str() {
            "on" => "closed".to_string(),
            "off" => "open".to_string(),
            other => other.to_string(),
        };
    }

    if friendly.contains("is open") || friendly.contains("sensor") {
        return match entity.state.as_str() {
            "on" => "open".to_string(),
            "off" => "closed".to_string(),
            other => other.to_string(),
        };
    }

    match entity.state.as_str() {
        "on" => "open".to_string(),
        "off" => "closed".to_string(),
        other => other.to_string(),
    }
}

fn friendly_or_id(entity: &HaEntitySummary) -> String {
    entity
        .friendly_name
        .clone()
        .unwrap_or_else(|| entity.entity_id.clone())
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
            description: "Basic Home Assistant connectivity check".into(),
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
            description: "List summarized Home Assistant entities".into(),
            risk_tier: RiskTier::Tier0,
            requires_confirmation: false,
        }
    }

    async fn execute(&self, _: Value) -> Result<Value, AppError> {
        let raw = self.client.get_states().await?;
        let entities = state_array(&raw);
        let entities = state_array(&raw);

        Ok(json!({
            "count": entities.len(),
            "entities": entities
        }))
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
            description: "Get one Home Assistant entity by entity_id".into(),
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
            description: "Search Home Assistant entities by id, friendly name, or domain".into(),
            risk_tier: RiskTier::Tier0,
            requires_confirmation: false,
        }
    }

    async fn execute(&self, args: Value) -> Result<Value, AppError> {
        let args: HaSearchEntitiesArgs = serde_json::from_value(args)
            .map_err(|e| AppError::BadRequest(format!("invalid args: {e}")))?;

        let raw = self.client.get_states().await?;
        let query = args.query.to_lowercase();
        let limit = args.limit.unwrap_or(20);

        let results: Vec<HaEntitySummary> = state_array(&raw)
            .into_iter()
            .filter(|e| {
                e.entity_id.to_lowercase().contains(&query)
                    || e.friendly_name
                        .as_deref()
                        .unwrap_or("")
                        .to_lowercase()
                        .contains(&query)
                    || e.domain.to_lowercase().contains(&query)
            })
            .take(limit)
            .collect();

        Ok(json!({ "results": results }))
    }
}

pub struct HaOverviewTool {
    client: HomeAssistantClient,
}

impl HaOverviewTool {
    pub fn new(client: HomeAssistantClient) -> Self {
        Self { client }
    }
}

#[async_trait]
impl Tool for HaOverviewTool {
    fn descriptor(&self) -> ToolDescriptor {
        ToolDescriptor {
            name: "ha.get_overview".into(),
            description: "Return compact LLM-ready Home Assistant house overview".into(),
            risk_tier: RiskTier::Tier0,
            requires_confirmation: false,
        }
    }

    async fn execute(&self, _: Value) -> Result<Value, AppError> {
        let raw = self.client.get_states().await?;
        let entities = state_array(&raw);

        let people: Vec<Value> = entities
            .iter()
            .filter(|e| e.domain == "person")
            .map(|e| {
                json!({
                    "entity_id": e.entity_id,
                    "name": friendly_or_id(e),
                    "state": e.state
                })
            })
            .collect();

        let house_mode = entities
            .iter()
            .find(|e| e.entity_id == "input_select.house_mode")
            .map(|e| e.state.clone());

        let locks: Vec<Value> = entities
            .iter()
            .filter(|e| e.domain == "lock")
            .map(|e| {
                json!({
                    "entity_id": e.entity_id,
                    "name": friendly_or_id(e),
                    "state": e.state
                })
            })
            .collect();

        let doors: Vec<Value> = entities
            .iter()
            .filter(|e| is_door_entity(e))
            .map(|e| {
                json!({
                    "entity_id": e.entity_id,
                    "name": friendly_or_id(e),
                    "state": e.state,
                    "interpreted": interpret_binary_door_state(e)
                })
            })
            .collect();

        let vacuums: Vec<Value> = entities
            .iter()
            .filter(|e| e.domain == "vacuum")
            .map(|e| {
                json!({
                    "entity_id": e.entity_id,
                    "name": friendly_or_id(e),
                    "state": e.state,
                    "battery": e.attributes.get("battery_level").cloned()
                })
            })
            .collect();

        let weather: Vec<Value> = entities
            .iter()
            .filter(|e| e.domain == "weather")
            .map(|e| {
                json!({
                    "entity_id": e.entity_id,
                    "name": friendly_or_id(e),
                    "state": e.state,
                    "temperature": e.attributes.get("temperature").cloned(),
                    "temperature_unit": e.attributes.get("temperature_unit").cloned(),
                    "humidity": e.attributes.get("humidity").cloned(),
                    "wind_speed": e.attributes.get("wind_speed").cloned(),
                    "wind_speed_unit": e.attributes.get("wind_speed_unit").cloned()
                })
            })
            .collect();

        let media_players: Vec<Value> = entities
            .iter()
            .filter(|e| e.domain == "media_player")
            .map(|e| {
                json!({
                    "entity_id": e.entity_id,
                    "name": friendly_or_id(e),
                    "state": e.state
                })
            })
            .collect();

        let energy_devices: Vec<Value> = entities
            .iter()
            .filter(|e| {
                e.domain == "switch"
                    && (e.entity_id.to_lowercase().contains("washer")
                        || e.entity_id.to_lowercase().contains("dryer")
                        || friendly_or_id(e).to_lowercase().contains("washer")
                        || friendly_or_id(e).to_lowercase().contains("dryer"))
            })
            .map(|e| {
                json!({
                    "entity_id": e.entity_id,
                    "name": friendly_or_id(e),
                    "state": e.state
                })
            })
            .collect();

        let problems: Vec<Value> = entities
            .iter()
            .filter(|e| is_problem_entity(e))
            .map(|e| {
                json!({
                    "entity_id": e.entity_id,
                    "name": friendly_or_id(e),
                    "state": e.state,
                    "device_class": e.attributes.get("device_class").cloned()
                })
            })
            .collect();

        Ok(json!({
            "entity_count": entities.len(),
            "people": people,
            "house_mode": house_mode,
            "locks": locks,
            "doors": doors,
            "vacuums": vacuums,
            "weather": weather,
            "media_players": media_players,
            "energy_devices": energy_devices,
            "problems": problems
        }))
    }
}
