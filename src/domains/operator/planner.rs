use serde_json::json;
use uuid::Uuid;

use super::models::{
    OPERATOR_IMPLEMENTATION_TASK_SET, OPERATOR_PATCH_PLAN, OPERATOR_TASK_DIAGNOSTIC,
};
use crate::{
    op_tasks::models::{OpTask, OpWorkItem},
    services::llm_router::LlmRouter,
};

pub type PlannedWorkItem = OpWorkItem;

#[derive(Clone)]
pub struct OperatorTaskPlanner {
    llm_router: LlmRouter,
}

impl OperatorTaskPlanner {
    pub fn new(llm_router: LlmRouter) -> Self {
        Self { llm_router }
    }

    pub fn plan(&self, task: &OpTask, run_id: Uuid) -> Option<Vec<PlannedWorkItem>> {
        match task.task_type.as_str() {
            "operator.review_failed_task" => Some(self.plan_review_failed_task(task, run_id)),
            "operator.generate_patch_plan" => Some(self.plan_generate_patch_plan(task, run_id)),
            "operator.convert_recommendation_to_tasks" => {
                Some(self.plan_convert_recommendation_to_tasks(task, run_id))
            }
            "operator.escalate_to_chatgpt" | "system.escalate_to_chatgpt" => {
                Some(self.plan_chatgpt_escalation(task, run_id))
            }
            _ => None,
        }
    }

    fn plan_review_failed_task(&self, task: &OpTask, run_id: Uuid) -> Vec<PlannedWorkItem> {
        let mut load_run = OpWorkItem::planned(
            run_id,
            "load_failed_run",
            "Load the failed OpTaskRun being reviewed.",
            "repository",
            1,
        );
        load_run.tool_args_json = Some(json!({
            "run_id": task.input_json.get("run_id").cloned().unwrap_or_default()
        }));

        let load_task = OpWorkItem::planned(
            run_id,
            "load_task_definition",
            "Load the task definition for the failed run when requested.",
            "repository",
            2,
        );

        let load_artifacts = OpWorkItem::planned(
            run_id,
            "load_run_artifacts",
            "Load artifacts produced by the failed run when requested.",
            "repository",
            3,
        );

        let load_audit = OpWorkItem::planned(
            run_id,
            "load_recent_audit",
            "Load recent audited execution attempts linked to the failed run.",
            "audit",
            4,
        );

        let mut classify = OpWorkItem::planned(
            run_id,
            "classify_failure",
            "Classify the failure and gather evidence.",
            "model",
            5,
        );
        classify.model_purpose = Some("failure_classification".to_string());
        classify.model_name = Some(self.llm_router.task_extraction_model());

        let root_cause_description =
            "Analyze likely root cause from loaded run, work items, artifacts, and audit evidence.";
        let mut analyze = OpWorkItem::planned(
            run_id,
            "analyze_root_cause",
            root_cause_description,
            "model",
            6,
        );
        analyze.model_purpose = Some("root_cause_analysis".to_string());
        analyze.model_name = Some(self.llm_router.deep_model());

        let mut save = OpWorkItem::planned(
            run_id,
            "save_diagnostic_artifact",
            "Save an operator_task_diagnostic artifact with evidence and recommendations.",
            "artifact",
            7,
        );
        save.tool_args_json = Some(json!({
            "artifact_type": OPERATOR_TASK_DIAGNOSTIC,
            "read_only": true
        }));

        let mut summarize = OpWorkItem::planned(
            run_id,
            "summarize_operator_review",
            "Generate a final summary of the diagnostic run.",
            "model",
            8,
        );
        summarize.model_purpose = Some("final_summary".to_string());
        summarize.model_name = Some(self.llm_router.task_summary_model());

        vec![
            load_run,
            load_task,
            load_artifacts,
            load_audit,
            classify,
            analyze,
            save,
            summarize,
        ]
    }

