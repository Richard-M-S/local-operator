use serde::{Deserialize, Serialize};

use super::policy::RiskTier;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlannedAction {
    pub tool: String,
    pub args: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionPlan {
    pub request_id: uuid::Uuid,
    pub raw_input: String,
    pub parsed_intent: String,
    pub risk_tier: RiskTier,
    pub actions: Vec<PlannedAction>,
    pub requires_confirmation: bool,
}