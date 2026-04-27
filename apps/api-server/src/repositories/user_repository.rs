use sqlx::{PgPool, Row};
use time::OffsetDateTime;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct UserRow {
    pub id: Uuid,
    pub email: String,
    pub display_name: String,
    pub password_hash: Option<String>,
    pub created_at: OffsetDateTime,
}

#[derive(Clone)]
pub struct UserRepository {
    pool: PgPool,
}

impl UserRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn insert_user(
        &self,
        id: Uuid,
        email: &str,
        display_name: &str,
        password_hash: &str,
    ) -> Result<UserRow, sqlx::Error> {
        let row = sqlx::query(
            r#"
            insert into users (id, email, display_name, password_hash)
            values ($1, $2, $3, $4)
            returning id, email, display_name, password_hash, created_at
            "#,
        )
        .bind(id)
        .bind(email)
        .bind(display_name)
        .bind(password_hash)
        .fetch_one(&self.pool)
        .await?;

        Ok(UserRow {
            id: row.try_get("id")?,
            email: row.try_get("email")?,
            display_name: row.try_get("display_name")?,
            password_hash: row.try_get("password_hash")?,
            created_at: row.try_get("created_at")?,
        })
    }

    pub async fn find_by_email(&self, email: &str) -> Result<Option<UserRow>, sqlx::Error> {
        let maybe_row = sqlx::query(
            r#"
            select id, email, display_name, password_hash, created_at
            from users
            where email = $1
            "#,
        )
        .bind(email)
        .fetch_optional(&self.pool)
        .await?;

        let Some(row) = maybe_row else {
            return Ok(None);
        };

        Ok(Some(UserRow {
            id: row.try_get("id")?,
            email: row.try_get("email")?,
            display_name: row.try_get("display_name")?,
            password_hash: row.try_get("password_hash")?,
            created_at: row.try_get("created_at")?,
        }))
    }

    pub async fn find_by_id(&self, user_id: Uuid) -> Result<Option<UserRow>, sqlx::Error> {
        let maybe_row = sqlx::query(
            r#"
            select id, email, display_name, password_hash, created_at
            from users
            where id = $1
            "#,
        )
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await?;

        let Some(row) = maybe_row else {
            return Ok(None);
        };

        Ok(Some(UserRow {
            id: row.try_get("id")?,
            email: row.try_get("email")?,
            display_name: row.try_get("display_name")?,
            password_hash: row.try_get("password_hash")?,
            created_at: row.try_get("created_at")?,
        }))
    }
}

