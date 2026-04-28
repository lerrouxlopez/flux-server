use axum::{http::StatusCode, response::IntoResponse, Json};
use permissions::{perms, Perms};
use sqlx::{PgPool, Row};
use uuid::Uuid;

pub fn api_error(status: StatusCode, code: &'static str) -> axum::response::Response {
    (status, Json(serde_json::json!({ "error": code }))).into_response()
}

pub fn is_unique_violation(err: &sqlx::Error) -> bool {
    match err {
        sqlx::Error::Database(db_err) => db_err.code().as_deref() == Some("23505"),
        _ => false,
    }
}

pub async fn is_member(pool: &PgPool, org_id: Uuid, user_id: Uuid) -> Result<bool, axum::response::Response> {
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
    .map_err(|_| api_error(StatusCode::INTERNAL_SERVER_ERROR, "db_error"))?
    .is_some();

    Ok(ok)
}

pub async fn member_perms(pool: &PgPool, org_id: Uuid, user_id: Uuid) -> Result<Perms, axum::response::Response> {
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
    .map_err(|_| api_error(StatusCode::INTERNAL_SERVER_ERROR, "db_error"))?;

    let Some(row) = row else {
        return Err(api_error(StatusCode::FORBIDDEN, "not_a_member"));
    };

    let role: String = row.get("role");
    if role == "owner" {
        return Ok(perms::ALL);
    }

    let permissions: Option<i64> = row.try_get("permissions").ok();
    Ok(permissions.unwrap_or(0))
}

