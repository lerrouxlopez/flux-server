use crate::{util, AppState, AuthContext};
use axum::{
    extract::{Query, Path, State, Json},
    http::StatusCode,
    response::IntoResponse,
    routing::get,
    Extension, Router,
};
use domain::api_error::ApiErrorCode;
use serde::{Deserialize, Serialize};
use sqlx::Row;
use time::OffsetDateTime;
use uuid::Uuid;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/public/branding", get(public_branding))
        .route("/orgs/{org_id}/branding", get(get_org_branding).patch(patch_org_branding))
}

#[derive(Debug, Deserialize)]
struct PublicBrandingQuery {
    host: String,
}

#[derive(Debug, Serialize)]
struct BrandingResponse {
    organization_id: Uuid,
    app_name: String,
    logo_url: Option<String>,
    icon_url: Option<String>,
    primary_color: Option<String>,
    secondary_color: Option<String>,
    privacy_url: Option<String>,
    terms_url: Option<String>,
    updated_at: OffsetDateTime,
}

async fn public_branding(
    State(state): State<AppState>,
    Query(q): Query<PublicBrandingQuery>,
) -> impl IntoResponse {
    let host = normalize_host(&q.host);
    if host.is_empty() {
        return util::api_error_msg(ApiErrorCode::ValidationError, "host is required");
    }

    // Resolve by:
    // 1) branding_profiles.custom_domain (exact match)
    // 2) organizations.slug (if host's first label matches)
    let maybe_slug = host
        .split('.')
        .next()
        .map(|s| s.to_string())
        .filter(|s| !s.is_empty());

    let row = sqlx::query(
        r#"
        select
          b.organization_id,
          b.app_name,
          b.logo_url,
          b.icon_url,
          b.primary_color,
          b.secondary_color,
          b.privacy_url,
          b.terms_url,
          b.updated_at
        from branding_profiles b
        join organizations o on o.id = b.organization_id
        where b.custom_domain = $1
           or ($2 is not null and o.slug = $2)
        order by (b.custom_domain = $1) desc
        limit 1
        "#,
    )
    .bind(&host)
    .bind(maybe_slug.as_deref())
    .fetch_optional(&state.pool)
    .await;

    let Some(row) = (match row {
        Ok(r) => r,
        Err(_) => return util::api_error(ApiErrorCode::InternalError),
    }) else {
        // Public endpoint: 404 is fine and leaks nothing.
        return util::api_error(ApiErrorCode::NotFound);
    };

    (
        StatusCode::OK,
        Json(BrandingResponse {
            organization_id: row.get("organization_id"),
            app_name: row.get("app_name"),
            logo_url: row.try_get("logo_url").ok(),
            icon_url: row.try_get("icon_url").ok(),
            primary_color: row.try_get("primary_color").ok(),
            secondary_color: row.try_get("secondary_color").ok(),
            privacy_url: row.try_get("privacy_url").ok(),
            terms_url: row.try_get("terms_url").ok(),
            updated_at: row.get("updated_at"),
        }),
    )
        .into_response()
}

async fn get_org_branding(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Path(org_id): Path<Uuid>,
) -> impl IntoResponse {
    // Must be org member to read.
    let _perms = match util::member_perms(&state.pool, org_id, auth.user_id).await {
        Ok(p) => p,
        Err(e) => return e,
    };

    let row = sqlx::query(
        r#"
        select organization_id, app_name, logo_url, icon_url, primary_color, secondary_color, privacy_url, terms_url, updated_at
        from branding_profiles
        where organization_id = $1
        "#,
    )
    .bind(org_id)
    .fetch_optional(&state.pool)
    .await;

    let Some(row) = (match row {
        Ok(r) => r,
        Err(_) => return util::api_error(ApiErrorCode::InternalError),
    }) else {
        return util::api_error(ApiErrorCode::NotFound);
    };

    (
        StatusCode::OK,
        Json(BrandingResponse {
            organization_id: row.get("organization_id"),
            app_name: row.get("app_name"),
            logo_url: row.try_get("logo_url").ok(),
            icon_url: row.try_get("icon_url").ok(),
            primary_color: row.try_get("primary_color").ok(),
            secondary_color: row.try_get("secondary_color").ok(),
            privacy_url: row.try_get("privacy_url").ok(),
            terms_url: row.try_get("terms_url").ok(),
            updated_at: row.get("updated_at"),
        }),
    )
        .into_response()
}

#[derive(Debug, Deserialize)]
struct PatchBrandingRequest {
    app_name: Option<String>,
    logo_url: Option<String>,
    primary_color: Option<String>,
    secondary_color: Option<String>,
    privacy_url: Option<String>,
    terms_url: Option<String>,
}

async fn patch_org_branding(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Path(org_id): Path<Uuid>,
    Json(req): Json<PatchBrandingRequest>,
) -> impl IntoResponse {
    let ok = match util::can(&state.pool, auth.user_id, org_id, permissions::Permission::BrandingManage).await {
        Ok(v) => v,
        Err(e) => return e,
    };
    if !ok {
        return util::api_error(ApiErrorCode::PermissionDenied);
    }

    let app_name = req
        .app_name
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());
    let logo_url = clean_opt(req.logo_url);
    let primary_color = clean_opt(req.primary_color);
    let secondary_color = clean_opt(req.secondary_color);
    let privacy_url = clean_opt(req.privacy_url);
    let terms_url = clean_opt(req.terms_url);

    let updated = sqlx::query(
        r#"
        update branding_profiles
        set
          app_name = coalesce($2, app_name),
          logo_url = $3,
          primary_color = $4,
          secondary_color = $5,
          privacy_url = $6,
          terms_url = $7,
          updated_at = now()
        where organization_id = $1
        "#,
    )
    .bind(org_id)
    .bind(app_name)
    .bind(logo_url)
    .bind(primary_color)
    .bind(secondary_color)
    .bind(privacy_url)
    .bind(terms_url)
    .execute(&state.pool)
    .await;

    match updated {
        Ok(r) if r.rows_affected() == 0 => util::api_error(ApiErrorCode::NotFound),
        Ok(_) => (StatusCode::OK, Json(serde_json::json!({"status":"ok"}))).into_response(),
        Err(_) => util::api_error(ApiErrorCode::InternalError),
    }
}

fn clean_opt(v: Option<String>) -> Option<String> {
    v.map(|s| s.trim().to_string())
        .and_then(|s| if s.is_empty() { None } else { Some(s) })
}

fn normalize_host(host: &str) -> String {
    let mut h = host.trim().to_lowercase();
    if let Some(stripped) = h.strip_prefix("http://") {
        h = stripped.to_string();
    }
    if let Some(stripped) = h.strip_prefix("https://") {
        h = stripped.to_string();
    }
    if let Some((base, _port)) = h.split_once(':') {
        h = base.to_string();
    }
    h.trim_matches('/').to_string()
}
