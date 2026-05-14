use crate::domains::employment::models::{
    EmploymentOpportunity, EmploymentOpportunitySearch, EmploymentOpportunityStatus,
};
use chrono::{DateTime, Utc};
use sqlx::{FromRow, QueryBuilder, Sqlite, SqlitePool};
use uuid::Uuid;

#[derive(Clone)]
pub struct EmploymentRepository {
    pool: SqlitePool,
}

impl EmploymentRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn create_opportunity(
        &self,
        opportunity: EmploymentOpportunity,
    ) -> anyhow::Result<EmploymentOpportunity> {
        let extracted_json = opportunity
            .extracted_json
            .as_ref()
            .map(serde_json::to_string)
            .transpose()?;

        sqlx::query(
            r#"
            INSERT INTO employment_opportunities (
                id,
                source_url,
                source_name,
                title,
                company,
                location,
                remote_type,
                salary_min,
                salary_max,
                description_text,
                extracted_json,
                fit_score,
                status,
                skip_reason,
                source_artifact_id,
                first_seen_at,
                last_seen_at
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17)
            "#,
        )
        .bind(opportunity.id.to_string())
        .bind(&opportunity.source_url)
        .bind(&opportunity.source_name)
        .bind(&opportunity.title)
        .bind(&opportunity.company)
        .bind(&opportunity.location)
        .bind(&opportunity.remote_type)
        .bind(opportunity.salary_min)
        .bind(opportunity.salary_max)
        .bind(&opportunity.description_text)
        .bind(extracted_json)
        .bind(opportunity.fit_score)
        .bind(opportunity.status.as_str())
        .bind(&opportunity.skip_reason)
        .bind(opportunity.source_artifact_id.map(|id| id.to_string()))
        .bind(opportunity.first_seen_at.to_rfc3339())
        .bind(opportunity.last_seen_at.to_rfc3339())
        .execute(&self.pool)
        .await?;

        Ok(opportunity)
    }

    pub async fn get_opportunity(
        &self,
        opportunity_id: Uuid,
    ) -> anyhow::Result<Option<EmploymentOpportunity>> {
        let row = sqlx::query_as::<_, EmploymentOpportunityRow>(
            r#"
            SELECT
                id,
                source_url,
                source_name,
                title,
                company,
                location,
                remote_type,
                salary_min,
                salary_max,
                description_text,
                extracted_json,
                fit_score,
                status,
                skip_reason,
                source_artifact_id,
                first_seen_at,
                last_seen_at
            FROM employment_opportunities
            WHERE id = ?1
            "#,
        )
        .bind(opportunity_id.to_string())
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(Into::into))
    }

    pub async fn find_opportunity_by_source_url(
        &self,
        source_url: &str,
    ) -> anyhow::Result<Option<EmploymentOpportunity>> {
        let row = sqlx::query_as::<_, EmploymentOpportunityRow>(
            r#"
            SELECT
                id,
                source_url,
                source_name,
                title,
                company,
                location,
                remote_type,
                salary_min,
                salary_max,
                description_text,
                extracted_json,
                fit_score,
                status,
                skip_reason,
                source_artifact_id,
                first_seen_at,
                last_seen_at
            FROM employment_opportunities
            WHERE source_url = ?1
            ORDER BY last_seen_at DESC
            LIMIT 1
            "#,
        )
        .bind(source_url)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(Into::into))
    }

    pub async fn list_opportunities(
        &self,
        search: EmploymentOpportunitySearch,
    ) -> anyhow::Result<Vec<EmploymentOpportunity>> {
        let limit = search.limit.unwrap_or(50).clamp(1, 200);
        let offset = search.offset.unwrap_or(0).max(0);

        let mut query = QueryBuilder::<Sqlite>::new(
            r#"
            SELECT
                id,
                source_url,
                source_name,
                title,
                company,
                location,
                remote_type,
                salary_min,
                salary_max,
                description_text,
                extracted_json,
                fit_score,
                status,
                skip_reason,
                source_artifact_id,
                first_seen_at,
                last_seen_at
            FROM employment_opportunities
            WHERE 1 = 1
            "#,
        );

        if let Some(status) = search.status {
            query.push(" AND status = ");
            query.push_bind(status.as_str());
        }

        if let Some(company) = search.company {
            query.push(" AND company LIKE ");
            query.push_bind(format!("%{}%", company));
        }

        if let Some(title) = search.title {
            query.push(" AND title LIKE ");
            query.push_bind(format!("%{}%", title));
        }

        if let Some(remote_type) = search.remote_type {
            query.push(" AND remote_type = ");
            query.push_bind(remote_type);
        }

        if let Some(min_fit_score) = search.min_fit_score {
            query.push(" AND fit_score >= ");
            query.push_bind(min_fit_score);
        }

        if let Some(source_artifact_id) = search.source_artifact_id {
            query.push(" AND source_artifact_id = ");
            query.push_bind(source_artifact_id.to_string());
        }

        query.push(" ORDER BY last_seen_at DESC LIMIT ");
        query.push_bind(limit);

        query.push(" OFFSET ");
        query.push_bind(offset);

        let rows = query
            .build_query_as::<EmploymentOpportunityRow>()
            .fetch_all(&self.pool)
            .await?;

        Ok(rows.into_iter().map(Into::into).collect())
    }

    pub async fn update_opportunity(
        &self,
        opportunity: EmploymentOpportunity,
    ) -> anyhow::Result<EmploymentOpportunity> {
        let extracted_json = opportunity
            .extracted_json
            .as_ref()
            .map(serde_json::to_string)
            .transpose()?;

        sqlx::query(
            r#"
            UPDATE employment_opportunities
            SET
                source_url = ?1,
                source_name = ?2,
                title = ?3,
                company = ?4,
                location = ?5,
                remote_type = ?6,
                salary_min = ?7,
                salary_max = ?8,
                description_text = ?9,
                extracted_json = ?10,
                fit_score = ?11,
                status = ?12,
                skip_reason = ?13,
                source_artifact_id = ?14,
                first_seen_at = ?15,
                last_seen_at = ?16
            WHERE id = ?17
            "#,
        )
        .bind(&opportunity.source_url)
        .bind(&opportunity.source_name)
        .bind(&opportunity.title)
        .bind(&opportunity.company)
        .bind(&opportunity.location)
        .bind(&opportunity.remote_type)
        .bind(opportunity.salary_min)
        .bind(opportunity.salary_max)
        .bind(&opportunity.description_text)
        .bind(extracted_json)
        .bind(opportunity.fit_score)
        .bind(opportunity.status.as_str())
        .bind(&opportunity.skip_reason)
        .bind(opportunity.source_artifact_id.map(|id| id.to_string()))
        .bind(opportunity.first_seen_at.to_rfc3339())
        .bind(opportunity.last_seen_at.to_rfc3339())
        .bind(opportunity.id.to_string())
        .execute(&self.pool)
        .await?;

        Ok(opportunity)
    }

    pub async fn update_opportunity_status(
        &self,
        opportunity_id: Uuid,
        status: EmploymentOpportunityStatus,
        skip_reason: Option<String>,
    ) -> anyhow::Result<Option<EmploymentOpportunity>> {
        sqlx::query(
            r#"
            UPDATE employment_opportunities
            SET
                status = ?1,
                skip_reason = ?2,
                last_seen_at = ?3
            WHERE id = ?4
            "#,
        )
        .bind(status.as_str())
        .bind(skip_reason)
        .bind(Utc::now().to_rfc3339())
        .bind(opportunity_id.to_string())
        .execute(&self.pool)
        .await?;

        self.get_opportunity(opportunity_id).await
    }

    pub async fn touch_opportunity_seen_at(
        &self,
        opportunity_id: Uuid,
    ) -> anyhow::Result<Option<EmploymentOpportunity>> {
        sqlx::query(
            r#"
            UPDATE employment_opportunities
            SET last_seen_at = ?1
            WHERE id = ?2
            "#,
        )
        .bind(Utc::now().to_rfc3339())
        .bind(opportunity_id.to_string())
        .execute(&self.pool)
        .await?;

        self.get_opportunity(opportunity_id).await
    }
}

