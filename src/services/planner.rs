use serde_json::json;
use uuid::Uuid;

use crate::models::plan::{ExecutionPlan, PlannedAction};
use crate::models::policy::RiskTier;

pub struct Planner;

impl Planner {
    pub fn build(input: &str) -> ExecutionPlan {
        let normalized = input.trim().to_lowercase();

        if normalized == "status" || normalized == "what is unhealthy right now?" {
            return ExecutionPlan {
                request_id: Uuid::new_v4(),
                raw_input: input.to_string(),
                parsed_intent: "status_summary".into(),
                risk_tier: RiskTier::Tier0,
                actions: vec![
                    PlannedAction {
                        tool: "system.get_status".into(),
                        args: json!({}),
                    },
                    PlannedAction {
                        tool: "docker.list_containers".into(),
                        args: json!({}),
                    },
                    PlannedAction {
                        tool: "ha.get_summary".into(),
                        args: json!({}),
                    },
                ],
                requires_confirmation: false,
            };
        }

        if let Some(name) = normalized.strip_prefix("restart ") {
            return ExecutionPlan {
                request_id: Uuid::new_v4(),
                raw_input: input.to_string(),
                parsed_intent: "restart_service".into(),
                risk_tier: RiskTier::Tier1,
                actions: vec![PlannedAction {
                    tool: "docker.restart_container".into(),
                    args: json!({ "name": name.trim() }),
                }],
                requires_confirmation: false,
            };
        }

        if normalized == "home summary" {
            return ExecutionPlan {
                request_id: Uuid::new_v4(),
                raw_input: input.to_string(),
                parsed_intent: "home_summary".into(),
                risk_tier: RiskTier::Tier0,
                actions: vec![PlannedAction {
                    tool: "ha.get_summary".into(),
                    args: json!({}),
                }],
                requires_confirmation: false,
            };
        }

        ExecutionPlan {
            request_id: Uuid::new_v4(),
            raw_input: input.to_string(),
            parsed_intent: "unknown".into(),
            risk_tier: RiskTier::Tier0,
            actions: vec![],
            requires_confirmation: false,
        }
    }
}