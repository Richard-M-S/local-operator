use serde_json::json;
use uuid::Uuid;

use crate::{domains::operator::planner::OperatorTaskPlanner, services::llm_router::LlmRouter};

use super::models::{OpTask, OpWorkItem};

#[derive(Clone)]
pub struct TaskPlanner {
    llm_router: LlmRouter,
    operator_planner: OperatorTaskPlanner,
}

impl TaskPlanner {
    pub fn new(llm_router: LlmRouter) -> Self {
        Self {
            operator_planner: OperatorTaskPlanner::new(llm_router.clone()),
            llm_router,
        }
    }

    pub fn plan(&self, task: &OpTask, run_id: Uuid) -> Vec<OpWorkItem> {
        if let Some(work_items) = self.operator_planner.plan(task, run_id) {
            return work_items;
        }

        match task.task_type.as_str() {
            "system.status_report" => self.plan_system_status_report(run_id),
            "reader.read_url" => self.plan_reader_read_url(task, run_id),
            "reader.search_web" => self.plan_reader_search_web(task, run_id),
            "employment.search_opportunities" => self.plan_employment_search(task, run_id),
            "artifact.summarize" => self.plan_artifact_summary(task, run_id),
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

    fn plan_artifact_summary(&self, task: &OpTask, run_id: Uuid) -> Vec<OpWorkItem> {
        let mut summarize = OpWorkItem::planned(
            run_id,
            "summarize_artifact",
            "Summarize or explain a prior artifact for continuation.",
            "model",
            1,
        );
        summarize.model_purpose = Some("artifact_continuation".to_string());
        summarize.model_name = Some(self.llm_router.task_summary_model());
        summarize.tool_args_json = Some(task.input_json.clone());

        vec![summarize]
    }

    fn plan_employment_search(&self, task: &OpTask, run_id: Uuid) -> Vec<OpWorkItem> {
        let load_profile = OpWorkItem::planned(
            run_id,
            "load_profile_context",
            "Load employment profile and saved context for search planning.",
            "profile",
            1,
        );

        let build_query = OpWorkItem::planned(
            run_id,
            "build_search_query",
            "Build the search query from task input, user request, and profile defaults.",
            "planning",
            2,
        );

        let mut search = OpWorkItem::planned(
            run_id,
            "run_search",
            "Run web/job search and save raw results.",
            "reader",
            3,
        );
        search.tool_name = Some("reader.search_web".to_string());
        search.model_purpose = task
            .input_json
            .get("model_purpose")
            .and_then(|value| value.as_str())
            .map(|value| value.to_string());

        let read_urls = OpWorkItem::planned(
            run_id,
            "read_result_urls",
            "Read promising result URLs into readable page artifacts where possible.",
            "reader",
            4,
        );

        let mut extract = OpWorkItem::planned(
            run_id,
            "extract_candidates",
            "Extract structured job/opportunity candidates from readable pages and search snippets.",
            "model",
            5,
        );
        extract.model_purpose = Some("task_extraction".to_string());
        extract.model_name = Some(self.llm_router.task_extraction_model());

        let mut score = OpWorkItem::planned(
            run_id,
            "score_matches",
            "Score and classify extracted opportunity candidates.",
            "model",
            6,
        );
        score.model_purpose = Some("task_reasoning".to_string());
        score.model_name = Some(self.llm_router.task_reasoning_model());

        let mut create = OpWorkItem::planned(
            run_id,
            "create_opportunities",
            "Create or refresh opportunity records from search results when requested.",
            "repository",
            7,
        );
        create.tool_args_json = Some(json!({
            "create_opportunities": task
                .input_json
                .get("create_opportunities")
                .and_then(|value| value.as_bool())
                .unwrap_or(false)
        }));

        let summarize = OpWorkItem::planned(
            run_id,
            "summarize_run",
            "Generate a final run summary from created artifacts and scored matches.",
            "summary",
            8,
        );

        vec![
            load_profile,
            build_query,
            search,
            read_urls,
            extract,
            score,
            create,
            summarize,
        ]
    }
}
