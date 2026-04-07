use crate::db::audit_repo::AuditRepo;
use crate::models::audit::AuditEntry;
use sqlx::SqlitePool;

#[derive(Clone)]
pub struct AuditService {
    repo: AuditRepo,
}

impl AuditService {
    pub fn new(pool: SqlitePool) -> Self {
        Self {
            repo: AuditRepo::new(pool),
        }
    }

    pub async fn save(&self, entry: AuditEntry) -> anyhow::Result<()> {
        self.repo.insert(entry).await
    }

    pub async fn recent(&self, limit: i64) -> anyhow::Result<Vec<AuditEntry>> {
        self.repo.recent(limit).await
    }
}