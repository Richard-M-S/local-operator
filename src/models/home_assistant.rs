use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HaEntitySummary {
    pub entity_id: String,
    pub state: String,
    pub friendly_name: Option<String>,
    pub area: Option<String>,
    pub domain: String,
    pub attributes: Value,
}

#[derive(Debug, Clone, Deserialize)]
pub struct HaGetEntityArgs {
    pub entity_id: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct HaSearchEntitiesArgs {
    pub query: String,
    pub limit: Option<usize>,
}
