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

pub struct HaEnergyHvacSnapshotTool {
    client: HomeAssistantClient,
}

impl HaEnergyHvacSnapshotTool {
    pub fn new(client: HomeAssistantClient) -> Self {
        Self { client }
    }
}

#[async_trait]
impl Tool for HaEnergyHvacSnapshotTool {
    fn descriptor(&self) -> ToolDescriptor {
        ToolDescriptor {
            name: "ha.get_energy_hvac_snapshot".to_string(),
            description: "Return a normalized read-only Home Assistant snapshot for HVAC, weather, power, battery, and energy-cost planning".to_string(),
            risk_tier: RiskTier::Tier0,
            requires_confirmation: false,
        }
    }

    async fn execute(&self, _args: Value) -> Result<Value, AppError> {
        let token = std::env::var(&self.token_env)
            .map_err(|_| AppError::Internal(format!("missing env var {}", self.token_env)))?;

        let url = format!("{}/api/states", self.base_url.trim_end_matches('/'));

        let client = reqwest::Client::new();
        let resp = client
            .get(url)
            .bearer_auth(token)
            .send()
            .await
            .map_err(|e| AppError::Internal(format!("home assistant request failed: {e}")))?;

        let status = resp.status();

        if !status.is_success() {
            return Err(AppError::Internal(format!(
                "home assistant returned status {}",
                status
            )));
        }

        let states: Vec<Value> = resp
            .json()
            .await
            .map_err(|e| AppError::Internal(format!("unable to parse HA states: {e}")))?;

        fn entity_id(v: &Value) -> &str {
            v.get("entity_id").and_then(Value::as_str).unwrap_or("")
        }

        fn domain(id: &str) -> &str {
            id.split('.').next().unwrap_or("")
        }

        fn friendly_name(v: &Value) -> String {
            v.pointer("/attributes/friendly_name")
                .and_then(Value::as_str)
                .unwrap_or_else(|| entity_id(v))
                .to_string()
        }

        fn state_string(v: &Value) -> String {
            v.get("state")
                .and_then(Value::as_str)
                .unwrap_or("unknown")
                .to_string()
        }

        fn unit(v: &Value) -> Option<String> {
            v.pointer("/attributes/unit_of_measurement")
                .and_then(Value::as_str)
                .map(|s| s.to_string())
        }

        fn device_class(v: &Value) -> Option<String> {
            v.pointer("/attributes/device_class")
                .and_then(Value::as_str)
                .map(|s| s.to_string())
        }

        fn state_class(v: &Value) -> Option<String> {
            v.pointer("/attributes/state_class")
                .and_then(Value::as_str)
                .map(|s| s.to_string())
        }

        fn compact_entity(v: &Value) -> Value {
            json!({
                "entity_id": entity_id(v),
                "name": friendly_name(v),
                "state": state_string(v),
                "unit": unit(v),
                "device_class": device_class(v),
                "state_class": state_class(v),
                "last_changed": v.get("last_changed").cloned().unwrap_or(Value::Null),
                "last_updated": v.get("last_updated").cloned().unwrap_or(Value::Null),
            })
        }

        let mut climates = Vec::new();
        let mut temperature_sensors = Vec::new();
        let mut humidity_sensors = Vec::new();
        let mut weather = Vec::new();
        let mut power = Vec::new();
        let mut energy = Vec::new();
        let mut battery = Vec::new();
        let mut energy_price = Vec::new();
        let mut helpers = Vec::new();

        for state in &states {
            let id = entity_id(state);
            let dom = domain(id);
            let name_l = friendly_name(state).to_lowercase();
            let id_l = id.to_lowercase();
            let dc = device_class(state).unwrap_or_default();

            match dom {
                "climate" => {
                    climates.push(json!({
                        "entity_id": id,
                        "name": friendly_name(state),
                        "state": state_string(state),
                        "current_temperature": state.pointer("/attributes/current_temperature").cloned().unwrap_or(Value::Null),
                        "temperature": state.pointer("/attributes/temperature").cloned().unwrap_or(Value::Null),
                        "target_temp_low": state.pointer("/attributes/target_temp_low").cloned().unwrap_or(Value::Null),
                        "target_temp_high": state.pointer("/attributes/target_temp_high").cloned().unwrap_or(Value::Null),
                        "hvac_action": state.pointer("/attributes/hvac_action").cloned().unwrap_or(Value::Null),
                        "hvac_modes": state.pointer("/attributes/hvac_modes").cloned().unwrap_or(Value::Null),
                        "fan_mode": state.pointer("/attributes/fan_mode").cloned().unwrap_or(Value::Null),
                        "last_changed": state.get("last_changed").cloned().unwrap_or(Value::Null),
                    }));
                }
                "weather" => {
                    weather.push(json!({
                        "entity_id": id,
                        "name": friendly_name(state),
                        "state": state_string(state),
                        "temperature": state.pointer("/attributes/temperature").cloned().unwrap_or(Value::Null),
                        "humidity": state.pointer("/attributes/humidity").cloned().unwrap_or(Value::Null),
                        "pressure": state.pointer("/attributes/pressure").cloned().unwrap_or(Value::Null),
                        "wind_speed": state.pointer("/attributes/wind_speed").cloned().unwrap_or(Value::Null),
                        "forecast": state.pointer("/attributes/forecast").cloned().unwrap_or(Value::Null),
                        "last_changed": state.get("last_changed").cloned().unwrap_or(Value::Null),
                    }));
                }
                "input_boolean" | "input_number" | "input_select" | "schedule" | "automation" => {
                    if id_l.contains("hvac")
                        || id_l.contains("thermostat")
                        || id_l.contains("energy")
                        || id_l.contains("battery")
                        || id_l.contains("power")
                        || name_l.contains("hvac")
                        || name_l.contains("thermostat")
                        || name_l.contains("energy")
                        || name_l.contains("battery")
                        || name_l.contains("power")
                    {
                        helpers.push(compact_entity(state));
                    }
                }
                "sensor" => {
                    if dc == "temperature"
                        || id_l.contains("temp")
                        || name_l.contains("temperature")
                    {
                        temperature_sensors.push(compact_entity(state));
                    } else if dc == "humidity" || id_l.contains("humidity") {
                        humidity_sensors.push(compact_entity(state));
                    } else if dc == "power"
                        || id_l.contains("power")
                        || unit(state).as_deref() == Some("W")
                    {
                        power.push(compact_entity(state));
                    } else if dc == "energy"
                        || id_l.contains("energy")
                        || unit(state).as_deref() == Some("kWh")
                    {
                        energy.push(compact_entity(state));
                    } else if dc == "battery" || id_l.contains("battery") {
                        battery.push(compact_entity(state));
                    } else if id_l.contains("price")
                        || id_l.contains("rate")
                        || id_l.contains("tariff")
                        || name_l.contains("price")
                        || name_l.contains("rate")
                        || name_l.contains("tariff")
                    {
                        energy_price.push(compact_entity(state));
                    }
                }
                _ => {}
            }
        }

        Ok(json!({
            "snapshot_type": "energy_hvac",
            "source": "home_assistant",
            "counts": {
                "climate": climates.len(),
                "temperature_sensors": temperature_sensors.len(),
                "humidity_sensors": humidity_sensors.len(),
                "weather": weather.len(),
                "power": power.len(),
                "energy": energy.len(),
                "battery": battery.len(),
                "energy_price": energy_price.len(),
                "helpers": helpers.len()
            },
            "climate": climates,
            "temperature_sensors": temperature_sensors,
            "humidity_sensors": humidity_sensors,
            "weather": weather,
            "power": power,
            "energy": energy,
            "battery": battery,
            "energy_price": energy_price,
            "helpers": helpers
        }))
    }
}
