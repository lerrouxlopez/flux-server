use sqlx::{PgPool, Row};
use time::OffsetDateTime;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct MembershipRow {
    pub organization_id: Uuid,
    pub user_id: Uuid,
    pub role: String,
    pub joined_at: OffsetDateTime,
}

#[derive(Clone)]
pub struct MembershipRepository {
    pool: PgPool,
}

impl MembershipRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn insert_membership(
        &self,
        organization_id: Uuid,
        user_id: Uuid,
        role: &str,
    ) -> Result<MembershipRow, sqlx::Error> {
        let row = sqlx::query(
            r#"
            insert into organization_members (organization_id, user_id, role)
            values ($1, $2, $3)
            returning organization_id, user_id, role, joined_at
            "#,
        )
        .bind(organization_id)
        .bind(user_id)
        .bind(role)
        .fetch_one(&self.pool)
        .await?;

        Ok(MembershipRow {
            organization_id: row.try_get("organization_id")?,
            user_id: row.try_get("user_id")?,
            role: row.try_get("role")?,
            joined_at: row.try_get("joined_at")?,
        })
    }

    pub async fn find_membership(
        &self,
        organization_id: Uuid,
        user_id: Uuid,
    ) -> Result<Option<MembershipRow>, sqlx::Error> {
        let maybe = sqlx::query(
            r#"
            select organization_id, user_id, role, joined_at
            from organization_members
            where organization_id = $1 and user_id = $2
            "#,
        )
        .bind(organization_id)
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await?;

        let Some(row) = maybe else {
            return Ok(None);
        };

        Ok(Some(MembershipRow {
            organization_id: row.try_get("organization_id")?,
            user_id: row.try_get("user_id")?,
            role: row.try_get("role")?,
            joined_at: row.try_get("joined_at")?,
        }))
    }

    pub async fn find_first_org_for_user(
        &self,
        user_id: Uuid,
    ) -> Result<Option<Uuid>, sqlx::Error> {
        let maybe = sqlx::query(
            r#"
            select organization_id
            from organization_members
            where user_id = $1
            order by joined_at asc
            limit 1
            "#,
        )
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(maybe.map(|row| row.try_get::<Uuid, _>("organization_id")).transpose()?)
    }

    pub async fn list_members_with_users(
        &self,
        organization_id: Uuid,
        limit: i64,
    ) -> Result<Vec<(MembershipRow, String, String)>, sqlx::Error> {
        let rows = sqlx::query(
            r#"
            select
              om.organization_id,
              om.user_id,
              om.role,
              om.joined_at,
              u.email,
              u.display_name
            from organization_members om
            join users u on u.id = om.user_id
            where om.organization_id = $1
            order by om.joined_at asc
            limit $2
            "#,
        )
        .bind(organization_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter()
            .map(|row| {
                Ok((
                    MembershipRow {
                        organization_id: row.try_get("organization_id")?,
                        user_id: row.try_get("user_id")?,
                        role: row.try_get("role")?,
                        joined_at: row.try_get("joined_at")?,
                    },
                    row.try_get("email")?,
                    row.try_get("display_name")?,
                ))
            })
            .collect()
    }

    pub async fn list_orgs_for_user(
        &self,
        user_id: Uuid,
        limit: i64,
    ) -> Result<Vec<Uuid>, sqlx::Error> {
        let rows = sqlx::query(
            r#"
            select organization_id
            from organization_members
            where user_id = $1
            order by joined_at asc
            limit $2
            "#,
        )
        .bind(user_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter()
            .map(|row| row.try_get::<Uuid, _>("organization_id"))
            .collect()
    }
}
