use chrono::Utc;
use serde::Serialize;
use sqlx::SqlitePool;

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
        let _ = &self.db;

        let _item = AuditItem {
            ts: Utc::now().to_rfc3339(),
            action: action.to_string(),
            ok,
        };

        Ok(())
    }

    pub async fn recent(&self, _limit: i64) -> anyhow::Result<Vec<AuditItem>> {
        Ok(vec![])
    }
}