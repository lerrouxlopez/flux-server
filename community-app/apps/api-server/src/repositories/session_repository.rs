use sqlx::{PgPool, Row};
use time::OffsetDateTime;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct SessionRow {
    pub id: Uuid,
    pub user_id: Uuid,
    pub refresh_token_hash: String,
    pub created_at: OffsetDateTime,
    pub expires_at: OffsetDateTime,
    pub last_used_at: Option<OffsetDateTime>,
    pub revoked_at: Option<OffsetDateTime>,
}

#[derive(Clone)]
pub struct SessionRepository {
    pool: PgPool,
}

impl SessionRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn insert_session(
        &self,
        id: Uuid,
        user_id: Uuid,
        refresh_token_hash: &str,
        expires_at: OffsetDateTime,
        user_agent: Option<&str>,
        ip: Option<&str>,
    ) -> Result<SessionRow, sqlx::Error> {
        let row = sqlx::query(
            r#"
            insert into user_sessions (id, user_id, refresh_token_hash, expires_at, user_agent, ip)
            values ($1, $2, $3, $4, $5, $6::inet)
            returning id, user_id, refresh_token_hash, created_at, expires_at, last_used_at, revoked_at
            "#,
        )
        .bind(id)
        .bind(user_id)
        .bind(refresh_token_hash)
        .bind(expires_at)
        .bind(user_agent)
        .bind(ip)
        .fetch_one(&self.pool)
        .await?;

        Ok(SessionRow {
            id: row.try_get("id")?,
            user_id: row.try_get("user_id")?,
            refresh_token_hash: row.try_get("refresh_token_hash")?,
            created_at: row.try_get("created_at")?,
            expires_at: row.try_get("expires_at")?,
            last_used_at: row.try_get("last_used_at")?,
            revoked_at: row.try_get("revoked_at")?,
        })
    }

    pub async fn find_by_refresh_hash(
        &self,
        refresh_token_hash: &str,
    ) -> Result<Option<SessionRow>, sqlx::Error> {
        let maybe_row = sqlx::query(
            r#"
            select id, user_id, refresh_token_hash, created_at, expires_at, last_used_at, revoked_at
            from user_sessions
            where refresh_token_hash = $1
            "#,
        )
        .bind(refresh_token_hash)
        .fetch_optional(&self.pool)
        .await?;

        let Some(row) = maybe_row else {
            return Ok(None);
        };

        Ok(Some(SessionRow {
            id: row.try_get("id")?,
            user_id: row.try_get("user_id")?,
            refresh_token_hash: row.try_get("refresh_token_hash")?,
            created_at: row.try_get("created_at")?,
            expires_at: row.try_get("expires_at")?,
            last_used_at: row.try_get("last_used_at")?,
            revoked_at: row.try_get("revoked_at")?,
        }))
    }

    pub async fn find_active_by_id(&self, session_id: Uuid) -> Result<Option<SessionRow>, sqlx::Error> {
        let maybe_row = sqlx::query(
            r#"
            select id, user_id, refresh_token_hash, created_at, expires_at, last_used_at, revoked_at
            from user_sessions
            where id = $1
            "#,
        )
        .bind(session_id)
        .fetch_optional(&self.pool)
        .await?;

        let Some(row) = maybe_row else {
            return Ok(None);
        };

        Ok(Some(SessionRow {
            id: row.try_get("id")?,
            user_id: row.try_get("user_id")?,
            refresh_token_hash: row.try_get("refresh_token_hash")?,
            created_at: row.try_get("created_at")?,
            expires_at: row.try_get("expires_at")?,
            last_used_at: row.try_get("last_used_at")?,
            revoked_at: row.try_get("revoked_at")?,
        }))
    }

    pub async fn mark_used(&self, session_id: Uuid, at: OffsetDateTime) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            update user_sessions
            set last_used_at = $2
            where id = $1
            "#,
        )
        .bind(session_id)
        .bind(at)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn revoke(&self, session_id: Uuid, at: OffsetDateTime) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            update user_sessions
            set revoked_at = $2
            where id = $1 and revoked_at is null
            "#,
        )
        .bind(session_id)
        .bind(at)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn rotate_refresh_token(
        &self,
        session_id: Uuid,
        new_refresh_token_hash: &str,
        at: OffsetDateTime,
        new_expires_at: OffsetDateTime,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            update user_sessions
            set refresh_token_hash = $2,
                last_used_at = $3,
                expires_at = $4
            where id = $1 and revoked_at is null
            "#,
        )
        .bind(session_id)
        .bind(new_refresh_token_hash)
        .bind(at)
        .bind(new_expires_at)
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}

