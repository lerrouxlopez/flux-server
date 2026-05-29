use crate::{util, AppState, AuthContext};
use api::ApiErrorCode;
use axum::{
    extract::{Path, State},
    http::{header, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    routing::get,
    Router,
};
use sqlx::Row;
use uuid::Uuid;

pub fn router() -> Router<AppState> {
    Router::new().route(
        "/attachments/{attachment_id}/download",
        get(download_attachment),
    )
}

pub async fn download_attachment(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(attachment_id): Path<Uuid>,
) -> impl IntoResponse {
    // Look up attachment + owning channel/org via message join.
    let row = sqlx::query(
        r#"
        select a.id,
               a.organization_id,
               a.message_id,
               a.filename,
               a.content_type,
               a.size_bytes,
               a.storage_kind,
               a.storage_path,
               m.channel_id
        from message_attachments a
        join messages m on m.id = a.message_id
        where a.id = $1
        "#,
    )
    .bind(attachment_id)
    .fetch_optional(&state.pool)
    .await;

    let row = match row {
        Ok(v) => v,
        Err(_) => return util::api_error(ApiErrorCode::InternalError),
    };
    let Some(row) = row else {
        return util::api_error(ApiErrorCode::NotFound);
    };

    let channel_id: Uuid = row.get("channel_id");
    let org_id: Uuid = row.get("organization_id");
    tracing::Span::current().record("organization_id", tracing::field::display(org_id));

    let can_access = match util::can_access_channel(&state.pool, auth.user_id, channel_id).await {
        Ok(v) => v,
        Err(e) => return e,
    };
    if !can_access {
        return util::api_error(ApiErrorCode::PermissionDenied);
    }

    let filename: String = row.get("filename");
    let content_type: Option<String> = row.try_get("content_type").ok();
    let storage_kind: String = row.get("storage_kind");
    let storage_path: String = row.get("storage_path");

    let (bytes, mime) = if storage_kind == "data_url" || storage_path.starts_with("data:") {
        let (mime, bytes) = match crate::attachments_storage::parse_data_url(&storage_path) {
            Ok(v) => v,
            Err(e) => return e,
        };
        (bytes, Some(mime))
    } else {
        let storage = crate::attachments_storage::storage_from_env();
        let path = storage.get_path(&storage_path);
        let bytes = match std::fs::read(&path) {
            Ok(b) => b,
            Err(_) => return util::api_error(ApiErrorCode::NotFound),
        };
        (bytes, content_type.clone())
    };

    let mut res = Response::new(axum::body::Body::from(bytes));
    *res.status_mut() = StatusCode::OK;
    res.headers_mut().insert(
        header::CONTENT_DISPOSITION,
        HeaderValue::from_str(&format!("attachment; filename=\"{}\"", sanitize_filename(&filename)))
            .unwrap_or(HeaderValue::from_static("attachment")),
    );
    if let Some(m) = mime.or(content_type) {
        if let Ok(v) = HeaderValue::from_str(&m) {
            res.headers_mut().insert(header::CONTENT_TYPE, v);
        }
    }
    res
}

fn sanitize_filename(input: &str) -> String {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return "download".to_string();
    }
    trimmed
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_' | ' ') {
                c
            } else {
                '_'
            }
        })
        .take(120)
        .collect::<String>()
}