    fn plan_generate_patch_plan(&self, task: &OpTask, run_id: Uuid) -> Vec<PlannedWorkItem> {
        let mut load = OpWorkItem::planned(
            run_id,
            "load_diagnostic_artifact",
            "Load the operator_task_diagnostic artifact that should drive patch planning.",
            "repository",
            1,
        );
        load.tool_args_json = Some(json!({
            "artifact_id": operator_source_artifact_id(task)
        }));

        let mut build = OpWorkItem::planned(
            run_id,
            "build_patch_plan",
            "Build a read-only operator_patch_plan from the diagnostic artifact.",
            "planning",
            2,
        );
        build.model_purpose = Some("patch_plan".to_string());
        build.model_name = Some(self.llm_router.coder_model());

        let mut save = OpWorkItem::planned(
            run_id,
            "save_patch_plan_artifact",
            "Save an operator_patch_plan artifact.",
            "artifact",
            3,
        );
        save.tool_args_json = Some(json!({
            "artifact_type": OPERATOR_PATCH_PLAN,
            "read_only": true
        }));

        let mut summarize = OpWorkItem::planned(
            run_id,
            "summarize_patch_plan",
            "Generate a final summary of the patch plan run.",
            "model",
            4,
        );
        summarize.model_purpose = Some("final_summary".to_string());
        summarize.model_name = Some(self.llm_router.task_summary_model());

        vec![load, build, save, summarize]
    }

    fn plan_convert_recommendation_to_tasks(
        &self,
        task: &OpTask,
        run_id: Uuid,
    ) -> Vec<PlannedWorkItem> {
        let mut load = OpWorkItem::planned(
            run_id,
            "load_patch_plan_artifact",
            "Load the operator_patch_plan artifact that should become implementation task specs.",
            "repository",
            1,
        );
        load.tool_args_json = Some(json!({
            "artifact_id": operator_source_artifact_id(task)
        }));

        let mut build = OpWorkItem::planned(
            run_id,
            "build_implementation_task_set",
            "Build a read-only operator_implementation_task_set artifact. No OpTasks are created.",
            "planning",
            2,
        );
        build.model_purpose = Some("implementation_task_planning".to_string());
        build.model_name = Some(self.llm_router.task_reasoning_model());

        let mut save = OpWorkItem::planned(
            run_id,
            "save_implementation_task_set_artifact",
            "Save an operator_implementation_task_set artifact.",
            "artifact",
            3,
        );
        save.tool_args_json = Some(json!({
            "artifact_type": OPERATOR_IMPLEMENTATION_TASK_SET,
            "approval_required": true,
            "read_only": true
        }));

        let mut summarize = OpWorkItem::planned(
            run_id,
            "summarize_implementation_task_set",
            "Generate a final summary of the implementation task set run.",
            "model",
            4,
        );
        summarize.model_purpose = Some("final_summary".to_string());
        summarize.model_name = Some(self.llm_router.task_summary_model());

        vec![load, build, save, summarize]
    }

    fn plan_chatgpt_escalation(&self, task: &OpTask, run_id: Uuid) -> Vec<PlannedWorkItem> {
        let mut collect = OpWorkItem::planned(
            run_id,
            "collect_escalation_context",
            "Collect local task, user request, profile context, and supplied context for manual ChatGPT escalation.",
            "context",
            1,
        );
        collect.tool_args_json = Some(task.input_json.clone());

        let mut redact = OpWorkItem::planned(
            run_id,
            "redact_escalation_context",
            "Redact sensitive values before preparing the escalation artifact.",
            "model",
            2,
        );
        redact.model_purpose = Some("escalation_redaction".to_string());
        redact.model_name = Some(self.llm_router.task_reasoning_model());

        let mut save = OpWorkItem::planned(
            run_id,
            "save_escalation_request",
            "Save a chatgpt_escalation_request artifact for manual copy/paste escalation.",
            "model",
            3,
        );
        save.model_purpose = Some("escalation_packet".to_string());
        save.model_name = Some(self.llm_router.task_writing_model());
        save.tool_args_json = Some(json!({
            "artifact_type": "chatgpt_escalation_request",
            "mode": task
                .input_json
                .get("mode")
                .and_then(|value| value.as_str())
                .unwrap_or("manual")
        }));

        vec![collect, redact, save]
    }
}

