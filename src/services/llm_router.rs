use crate::{
    config::LlmRouterConfig,
    models::llm::{LlmRoute, LlmRouteDecision},
};

#[derive(Clone)]
pub struct LlmRouter {
    config: LlmRouterConfig,
}

impl LlmRouter {
    pub fn new(config: LlmRouterConfig) -> Self {
        Self { config }
    }

    pub fn route(&self, input: &str) -> LlmRouteDecision {
        let text = input.trim().to_lowercase();

        let needs_home_context = text.contains("home")
            || text.contains("house")
            || text.contains("front door")
            || text.contains("door")
            || text.contains("lock")
            || text.contains("weather")
            || text.contains("garage")
            || text.contains("attic")
            || text.contains("vacuum")
            || text.contains("bottom maid")
            || text.contains("washer")
            || text.contains("dryer")
            || text.contains("sensor");

        let looks_like_code = text.contains("rust")
            || text.contains("cargo")
            || text.contains("compile")
            || text.contains("error")
            || text.contains("stack trace")
            || text.contains("yaml")
            || text.contains("apex")
            || text.contains("lwc")
            || text.contains("code")
            || text.contains("function")
            || text.contains("class");

        let looks_deep = text.contains("architecture")
            || text.contains("design")
            || text.contains("strategy")
            || text.contains("investigate")
            || text.contains("look into")
            || text.contains("analyze")
            || text.contains("review")
            || text.contains("risk")
            || text.contains("security");

        let needs_escalation = text.contains("search the internet")
            || text.contains("latest")
            || text.contains("current news")
            || text.contains("chatgpt")
            || text.contains("escalate");

        if needs_escalation {
            return LlmRouteDecision {
                route: LlmRoute::Escalate,
                model: self.config.deep_model.clone(),
                needs_home_context,
                reason: "Request likely needs external/current information or explicit escalation."
                    .to_string(),
            };
        }

        if looks_like_code {
            return LlmRouteDecision {
                route: LlmRoute::Coder,
                model: self.config.coder_model.clone(),
                needs_home_context,
                reason: "Request appears code/config related.".to_string(),
            };
        }

        if looks_deep {
            return LlmRouteDecision {
                route: LlmRoute::Deep,
                model: self.config.deep_model.clone(),
                needs_home_context,
                reason: "Request appears to require deeper multi-step reasoning.".to_string(),
            };
        }

        if input.len() < 120 && !needs_home_context {
            return LlmRouteDecision {
                route: LlmRoute::Fast,
                model: self.config.fast_model.clone(),
                needs_home_context,
                reason: "Short general request; fast model is sufficient.".to_string(),
            };
        }

        LlmRouteDecision {
            route: LlmRoute::Default,
            model: self.config.default_model.clone(),
            needs_home_context,
            reason: "Default operator route.".to_string(),
        }
    }
}