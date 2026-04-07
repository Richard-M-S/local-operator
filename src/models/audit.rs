use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    pub request_id: Uuid,
    pub created_at: DateTime<Utc>,
    pub raw_input: String,
    pub parsed_intent: Option<String>,
    pub risk_tier: i32,
    pub allowed: bool,
    pub actions_json: Option<String>,
    pub results_json: Option<String>,
    pub final_message: Option<String>,
}use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    pub request_id: Uuid,
    pub created_at: DateTime<Utc>,
    pub raw_input: String,
    pub parsed_intent: Option<String>,
    pub risk_tier: i32,
    pub allowed: bool,
    pub actions_json: Option<String>,
    pub results_json: Option<String>,
    pub final_message: Option<String>,
}