fn operator_source_artifact_id(task: &OpTask) -> serde_json::Value {
    task.input_json
        .get("artifact_id")
        .or_else(|| task.input_json.get("source_artifact_id"))
        .or_else(|| task.input_json.get("diagnostic_artifact_id"))
        .or_else(|| task.input_json.get("patch_plan_artifact_id"))
        .cloned()
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::LlmRouterConfig;
    use crate::op_tasks::models::{OpTask, OpTaskStatus};
    use chrono::Utc;
    use uuid::Uuid;

    fn planner() -> OperatorTaskPlanner {
        let router = LlmRouter::new(LlmRouterConfig {
            fast_model: "fast-test".to_string(),
            default_model: "default-test".to_string(),
            coder_model: "coder-test".to_string(),
            deep_model: "deep-test".to_string(),
            task_summary_model: "summary-test".to_string(),
            task_extraction_model: "extract-test".to_string(),
            task_reasoning_model: "reasoning-test".to_string(),
            task_writing_model: "writing-test".to_string(),
        });

        OperatorTaskPlanner::new(router)
    }

    fn task(task_type: &str) -> OpTask {
        OpTask {
            id: Uuid::new_v4(),
            profile_id: Uuid::new_v4(),
            task_type: task_type.to_string(),
            name: "operator test task".to_string(),
            description: None,
            input_json: json!({ "artifact_id": Uuid::new_v4() }),
            status: OpTaskStatus::Active,
            created_at: Utc::now(),
            updated_at: None,
        }
    }

    #[test]
    fn operator_review_failed_task_plan_is_ordered_and_populated() {
        let planner = planner();
        let run_id = Uuid::new_v4();
        let plan = planner
            .plan(&task("operator.review_failed_task"), run_id)
            .expect("planner result");

        let names: Vec<_> = plan.iter().map(|item| item.name.as_str()).collect();
        assert_eq!(
            names,
            vec![
                "load_failed_run",
                "load_task_definition",
                "load_run_artifacts",
                "load_recent_audit",
                "classify_failure",
                "analyze_root_cause",
                "save_diagnostic_artifact",
                "summarize_operator_review",
            ]
        );

        assert!(plan.iter().all(|item| !item.step_type.is_empty()));
        assert!(plan[4].model_name.is_some());
        assert!(plan[5].model_name.is_some());
        assert!(plan[7].model_name.is_some());
        assert_eq!(plan[4].run_id, run_id);
        assert_eq!(plan[0].order, 1);
        assert_eq!(plan[7].order, 8);
        assert_eq!(plan[4].run_id, run_id);
    }

    #[test]
    fn operator_generate_patch_plan_plan_has_planner_boundaries_and_models() {
        let planner = planner();
        let run_id = Uuid::new_v4();
        let plan = planner
            .plan(&task("operator.generate_patch_plan"), run_id)
            .expect("planner result");

        let names: Vec<_> = plan.iter().map(|item| item.name.as_str()).collect();
        assert_eq!(
            names,
            vec![
                "load_diagnostic_artifact",
                "build_patch_plan",
                "save_patch_plan_artifact",
                "summarize_patch_plan",
            ]
        );

        assert!(plan[0].tool_args_json.is_some());
        assert!(plan[1].model_purpose.as_deref() == Some("patch_plan"));
        assert!(plan[1].model_name.is_some());
        assert!(plan[3].model_name.is_some());
    }

    #[test]
    fn operator_convert_recommendation_to_tasks_plan_targets_implementation_tasks() {
        let planner = planner();
        let run_id = Uuid::new_v4();
        let plan = planner
            .plan(&task("operator.convert_recommendation_to_tasks"), run_id)
            .expect("planner result");

        let names: Vec<_> = plan.iter().map(|item| item.name.as_str()).collect();
        assert_eq!(
            names,
            vec![
                "load_patch_plan_artifact",
                "build_implementation_task_set",
                "save_implementation_task_set_artifact",
                "summarize_implementation_task_set",
            ]
        );
        assert!(plan[1].model_purpose.as_deref() == Some("implementation_task_planning"));
    }

    #[test]
    fn operator_escalation_plan_uses_context_and_redaction_steps() {
        let planner = planner();
        let run_id = Uuid::new_v4();
        let mut task = task("operator.escalate_to_chatgpt");
        task.input_json = json!({ "mode": "manual" });

        let plan = planner.plan(&task, run_id).expect("planner result");

        let names: Vec<_> = plan.iter().map(|item| item.name.as_str()).collect();
        assert_eq!(
            names,
            vec![
                "collect_escalation_context",
                "redact_escalation_context",
                "save_escalation_request"
            ]
        );
        assert!(plan[1].model_name.is_some());
    }

    #[test]
    fn operator_plan_returns_none_for_unknown_task_type() {
        let planner = planner();
        assert!(planner
            .plan(&task("operator.unknown"), Uuid::new_v4())
            .is_none());
    }
}
