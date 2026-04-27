use sqlx::{PgPool, Row};
use time::OffsetDateTime;
use uuid::Uuid;
use domain::ChannelKind;

#[derive(Debug, Clone)]
pub struct ChannelRow {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub name: String,
    pub kind: ChannelKind,
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
        kind: ChannelKind,
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
        .bind(kind.as_str())
        .fetch_one(&self.pool)
        .await?;

        let kind_str: String = row.try_get("kind")?;
        let kind = ChannelKind::try_from(kind_str.as_str())
            .map_err(|_| sqlx::Error::ColumnDecode {
                index: "kind".into(),
                source: Box::new(std::fmt::Error),
            })?;

        Ok(ChannelRow {
            id: row.try_get("id")?,
            organization_id: row.try_get("organization_id")?,
            name: row.try_get("name")?,
            kind,
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
                let kind_str: String = row.try_get("kind")?;
                let kind = ChannelKind::try_from(kind_str.as_str())
                    .map_err(|_| sqlx::Error::ColumnDecode {
                        index: "kind".into(),
                        source: Box::new(std::fmt::Error),
                    })?;
                Ok(ChannelRow {
                    id: row.try_get("id")?,
                    organization_id: row.try_get("organization_id")?,
                    name: row.try_get("name")?,
                    kind,
                    created_at: row.try_get("created_at")?,
                })
            })
            .collect()
    }
}
