use sqlx::{PgPool, Row};
use time::OffsetDateTime;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct InviteRow {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub token: String,
    pub role: String,
    pub created_by: Uuid,
    pub created_at: OffsetDateTime,
    pub expires_at: Option<OffsetDateTime>,
    pub used_at: Option<OffsetDateTime>,
    pub used_by: Option<Uuid>,
}

#[derive(Clone)]
pub struct InviteRepository {
    pool: PgPool,
}

impl InviteRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn insert_invite(
        &self,
        id: Uuid,
        organization_id: Uuid,
        token: &str,
        role: &str,
        created_by: Uuid,
        expires_at: Option<OffsetDateTime>,
    ) -> Result<InviteRow, sqlx::Error> {
        let row = sqlx::query(
            r#"
            insert into organization_invites (id, organization_id, token, role, created_by, expires_at)
            values ($1, $2, $3, $4, $5, $6)
            returning id, organization_id, token, role, created_by, created_at, expires_at, used_at, used_by
            "#,
        )
        .bind(id)
        .bind(organization_id)
        .bind(token)
        .bind(role)
        .bind(created_by)
        .bind(expires_at)
        .fetch_one(&self.pool)
        .await?;

        Ok(InviteRow {
            id: row.try_get("id")?,
            organization_id: row.try_get("organization_id")?,
            token: row.try_get("token")?,
            role: row.try_get("role")?,
            created_by: row.try_get("created_by")?,
            created_at: row.try_get("created_at")?,
            expires_at: row.try_get("expires_at")?,
            used_at: row.try_get("used_at")?,
            used_by: row.try_get("used_by")?,
        })
    }
}

