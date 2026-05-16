use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use crate::{
    op_tasks::models::{ArtifactSearch, OpTask, OpTaskRun, TaskArtifact},
    services::audit_service::AuditExecutionSummary,
};

pub const OPERATOR_TASK_DIAGNOSTIC: &str = "operator_task_diagnostic";
pub const OPERATOR_GAP_ANALYSIS: &str = "operator_gap_analysis";
pub const OPERATOR_TASK_TYPE_SPEC: &str = "operator_task_type_spec";
pub const OPERATOR_TOOL_SPEC: &str = "operator_tool_spec";
pub const OPERATOR_PATCH_PLAN: &str = "operator_patch_plan";
pub const OPERATOR_TEST_PLAN: &str = "operator_test_plan";
pub const OPERATOR_OPENAPI_REVIEW: &str = "operator_openapi_review";
pub const OPERATOR_IMPLEMENTATION_TASK_SET: &str = "operator_implementation_task_set";
pub const CHATGPT_ESCALATION_REQUEST: &str = "chatgpt_escalation_request";
pub const CHATGPT_ESCALATION_RESPONSE: &str = "chatgpt_escalation_response";

pub const OPERATOR_ARTIFACT_TYPES: &[&str] = &[
    OPERATOR_TASK_DIAGNOSTIC,
    OPERATOR_GAP_ANALYSIS,
    OPERATOR_TASK_TYPE_SPEC,
    OPERATOR_TOOL_SPEC,
    OPERATOR_PATCH_PLAN,
    OPERATOR_TEST_PLAN,
    OPERATOR_OPENAPI_REVIEW,
    OPERATOR_IMPLEMENTATION_TASK_SET,
    CHATGPT_ESCALATION_REQUEST,
    CHATGPT_ESCALATION_RESPONSE,
];

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct OperatorReviewFailedTaskInput {
    pub run_id: Uuid,
    #[serde(default = "default_true")]
    pub include_task: bool,
    #[serde(default = "default_true")]
    pub include_artifacts: bool,
    #[serde(default = "default_true")]
    pub include_recent_audit: bool,
    #[serde(default)]
    pub include_repo_context: bool,
    #[serde(default)]
    pub escalate_if_needed: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct OperatorGeneratePatchPlanInput {
    #[serde(alias = "source_artifact_id", alias = "diagnostic_artifact_id")]
    pub artifact_id: Uuid,
    #[serde(default)]
    pub title: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct OperatorConvertRecommendationToTasksInput {
    #[serde(alias = "source_artifact_id", alias = "patch_plan_artifact_id")]
    pub artifact_id: Uuid,
}

#[derive(Clone, Debug)]
pub struct FailedTaskReviewContext {
    pub task: Option<OpTask>,
    pub run: OpTaskRun,
    pub artifacts: Vec<TaskArtifact>,
    pub audit_entries: Vec<AuditExecutionSummary>,
    pub input: OperatorReviewFailedTaskInput,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct OperatorTaskStateQuery {
    pub profile_id: Option<Uuid>,
    pub task_id: Option<Uuid>,
    pub run_id: Option<Uuid>,
    pub artifact_id: Option<Uuid>,
    pub artifact_type: Option<String>,
    pub source_url: Option<String>,
    pub include_content: Option<bool>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Clone, Debug, Serialize)]
pub struct OperatorTaskStateSnapshot {
    pub filters: OperatorTaskStateQuery,
    pub tasks: Vec<OpTask>,
    pub runs: Vec<OpTaskRun>,
    pub artifacts: Vec<TaskArtifact>,
    pub note: String,
}

impl From<&OperatorTaskStateQuery> for ArtifactSearch {
    fn from(query: &OperatorTaskStateQuery) -> Self {
        Self {
            profile_id: query.profile_id,
            run_id: query.run_id,
            task_id: query.task_id,
            artifact_type: query.artifact_type.clone(),
            source_url: query.source_url.clone(),
            include_content: query.include_content,
            limit: query.limit,
            offset: query.offset,
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct OperatorTaskDiagnostic {
    pub schema_version: String,
    pub diagnostic_type: String,
    pub source: OperatorDiagnosticSource,
    pub failure_classification: OperatorFailureClassification,
    pub summary: String,
    pub evidence: Vec<OperatorDiagnosticEvidence>,
    pub recommended_actions: Vec<OperatorRecommendedAction>,
    pub next_actions: Vec<String>,
    pub review_context: Value,
    pub read_only: bool,
    pub actions_executed_by_local_operator: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct OperatorDiagnosticSource {
    pub task_id: Uuid,
    pub run_id: Uuid,
}

#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum OperatorFailureClassification {
    BadSearchResults,
    ToolError,
    ModelError,
    MissingContext,
    BadTaskInput,
    PolicyDenied,
    CodeBug,
    Unknown,
}

impl OperatorFailureClassification {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::BadSearchResults => "bad_search_results",
            Self::ToolError => "tool_error",
            Self::ModelError => "model_error",
            Self::MissingContext => "missing_context",
            Self::BadTaskInput => "bad_task_input",
            Self::PolicyDenied => "policy_denied",
            Self::CodeBug => "code_bug",
            Self::Unknown => "unknown",
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct OperatorDiagnosticEvidence {
    pub kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub artifact_id: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub work_item_id: Option<Uuid>,
    pub detail: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<Value>,
}

#[derive(Clone, Debug, Serialize)]
pub struct OperatorRecommendedAction {
    pub title: String,
    pub priority: OperatorRecommendationPriority,
    pub task_type: String,
}

#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum OperatorRecommendationPriority {
    #[allow(dead_code)]
    Low,
    Medium,
    High,
}

#[derive(Clone, Debug)]
pub struct OperatorDiagnosticArtifact {
    pub name: String,
    pub artifact_type: String,
    pub metadata: Value,
    pub content_text: String,
    pub content_json: Value,
}

#[derive(Clone, Debug, Serialize)]
pub struct OperatorPatchPlan {
    pub schema_version: String,
    pub source_artifact_id: Option<Uuid>,
    pub title: String,
    pub summary: String,
    pub files_to_change: Vec<String>,
    pub changes: Vec<OperatorPatchPlanChange>,
    pub risks: Vec<String>,
    pub tests: Vec<String>,
    pub read_only: bool,
    pub actions_executed_by_local_operator: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct OperatorPatchPlanChange {
    pub file: String,
    pub change: String,
}

#[allow(dead_code)]
#[derive(Clone, Debug, Serialize)]
pub struct OperatorTaskSpec {
    pub task_type: String,
    pub name: String,
    pub input_json: Value,
}

#[derive(Clone, Debug, Serialize)]
pub struct OperatorImplementationTaskSet {
    pub schema_version: String,
    pub source_artifact_id: Uuid,
    pub tasks: Vec<OperatorTaskSpec>,
    pub approval_required: bool,
    pub read_only: bool,
    pub actions_executed_by_local_operator: Vec<String>,
}

#[allow(dead_code)]
#[derive(Clone, Debug, Serialize)]
pub struct OperatorEscalationRequestPacket {
    pub schema_version: String,
    pub created_at: DateTime<Utc>,
    pub diagnostic: Value,
    pub desired_output: String,
    pub actions_executed_by_local_operator: Vec<String>,
}

fn default_true() -> bool {
    true
}
