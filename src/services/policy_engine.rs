use crate::{config::PolicyConfig, error::AppError, models::tool::RiskTier};

#[derive(Debug, Clone)]
pub struct PolicyEngine {
    config: PolicyConfig,
}

impl PolicyEngine {
    pub fn new(config: PolicyConfig) -> Self {
        Self { config }
    }

    pub fn check_tool_execution(&self, risk_tier: RiskTier, confirm: bool) -> Result<(), AppError> {
        match risk_tier {
            RiskTier::Tier0 => Ok(()),
            RiskTier::Tier1 => {
                if self.config.allow_tier1_without_confirm || confirm {
                    Ok(())
                } else {
                    Err(AppError::PolicyDenied(
                        "tier 1 action requires confirmation".into(),
                    ))
                }
            }
            RiskTier::Tier2 => {
                if self.config.allow_tier2_without_confirm || confirm {
                    Ok(())
                } else {
                    Err(AppError::PolicyDenied(
                        "tier 2 action requires confirmation".into(),
                    ))
                }
            }
            RiskTier::Tier3 => {
                if self.config.block_tier3 {
                    Err(AppError::PolicyDenied(
                        "tier 3 actions are blocked in v0.1".into(),
                    ))
                } else if confirm {
                    Ok(())
                } else {
                    Err(AppError::PolicyDenied(
                        "tier 3 action requires confirmation".into(),
                    ))
                }
            }
        }
    }
}
