use serde_json::json;
use uuid::Uuid;

use crate::services::llm_router::LlmRouter;

use super::models::{OpTask, OpWorkItem};

#[derive(Clone)]
pub struct TaskPlanner {
    llm_router: LlmRouter,
}

impl TaskPlanner {
    pub fn new(llm_router: LlmRouter) -> Self {
        Self { llm_router }
    }

    pub fn plan(&self, task: &OpTask, run_id: Uuid) -> Vec<OpWorkItem> {
        match task.task_type.as_str() {
            "system.status_report" => self.plan_system_status_report(run_id),
            "reader.read_url" => self.plan_reader_read_url(task, run_id),
            "reader.search_web" => self.plan_reader_search_web(task, run_id),
            "employment.search_opportunities" => self.plan_employment_search(task, run_id),
            _ => vec![OpWorkItem::planned(
                run_id,
                "unsupported_task",
                format!("No planner is registered for {}", task.task_type),
                "unsupported",
                1,
            )],
        }
    }

    fn plan_system_status_report(&self, run_id: Uuid) -> Vec<OpWorkItem> {
        let mut collect = OpWorkItem::planned(
            run_id,
            "collect_system_status",
            "Collect local operator system status.",
            "tool",
            1,
        );
        collect.tool_name = Some("system.get_status".to_string());
        collect.tool_args_json = Some(json!({}));

        let mut summarize = OpWorkItem::planned(
            run_id,
            "summarize_system_status",
            "Summarize collected status into a short report.",
            "model",
            2,
        );
        summarize.model_purpose = Some("task_summary".to_string());
        summarize.model_name = Some(self.llm_router.task_summary_model());

        vec![collect, summarize]
    }

    fn plan_reader_read_url(&self, task: &OpTask, run_id: Uuid) -> Vec<OpWorkItem> {
        let mut read = OpWorkItem::planned(
            run_id,
            "read_url",
            "Read and extract text from a URL.",
            "reader",
            1,
        );
        read.tool_name = Some("reader.read_url".to_string());
        read.tool_args_json = Some(task.input_json.clone());

        vec![read]
    }

    fn plan_reader_search_web(&self, task: &OpTask, run_id: Uuid) -> Vec<OpWorkItem> {
        let mut search = OpWorkItem::planned(
            run_id,
            "search_web",
            "Search the web and save result links.",
            "reader",
            1,
        );
        search.tool_name = Some("reader.search_web".to_string());
        search.tool_args_json = Some(task.input_json.clone());
        search.model_purpose = task
            .input_json
            .get("model_purpose")
            .and_then(|value| value.as_str())
            .map(|value| value.to_string());

        vec![search]
    }

    fn plan_employment_search(&self, task: &OpTask, run_id: Uuid) -> Vec<OpWorkItem> {
        let load_profile = OpWorkItem::planned(
            run_id,
            "load_profile_criteria",
            "Load employment profile criteria for search planning.",
            "profile",
            1,
        );

        let mut search = OpWorkItem::planned(
            run_id,
            "search_opportunities",
            "Search job opportunities based on profile criteria.",
            "reader",
            2,
        );
        search.tool_name = Some("reader.search_web".to_string());
        search.model_purpose = task
            .input_json
            .get("model_purpose")
            .and_then(|value| value.as_str())
            .map(|value| value.to_string());

        let mut create = OpWorkItem::planned(
            run_id,
            "create_opportunities",
            "Create or refresh opportunity records from search results when requested.",
            "repository",
            3,
        );
        create.tool_args_json = Some(json!({
            "create_opportunities": task
                .input_json
                .get("create_opportunities")
                .and_then(|value| value.as_bool())
                .unwrap_or(false)
        }));

        vec![load_profile, search, create]
    }
}
