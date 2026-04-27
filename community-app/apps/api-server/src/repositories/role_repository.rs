use serde_json::Value;
use sqlx::{PgPool, Row};
use time::OffsetDateTime;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct RoleRow {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub name: String,
    pub permissions: Value,
    pub created_at: OffsetDateTime,
}

#[derive(Clone)]
pub struct RoleRepository {
    pool: PgPool,
}

impl RoleRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn insert_role(
        &self,
        id: Uuid,
        organization_id: Uuid,
        name: &str,
        permissions: &Value,
    ) -> Result<RoleRow, sqlx::Error> {
        let row = sqlx::query(
            r#"
            insert into roles (id, organization_id, name, permissions)
            values ($1, $2, $3, $4)
            returning id, organization_id, name, permissions, created_at
            "#,
        )
        .bind(id)
        .bind(organization_id)
        .bind(name)
        .bind(permissions)
        .fetch_one(&self.pool)
        .await?;

        Ok(RoleRow {
            id: row.try_get("id")?,
            organization_id: row.try_get("organization_id")?,
            name: row.try_get("name")?,
            permissions: row.try_get("permissions")?,
            created_at: row.try_get("created_at")?,
        })
    }

    pub async fn find_by_org_and_name(
        &self,
        organization_id: Uuid,
        name: &str,
    ) -> Result<Option<RoleRow>, sqlx::Error> {
        let maybe = sqlx::query(
            r#"
            select id, organization_id, name, permissions, created_at
            from roles
            where organization_id = $1 and name = $2
            "#,
        )
        .bind(organization_id)
        .bind(name)
        .fetch_optional(&self.pool)
        .await?;

        let Some(row) = maybe else {
            return Ok(None);
        };

        Ok(Some(RoleRow {
            id: row.try_get("id")?,
            organization_id: row.try_get("organization_id")?,
            name: row.try_get("name")?,
            permissions: row.try_get("permissions")?,
            created_at: row.try_get("created_at")?,
        }))
    }
}

