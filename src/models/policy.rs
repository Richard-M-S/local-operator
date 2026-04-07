use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum RiskTier {
    Tier0,
    Tier1,
    Tier2,
    Tier3,
}

impl RiskTier {
    pub fn as_i32(&self) -> i32 {
        match self {
            RiskTier::Tier0 => 0,
            RiskTier::Tier1 => 1,
            RiskTier::Tier2 => 2,
            RiskTier::Tier3 => 3,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyDecision {
    pub allowed: bool,
    pub requires_confirmation: bool,
    pub risk_tier: RiskTier,
    pub reason: Option<String>,
}