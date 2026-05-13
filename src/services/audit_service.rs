use chrono::Utc;
use serde::Serialize;
use sqlx::{FromRow, SqlitePool};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize)]
pub struct AuditItem {
    pub ts: String,
    pub action: String,
    pub ok: bool,
}

#[derive(Clone)]
pub struct AuditService {
    db: SqlitePool,
}

impl AuditService {
    pub fn new(db: SqlitePool) -> Self {
        Self { db }
    }

    pub async fn record_tool_call(&self, action: &str, ok: bool) -> anyhow::Result<()> {
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
        .bind("tool_call")
        .bind(0_i32)
        .bind(if ok { 1 } else { 0 })
        .bind(serde_json::json!([{ "tool": action }]).to_string())
        .bind(serde_json::json!({ "ok": ok }).to_string())
        .bind(if ok {
            "tool call succeeded"
        } else {
            "tool call failed"
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
}

#[derive(FromRow)]
struct AuditRow {
    created_at: String,
    raw_input: String,
    allowed: i32,
}
