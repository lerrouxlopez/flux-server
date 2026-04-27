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

    pub async fn find_by_id(&self, channel_id: Uuid) -> Result<Option<ChannelRow>, sqlx::Error> {
        let maybe = sqlx::query(
            r#"
            select id, organization_id, name, kind, created_at
            from channels
            where id = $1
            "#,
        )
        .bind(channel_id)
        .fetch_optional(&self.pool)
        .await?;

        let Some(row) = maybe else {
            return Ok(None);
        };

        let kind_str: String = row.try_get("kind")?;
        let kind = ChannelKind::try_from(kind_str.as_str()).map_err(|_| sqlx::Error::ColumnDecode {
            index: "kind".into(),
            source: Box::new(std::fmt::Error),
        })?;

        Ok(Some(ChannelRow {
            id: row.try_get("id")?,
            organization_id: row.try_get("organization_id")?,
            name: row.try_get("name")?,
            kind,
            created_at: row.try_get("created_at")?,
        }))
    }

    pub async fn update_channel(
        &self,
        channel_id: Uuid,
        name: Option<&str>,
        kind: Option<ChannelKind>,
    ) -> Result<Option<ChannelRow>, sqlx::Error> {
        let maybe = sqlx::query(
            r#"
            update channels
            set
              name = coalesce($2, name),
              kind = coalesce($3, kind)
            where id = $1
            returning id, organization_id, name, kind, created_at
            "#,
        )
        .bind(channel_id)
        .bind(name)
        .bind(kind.map(|k| k.as_str()))
        .fetch_optional(&self.pool)
        .await?;

        let Some(row) = maybe else {
            return Ok(None);
        };

        let kind_str: String = row.try_get("kind")?;
        let kind = ChannelKind::try_from(kind_str.as_str()).map_err(|_| sqlx::Error::ColumnDecode {
            index: "kind".into(),
            source: Box::new(std::fmt::Error),
        })?;

        Ok(Some(ChannelRow {
            id: row.try_get("id")?,
            organization_id: row.try_get("organization_id")?,
            name: row.try_get("name")?,
            kind,
            created_at: row.try_get("created_at")?,
        }))
    }

    pub async fn delete_channel(&self, channel_id: Uuid) -> Result<bool, sqlx::Error> {
        let res = sqlx::query(
            r#"
            delete from channels
            where id = $1
            "#,
        )
        .bind(channel_id)
        .execute(&self.pool)
        .await?;

        Ok(res.rows_affected() == 1)
    }
}
