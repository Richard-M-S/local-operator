use crate::models::audit::AuditEntry;
use sqlx::SqlitePool;

#[derive(Clone)]
pub struct AuditRepo {
    pool: SqlitePool,
}

impl AuditRepo {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn insert(&self, entry: AuditEntry) -> anyhow::Result<()> {
        sqlx::query(
            r#"
            INSERT INTO audit_log (
              request_id, created_at, raw_input, parsed_intent, risk_tier,
              allowed, actions_json, results_json, final_message
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
            "#,
        )
        .bind(entry.request_id.to_string())
        .bind(entry.created_at.to_rfc3339())
        .bind(entry.raw_input)
        .bind(entry.parsed_intent)
        .bind(entry.risk_tier)
        .bind(if entry.allowed { 1 } else { 0 })
        .bind(entry.actions_json)
        .bind(entry.results_json)
        .bind(entry.final_message)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn recent(&self, limit: i64) -> anyhow::Result<Vec<AuditEntry>> {
        let rows = sqlx::query_as::<_, AuditRow>(
            r#"
            SELECT request_id, created_at, raw_input, parsed_intent, risk_tier,
                   allowed, actions_json, results_json, final_message
            FROM audit_log
            ORDER BY created_at DESC
            LIMIT ?1
            "#,
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(Into::into).collect())
    }
}

#[derive(sqlx::FromRow)]
struct AuditRow {
    request_id: String,
    created_at: String,
    raw_input: String,
    parsed_intent: Option<String>,
    risk_tier: i32,
    allowed: i32,
    actions_json: Option<String>,
    results_json: Option<String>,
    final_message: Option<String>,
}

impl From<AuditRow> for AuditEntry {
    fn from(row: AuditRow) -> Self {
        Self {
            request_id: row.request_id.parse().unwrap(),
            created_at: row.created_at.parse().unwrap(),
            raw_input: row.raw_input,
            parsed_intent: row.parsed_intent,
            risk_tier: row.risk_tier,
            allowed: row.allowed != 0,
            actions_json: row.actions_json,
            results_json: row.results_json,
            final_message: row.final_message,
        }
    }
}