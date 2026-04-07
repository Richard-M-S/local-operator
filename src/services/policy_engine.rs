use crate::config::PolicyConfig;
use crate::models::policy::{PolicyDecision, RiskTier};

#[derive(Clone)]
pub struct PolicyEngine {
    cfg: PolicyConfig,
}

impl PolicyEngine {
    pub fn new(cfg: PolicyConfig) -> Self {
        Self { cfg }
    }

    pub fn evaluate(&self, risk_tier: RiskTier, confirm: bool) -> PolicyDecision {
        match risk_tier {
            RiskTier::Tier0 => PolicyDecision {
                allowed: true,
                requires_confirmation: false,
                risk_tier,
                reason: None,
            },
            RiskTier::Tier1 => PolicyDecision {
                allowed: self.cfg.allow_tier1_without_confirm || confirm,
                requires_confirmation: !self.cfg.allow_tier1_without_confirm && !confirm,
                risk_tier,
                reason: if self.cfg.allow_tier1_without_confirm || confirm {
                    None
                } else {
                    Some("Tier 1 action requires confirmation".into())
                },
            },
            RiskTier::Tier2 => PolicyDecision {
                allowed: self.cfg.allow_tier2_without_confirm || confirm,
                requires_confirmation: !self.cfg.allow_tier2_without_confirm && !confirm,
                risk_tier,
                reason: if self.cfg.allow_tier2_without_confirm || confirm {
                    None
                } else {
                    Some("Tier 2 action requires confirmation".into())
                },
            },
            RiskTier::Tier3 => PolicyDecision {
                allowed: !self.cfg.block_tier3 && confirm,
                requires_confirmation: true,
                risk_tier,
                reason: Some("Tier 3 actions are blocked in Operator v0.1".into()),
            },
        }
    }
}