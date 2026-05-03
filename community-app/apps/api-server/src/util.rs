use api::{ApiError, ApiErrorCode};
use permissions::{perms, Permission, Perms};
use sqlx::{PgPool, Row};
use uuid::Uuid;

pub fn api_error(code: ApiErrorCode) -> axum::response::Response {
    ApiError::new(code).into_response()
}

pub fn api_error_msg(code: ApiErrorCode, message: impl Into<String>) -> axum::response::Response {
    ApiError::with_message(code, message).into_response()
}

pub fn is_unique_violation(err: &sqlx::Error) -> bool {
    match err {
        sqlx::Error::Database(db_err) => db_err.code().as_deref() == Some("23505"),
        _ => false,
    }
}

pub async fn is_member(
    pool: &PgPool,
    org_id: Uuid,
    user_id: Uuid,
) -> Result<bool, axum::response::Response> {
    let ok = sqlx::query_scalar::<_, i64>(
        r#"
        select 1
        from organization_members
        where organization_id = $1 and user_id = $2
        "#,
    )
    .bind(org_id)
    .bind(user_id)
    .fetch_optional(pool)
    .await
    .map_err(|_| api_error(ApiErrorCode::InternalError))?
    .is_some();

    Ok(ok)
}

pub async fn member_perms(
    pool: &PgPool,
    org_id: Uuid,
    user_id: Uuid,
) -> Result<Perms, axum::response::Response> {
    let row = sqlx::query(
        r#"
        select m.role, r.permissions
        from organization_members m
        left join roles r
          on r.organization_id = m.organization_id
         and r.name = m.role
        where m.organization_id = $1 and m.user_id = $2
        "#,
    )
    .bind(org_id)
    .bind(user_id)
    .fetch_optional(pool)
    .await
    .map_err(|_| api_error(ApiErrorCode::InternalError))?;

    let Some(row) = row else {
        return Err(api_error(ApiErrorCode::PermissionDenied));
    };

    let role: String = row.get("role");
    if role == "owner" {
        return Ok(perms::ALL);
    }

    let permissions: Option<i64> = row.try_get("permissions").ok();
    Ok(permissions.unwrap_or(0))
}

pub async fn can(
    pool: &PgPool,
    user_id: Uuid,
    organization_id: Uuid,
    permission: Permission,
) -> Result<bool, axum::response::Response> {
    let perms = member_perms(pool, organization_id, user_id).await?;
    Ok(permissions::has(perms, permission.bit()))
}

pub async fn can_access_channel(
    pool: &PgPool,
    user_id: Uuid,
    channel_id: Uuid,
) -> Result<bool, axum::response::Response> {
    let row = sqlx::query(
        r#"
        select organization_id
        from channels
        where id = $1
        "#,
    )
    .bind(channel_id)
    .fetch_optional(pool)
    .await
    .map_err(|_| api_error(ApiErrorCode::InternalError))?;

    let Some(row) = row else {
        return Ok(false);
    };

    let org_id: Uuid = row.get("organization_id");
    let perms = member_perms(pool, org_id, user_id).await?;
    Ok(permissions::has(perms, perms::CHANNELS_VIEW))
}

pub async fn write_audit_log(
    pool: &PgPool,
    organization_id: Uuid,
    actor_user_id: Option<Uuid>,
    action: &'static str,
    target_type: Option<&'static str>,
    target_id: Option<Uuid>,
    metadata: serde_json::Value,
) {
    let id = Uuid::now_v7();
    let _ = sqlx::query(
        r#"
        insert into audit_logs (id, organization_id, actor_user_id, action, target_type, target_id, metadata, created_at)
        values ($1, $2, $3, $4, $5, $6, $7, now())
        "#,
    )
    .bind(id)
    .bind(organization_id)
    .bind(actor_user_id)
    .bind(action)
    .bind(target_type)
    .bind(target_id)
    .bind(metadata)
    .execute(pool)
    .await;
}
