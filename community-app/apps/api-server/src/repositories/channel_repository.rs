use sqlx::{PgPool, Row};
use time::OffsetDateTime;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct ChannelRow {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub name: String,
    pub kind: String,
    pub created_at: OffsetDateTime,
}

#[derive(Clone)]
pub struct ChannelRepository {
    pool: PgPool,
}

impl ChannelRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn insert_channel(
        &self,
        id: Uuid,
        organization_id: Uuid,
        name: &str,
        kind: &str,
    ) -> Result<ChannelRow, sqlx::Error> {
        let row = sqlx::query(
            r#"
            insert into channels (id, organization_id, name, kind)
            values ($1, $2, $3, $4)
            returning id, organization_id, name, kind, created_at
            "#,
        )
        .bind(id)
        .bind(organization_id)
        .bind(name)
        .bind(kind)
        .fetch_one(&self.pool)
        .await?;

        Ok(ChannelRow {
            id: row.try_get("id")?,
            organization_id: row.try_get("organization_id")?,
            name: row.try_get("name")?,
            kind: row.try_get("kind")?,
            created_at: row.try_get("created_at")?,
        })
    }

    pub async fn list_channels_by_org(
        &self,
        organization_id: Uuid,
        limit: i64,
    ) -> Result<Vec<ChannelRow>, sqlx::Error> {
        let rows = sqlx::query(
            r#"
            select id, organization_id, name, kind, created_at
            from channels
            where organization_id = $1
            order by created_at asc
            limit $2
            "#,
        )
        .bind(organization_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter()
            .map(|row| {
                Ok(ChannelRow {
                    id: row.try_get("id")?,
                    organization_id: row.try_get("organization_id")?,
                    name: row.try_get("name")?,
                    kind: row.try_get("kind")?,
                    created_at: row.try_get("created_at")?,
                })
            })
            .collect()
    }
}

