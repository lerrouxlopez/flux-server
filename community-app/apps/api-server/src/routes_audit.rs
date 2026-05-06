use crate::{util, AppState, AuthContext};
use api::ApiErrorCode;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::get,
    Extension, Json, Router,
};
use serde::{Deserialize, Serialize};
use sqlx::Row;
use tracing::Span;
use uuid::Uuid;

pub fn router() -> Router<AppState> {
    Router::new().route("/orgs/{org_id}/audit-logs", get(list_audit_logs))
}

#[derive(Debug, Deserialize)]
struct ListAuditQuery {
    limit: Option<i64>,
}

#[derive(Debug, Serialize)]
struct Actor {
    id: Uuid,
    email: String,
    display_name: String,
}

#[derive(Debug, Serialize)]
struct AuditEntry {
    id: Uuid,
    actor: Option<Actor>,
    action: String,
    target_type: Option<String>,
    target_id: Option<Uuid>,
    metadata: serde_json::Value,
    created_at: time::OffsetDateTime,
}

#[derive(Debug, Serialize)]
struct AuditLogsResponse {
    entries: Vec<AuditEntry>,
}

async fn list_audit_logs(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Path(org_id): Path<Uuid>,
    Query(q): Query<ListAuditQuery>,
) -> impl IntoResponse {
    Span::current().record("organization_id", tracing::field::display(org_id));

    let ok = match util::can(
        &state.pool,
        auth.user_id,
        org_id,
        permissions::Permission::AdminAuditLogView,
    )
    .await
    {
        Ok(v) => v,
        Err(e) => return e,
    };
    if !ok {
        return util::api_error(ApiErrorCode::PermissionDenied);
    }

    let limit = q.limit.unwrap_or(100).clamp(1, 200);

    let rows = sqlx::query(
        r#"
        select
          a.id,
          a.action,
          a.target_type,
          a.target_id,
          a.metadata,
          a.created_at,
          u.id as actor_id,
          u.email as actor_email,
          u.display_name as actor_display_name
        from audit_logs a
        left join users u on u.id = a.actor_user_id
        where a.organization_id = $1
        order by a.created_at desc
        limit $2
        "#,
    )
    .bind(org_id)
    .bind(limit)
    .fetch_all(&state.pool)
    .await;

    let rows = match rows {
        Ok(r) => r,
        Err(_) => return util::api_error(ApiErrorCode::InternalError),
    };

    let mut entries = Vec::with_capacity(rows.len());
    for row in rows {
        let actor_id: Option<Uuid> = row.try_get("actor_id").ok();
        let actor = actor_id.map(|id| Actor {
            id,
            email: row.try_get::<String, _>("actor_email").unwrap_or_default(),
            display_name: row
                .try_get::<String, _>("actor_display_name")
                .unwrap_or_default(),
        });

        entries.push(AuditEntry {
            id: row.get("id"),
            actor,
            action: row.get("action"),
            target_type: row.try_get("target_type").ok(),
            target_id: row.try_get("target_id").ok(),
            metadata: row
                .try_get::<serde_json::Value, _>("metadata")
                .unwrap_or(serde_json::json!({})),
            created_at: row.get("created_at"),
        });
    }

    (StatusCode::OK, Json(AuditLogsResponse { entries })).into_response()
}
