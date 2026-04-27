use sqlx::{PgPool, Row};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::repositories::MembershipRow;
use serde_json::Value;

#[derive(Debug, Clone)]
pub struct OrganizationRow {
    pub id: Uuid,
    pub slug: String,
    pub name: String,
    pub created_at: OffsetDateTime,
}

#[derive(Clone)]
pub struct OrgRepository {
    pool: PgPool,
}

impl OrgRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn create_org_with_owner(
        &self,
        org_id: Uuid,
        slug: &str,
        name: &str,
        user_id: Uuid,
        role: &str,
    ) -> Result<(OrganizationRow, MembershipRow), sqlx::Error> {
        let mut tx = self.pool.begin().await?;

        let org_row = sqlx::query(
            r#"
            insert into organizations (id, slug, name)
            values ($1, $2, $3)
            returning id, slug, name, created_at
            "#,
        )
        .bind(org_id)
        .bind(slug)
        .bind(name)
        .fetch_one(&mut *tx)
        .await?;

        let membership_row = sqlx::query(
            r#"
            insert into organization_members (organization_id, user_id, role)
            values ($1, $2, $3)
            returning organization_id, user_id, role, joined_at
            "#,
        )
        .bind(org_id)
        .bind(user_id)
        .bind(role)
        .fetch_one(&mut *tx)
        .await?;

        tx.commit().await?;

        Ok((
            OrganizationRow {
                id: org_row.try_get("id")?,
                slug: org_row.try_get("slug")?,
                name: org_row.try_get("name")?,
                created_at: org_row.try_get("created_at")?,
            },
            MembershipRow {
                organization_id: membership_row.try_get("organization_id")?,
                user_id: membership_row.try_get("user_id")?,
                role: membership_row.try_get("role")?,
                joined_at: membership_row.try_get("joined_at")?,
            },
        ))
    }

    pub async fn create_org_with_defaults(
        &self,
        org_id: Uuid,
        slug: &str,
        name: &str,
        creator_id: Uuid,
        owner_permissions: &Value,
        member_permissions: &Value,
    ) -> Result<OrganizationRow, sqlx::Error> {
        let mut tx = self.pool.begin().await?;

        let org_row = sqlx::query(
            r#"
            insert into organizations (id, slug, name)
            values ($1, $2, $3)
            returning id, slug, name, created_at
            "#,
        )
        .bind(org_id)
        .bind(slug)
        .bind(name)
        .fetch_one(&mut *tx)
        .await?;

        // Creator membership
        sqlx::query(
            r#"
            insert into organization_members (organization_id, user_id, role)
            values ($1, $2, 'owner')
            "#,
        )
        .bind(org_id)
        .bind(creator_id)
        .execute(&mut *tx)
        .await?;

        // Default roles
        sqlx::query(
            r#"
            insert into roles (id, organization_id, name, permissions)
            values
              ($1, $3, 'owner', $4),
              ($2, $3, 'member', $5)
            "#,
        )
        .bind(Uuid::now_v7())
        .bind(Uuid::now_v7())
        .bind(org_id)
        .bind(owner_permissions)
        .bind(member_permissions)
        .execute(&mut *tx)
        .await?;

        // Default channels
        sqlx::query(
            r#"
            insert into channels (id, organization_id, name, kind)
            values
              ($1, $4, 'general', 'text'),
              ($2, $4, 'announcements', 'announcement'),
              ($3, $4, 'General Voice', 'voice')
            "#,
        )
        .bind(Uuid::now_v7())
        .bind(Uuid::now_v7())
        .bind(Uuid::now_v7())
        .bind(org_id)
        .execute(&mut *tx)
        .await?;

        // Default branding
        sqlx::query(
            r#"
            insert into branding_profiles (organization_id, app_name)
            values ($1, $2)
            "#,
        )
        .bind(org_id)
        .bind(name)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;

        Ok(OrganizationRow {
            id: org_row.try_get("id")?,
            slug: org_row.try_get("slug")?,
            name: org_row.try_get("name")?,
            created_at: org_row.try_get("created_at")?,
        })
    }

    pub async fn insert_org(
        &self,
        id: Uuid,
        slug: &str,
        name: &str,
    ) -> Result<OrganizationRow, sqlx::Error> {
        let row = sqlx::query(
            r#"
            insert into organizations (id, slug, name)
            values ($1, $2, $3)
            returning id, slug, name, created_at
            "#,
        )
        .bind(id)
        .bind(slug)
        .bind(name)
        .fetch_one(&self.pool)
        .await?;

        Ok(OrganizationRow {
            id: row.try_get("id")?,
            slug: row.try_get("slug")?,
            name: row.try_get("name")?,
            created_at: row.try_get("created_at")?,
        })
    }

    pub async fn find_by_id(&self, org_id: Uuid) -> Result<Option<OrganizationRow>, sqlx::Error> {
        let maybe = sqlx::query(
            r#"
            select id, slug, name, created_at
            from organizations
            where id = $1
            "#,
        )
        .bind(org_id)
        .fetch_optional(&self.pool)
        .await?;

        let Some(row) = maybe else {
            return Ok(None);
        };

        Ok(Some(OrganizationRow {
            id: row.try_get("id")?,
            slug: row.try_get("slug")?,
            name: row.try_get("name")?,
            created_at: row.try_get("created_at")?,
        }))
    }
}
