use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::{FromRow, SqlitePool};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize)]
pub struct AuditItem {
    pub ts: String,
    pub action: String,
    pub ok: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct AuditExecutionSummary {
    pub created_at: String,
    pub raw_input: String,
    pub parsed_intent: Option<String>,
    pub risk_tier: i32,
    pub allowed: bool,
    pub final_message: Option<String>,
    pub execution_type: Option<String>,
    pub name: Option<String>,
    pub task_id: Option<Uuid>,
    pub run_id: Option<Uuid>,
    pub work_item_id: Option<Uuid>,
    pub model_purpose: Option<String>,
    pub policy_decision: Option<String>,
    pub success: Option<bool>,
    pub error: Option<String>,
}

#[derive(Clone)]
pub struct AuditService {
    db: SqlitePool,
}

impl AuditService {
    pub fn new(db: SqlitePool) -> Self {
        Self { db }
    }

    pub async fn record_execution_attempt(
        &self,
        record: ExecutionAuditRecord,
    ) -> anyhow::Result<()> {
        let ok = record.allowed && record.success;
        let action = format!("{}:{}", record.execution_type, record.name);
        let actions_json = serde_json::to_string(&record)?;
        let results_json = serde_json::json!({
            "success": record.success,
            "error": record.error,
            "output_artifact_ids": record.output_artifact_ids,
        })
        .to_string();

        sqlx::query(
            r#"
            INSERT INTO audit_log (
              request_id, created_at, raw_input, parsed_intent, risk_tier,
              allowed, actions_json, results_json, final_message
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
            "#,
        )
        .bind(Uuid::new_v4().to_string())
        .bind(Utc::now().to_rfc3339())
        .bind(action)
        .bind(
            record
                .model_purpose
                .clone()
                .unwrap_or_else(|| record.execution_type.clone()),
        )
        .bind(record.risk_tier.unwrap_or(0))
        .bind(if ok { 1 } else { 0 })
        .bind(actions_json)
        .bind(results_json)
        .bind(if record.success {
            "execution succeeded".to_string()
        } else {
            record
                .error
                .unwrap_or_else(|| "execution failed".to_string())
        })
        .execute(&self.db)
        .await?;

        Ok(())
    }

    pub async fn recent(&self, limit: i64) -> anyhow::Result<Vec<AuditItem>> {
        let rows = sqlx::query_as::<_, AuditRow>(
            r#"
            SELECT created_at, raw_input, allowed
            FROM audit_log
            ORDER BY created_at DESC
            LIMIT ?1
            "#,
        )
        .bind(limit)
        .fetch_all(&self.db)
        .await?;

        Ok(rows
            .into_iter()
            .map(|row| AuditItem {
                ts: row.created_at,
                action: row.raw_input,
                ok: row.allowed != 0,
            })
            .collect())
    }

    pub async fn recent_for_run(
        &self,
        run_id: Uuid,
        limit: i64,
    ) -> anyhow::Result<Vec<AuditExecutionSummary>> {
        let rows = sqlx::query_as::<_, DetailedAuditRow>(
            r#"
            SELECT created_at, raw_input, parsed_intent, risk_tier,
                   allowed, actions_json, results_json, final_message
            FROM audit_log
            ORDER BY created_at DESC
            LIMIT ?1
            "#,
        )
        .bind(limit.clamp(1, 200))
        .fetch_all(&self.db)
        .await?;

        Ok(rows
            .into_iter()
            .filter_map(AuditExecutionSummary::from_row)
            .filter(|item| item.run_id == Some(run_id))
            .collect())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionAuditRecord {
    pub execution_type: String,
    pub name: String,
    pub task_id: Option<Uuid>,
    pub run_id: Option<Uuid>,
    pub work_item_id: Option<Uuid>,
    pub model_purpose: Option<String>,
    pub input_summary: Option<String>,
    pub args_json: Option<Value>,
    pub policy_decision: String,
    pub risk_tier: Option<i32>,
    pub allowed: bool,
    pub success: bool,
    pub error: Option<String>,
    pub output_artifact_ids: Vec<Uuid>,
}

#[derive(FromRow)]
struct AuditRow {
    created_at: String,
    raw_input: String,
    allowed: i32,
}

#[derive(FromRow)]
struct DetailedAuditRow {
    created_at: String,
    raw_input: String,
    parsed_intent: Option<String>,
    risk_tier: i32,
    allowed: i32,
    actions_json: Option<String>,
    results_json: Option<String>,
    final_message: Option<String>,
}

impl AuditExecutionSummary {
    fn from_row(row: DetailedAuditRow) -> Option<Self> {
        let actions = row
            .actions_json
            .as_deref()
            .and_then(|value| serde_json::from_str::<ExecutionAuditRecord>(value).ok());
        let results = row
            .results_json
            .as_deref()
            .and_then(|value| serde_json::from_str::<Value>(value).ok());

        Some(Self {
            created_at: row.created_at,
            raw_input: row.raw_input,
            parsed_intent: row.parsed_intent,
            risk_tier: row.risk_tier,
            allowed: row.allowed != 0,
            final_message: row.final_message,
            execution_type: actions.as_ref().map(|record| record.execution_type.clone()),
            name: actions.as_ref().map(|record| record.name.clone()),
            task_id: actions.as_ref().and_then(|record| record.task_id),
            run_id: actions.as_ref().and_then(|record| record.run_id),
            work_item_id: actions.as_ref().and_then(|record| record.work_item_id),
            model_purpose: actions
                .as_ref()
                .and_then(|record| record.model_purpose.clone()),
            policy_decision: actions
                .as_ref()
                .map(|record| record.policy_decision.clone()),
            success: actions.as_ref().map(|record| record.success),
            error: actions
                .as_ref()
                .and_then(|record| record.error.clone())
                .or_else(|| {
                    results
                        .as_ref()
                        .and_then(|value| value.get("error"))
                        .and_then(|value| value.as_str())
                        .map(str::to_string)
                }),
        })
    }
}