#[derive(FromRow)]
struct EmploymentOpportunityRow {
    id: String,
    source_url: String,
    source_name: Option<String>,
    title: Option<String>,
    company: Option<String>,
    location: Option<String>,
    remote_type: Option<String>,
    salary_min: Option<i64>,
    salary_max: Option<i64>,
    description_text: Option<String>,
    extracted_json: Option<String>,
    fit_score: Option<i64>,
    status: String,
    skip_reason: Option<String>,
    source_artifact_id: Option<String>,
    first_seen_at: String,
    last_seen_at: String,
}

impl From<EmploymentOpportunityRow> for EmploymentOpportunity {
    fn from(row: EmploymentOpportunityRow) -> Self {
        Self {
            id: Uuid::parse_str(&row.id).unwrap(),
            source_url: row.source_url,
            source_name: row.source_name,
            title: row.title,
            company: row.company,
            location: row.location,
            remote_type: row.remote_type,
            salary_min: row.salary_min,
            salary_max: row.salary_max,
            description_text: row.description_text,
            extracted_json: row
                .extracted_json
                .and_then(|value| serde_json::from_str(&value).ok()),
            fit_score: row.fit_score,
            status: parse_opportunity_status(&row.status),
            skip_reason: row.skip_reason,
            source_artifact_id: row
                .source_artifact_id
                .and_then(|id| Uuid::parse_str(&id).ok()),
            first_seen_at: parse_datetime(&row.first_seen_at),
            last_seen_at: parse_datetime(&row.last_seen_at),
        }
    }
}

fn parse_datetime(value: &str) -> DateTime<Utc> {
    value.parse().unwrap_or_else(|_| Utc::now())
}

fn parse_opportunity_status(value: &str) -> EmploymentOpportunityStatus {
    match value {
        "discovered" | "Discovered" => EmploymentOpportunityStatus::Discovered,
        "parsed" | "Parsed" => EmploymentOpportunityStatus::Parsed,
        "scored" | "Scored" => EmploymentOpportunityStatus::Scored,
        "skipped" | "Skipped" => EmploymentOpportunityStatus::Skipped,
        "queued_for_review" | "QueuedForReview" => EmploymentOpportunityStatus::QueuedForReview,
        "closed" | "Closed" => EmploymentOpportunityStatus::Closed,
        other => EmploymentOpportunityStatus::Other(other.to_string()),
    }
}

trait EmploymentOpportunityStatusAsStr {
    fn as_str(&self) -> String;
}

impl EmploymentOpportunityStatusAsStr for EmploymentOpportunityStatus {
    fn as_str(&self) -> String {
        match self {
            EmploymentOpportunityStatus::Discovered => "discovered".to_string(),
            EmploymentOpportunityStatus::Parsed => "parsed".to_string(),
            EmploymentOpportunityStatus::Scored => "scored".to_string(),
            EmploymentOpportunityStatus::Skipped => "skipped".to_string(),
            EmploymentOpportunityStatus::QueuedForReview => "queued_for_review".to_string(),
            EmploymentOpportunityStatus::Closed => "closed".to_string(),
            EmploymentOpportunityStatus::Other(value) => value.clone(),
        }
    }
}
