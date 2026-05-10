use crate::{util, AppState, AuthContext};
use api::ApiErrorCode;
use axum::{
    extract::{Json, Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::get,
    Extension, Router,
};
use serde::{Deserialize, Serialize};
use sqlx::Row;
use time::OffsetDateTime;
use tracing::Span;
use uuid::Uuid;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/public/branding", get(public_branding))
        .route(
            "/orgs/{org_id}/branding",
            get(get_org_branding).patch(patch_org_branding),
        )
}

#[derive(Debug, Deserialize)]
struct PublicBrandingQuery {
    host: String,
}

#[derive(Debug, Serialize)]
struct BrandingResponse {
    organization_id: Uuid,
    app_name: String,
    theme: String,
    ui_mode: String,
    ui_theme: String,
    logo_url: Option<String>,
    icon_url: Option<String>,
    primary_color: Option<String>,
    secondary_color: Option<String>,
    bg_color: Option<String>,
    surface_color: Option<String>,
    text_color: Option<String>,
    muted_color: Option<String>,
    border_color: Option<String>,
    selection_bg: Option<String>,
    selection_text: Option<String>,
    dropdown_bg: Option<String>,
    dropdown_text: Option<String>,
    chat_bubble_me_bg: Option<String>,
    chat_bubble_me_text: Option<String>,
    chat_bubble_other_bg: Option<String>,
    chat_bubble_other_text: Option<String>,
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
          b.theme,
          b.ui_mode,
          b.ui_theme,
          b.logo_url,
          b.icon_url,
          b.primary_color,
          b.secondary_color,
          b.bg_color,
          b.surface_color,
          b.text_color,
          b.muted_color,
          b.border_color,
          b.selection_bg,
          b.selection_text,
          b.dropdown_bg,
          b.dropdown_text,
          b.chat_bubble_me_bg,
          b.chat_bubble_me_text,
          b.chat_bubble_other_bg,
          b.chat_bubble_other_text,
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
            theme: row.get("theme"),
            ui_mode: row.get("ui_mode"),
            ui_theme: row.get("ui_theme"),
            logo_url: row.try_get("logo_url").ok(),
            icon_url: row.try_get("icon_url").ok(),
            primary_color: row.try_get("primary_color").ok(),
            secondary_color: row.try_get("secondary_color").ok(),
            bg_color: row.try_get("bg_color").ok(),
            surface_color: row.try_get("surface_color").ok(),
            text_color: row.try_get("text_color").ok(),
            muted_color: row.try_get("muted_color").ok(),
            border_color: row.try_get("border_color").ok(),
            selection_bg: row.try_get("selection_bg").ok(),
            selection_text: row.try_get("selection_text").ok(),
            dropdown_bg: row.try_get("dropdown_bg").ok(),
            dropdown_text: row.try_get("dropdown_text").ok(),
            chat_bubble_me_bg: row.try_get("chat_bubble_me_bg").ok(),
            chat_bubble_me_text: row.try_get("chat_bubble_me_text").ok(),
            chat_bubble_other_bg: row.try_get("chat_bubble_other_bg").ok(),
            chat_bubble_other_text: row.try_get("chat_bubble_other_text").ok(),
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
    Span::current().record("organization_id", tracing::field::display(org_id));
    // Must be org member to read.
    let _perms = match util::member_perms(&state.pool, org_id, auth.user_id).await {
        Ok(p) => p,
        Err(e) => return e,
    };

    let row = sqlx::query(
        r#"
        select organization_id, app_name, theme, ui_mode, ui_theme, logo_url, icon_url, primary_color, secondary_color, bg_color, surface_color, text_color, muted_color, border_color,
               selection_bg, selection_text, dropdown_bg, dropdown_text, chat_bubble_me_bg, chat_bubble_me_text, chat_bubble_other_bg, chat_bubble_other_text,
               privacy_url, terms_url, updated_at
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
            theme: row.get("theme"),
            ui_mode: row.get("ui_mode"),
            ui_theme: row.get("ui_theme"),
            logo_url: row.try_get("logo_url").ok(),
            icon_url: row.try_get("icon_url").ok(),
            primary_color: row.try_get("primary_color").ok(),
            secondary_color: row.try_get("secondary_color").ok(),
            bg_color: row.try_get("bg_color").ok(),
            surface_color: row.try_get("surface_color").ok(),
            text_color: row.try_get("text_color").ok(),
            muted_color: row.try_get("muted_color").ok(),
            border_color: row.try_get("border_color").ok(),
            selection_bg: row.try_get("selection_bg").ok(),
            selection_text: row.try_get("selection_text").ok(),
            dropdown_bg: row.try_get("dropdown_bg").ok(),
            dropdown_text: row.try_get("dropdown_text").ok(),
            chat_bubble_me_bg: row.try_get("chat_bubble_me_bg").ok(),
            chat_bubble_me_text: row.try_get("chat_bubble_me_text").ok(),
            chat_bubble_other_bg: row.try_get("chat_bubble_other_bg").ok(),
            chat_bubble_other_text: row.try_get("chat_bubble_other_text").ok(),
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
    theme: Option<String>,
    logo_url: Option<String>,
    primary_color: Option<String>,
    secondary_color: Option<String>,
    ui_mode: Option<String>,
    ui_theme: Option<String>,
    bg_color: Option<String>,
    surface_color: Option<String>,
    text_color: Option<String>,
    muted_color: Option<String>,
    border_color: Option<String>,
    selection_bg: Option<String>,
    selection_text: Option<String>,
    dropdown_bg: Option<String>,
    dropdown_text: Option<String>,
    chat_bubble_me_bg: Option<String>,
    chat_bubble_me_text: Option<String>,
    chat_bubble_other_bg: Option<String>,
    chat_bubble_other_text: Option<String>,
    privacy_url: Option<String>,
    terms_url: Option<String>,
}

async fn patch_org_branding(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Path(org_id): Path<Uuid>,
    Json(req): Json<PatchBrandingRequest>,
) -> impl IntoResponse {
    Span::current().record("organization_id", tracing::field::display(org_id));
    let ok = match util::can(
        &state.pool,
        auth.user_id,
        org_id,
        permissions::Permission::BrandingManage,
    )
    .await
    {
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
    let theme = req
        .theme
        .map(|s| s.trim().to_lowercase())
        .filter(|s| !s.is_empty());
    if theme
        .as_deref()
        .is_some_and(|t| t != "dark" && t != "light")
    {
        return util::api_error(ApiErrorCode::ValidationError);
    }
    let logo_url = clean_opt(req.logo_url);
    let primary_color = clean_opt(req.primary_color);
    let secondary_color = clean_opt(req.secondary_color);
    let ui_mode = req
        .ui_mode
        .map(|s| s.trim().to_lowercase())
        .filter(|s| !s.is_empty());
    if ui_mode.as_deref().is_some_and(|m| m != "work" && m != "play") {
        return util::api_error(ApiErrorCode::ValidationError);
    }
    let ui_theme = req
        .ui_theme
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());
    let bg_color = clean_opt(req.bg_color);
    let surface_color = clean_opt(req.surface_color);
    let text_color = clean_opt(req.text_color);
    let muted_color = clean_opt(req.muted_color);
    let border_color = clean_opt(req.border_color);
    let selection_bg = clean_opt(req.selection_bg);
    let selection_text = clean_opt(req.selection_text);
    let dropdown_bg = clean_opt(req.dropdown_bg);
    let dropdown_text = clean_opt(req.dropdown_text);
    let chat_bubble_me_bg = clean_opt(req.chat_bubble_me_bg);
    let chat_bubble_me_text = clean_opt(req.chat_bubble_me_text);
    let chat_bubble_other_bg = clean_opt(req.chat_bubble_other_bg);
    let chat_bubble_other_text = clean_opt(req.chat_bubble_other_text);
    let privacy_url = clean_opt(req.privacy_url);
    let terms_url = clean_opt(req.terms_url);

    let updated = sqlx::query(
        r#"
        update branding_profiles
        set
          app_name = coalesce($2, app_name),
          theme = coalesce($3, theme),
          ui_mode = coalesce($4, ui_mode),
          ui_theme = coalesce($5, ui_theme),
          logo_url = $6,
          primary_color = $7,
          secondary_color = $8,
          bg_color = $9,
          surface_color = $10,
          text_color = $11,
          muted_color = $12,
          border_color = $13,
          selection_bg = $14,
          selection_text = $15,
          dropdown_bg = $16,
          dropdown_text = $17,
          chat_bubble_me_bg = $18,
          chat_bubble_me_text = $19,
          chat_bubble_other_bg = $20,
          chat_bubble_other_text = $21,
          privacy_url = $22,
          terms_url = $23,
          updated_at = now()
        where organization_id = $1
        "#,
    )
    .bind(org_id)
    .bind(app_name)
    .bind(theme)
    .bind(ui_mode)
    .bind(ui_theme)
    .bind(logo_url)
    .bind(primary_color)
    .bind(secondary_color)
    .bind(bg_color)
    .bind(surface_color)
    .bind(text_color)
    .bind(muted_color)
    .bind(border_color)
    .bind(selection_bg)
    .bind(selection_text)
    .bind(dropdown_bg)
    .bind(dropdown_text)
    .bind(chat_bubble_me_bg)
    .bind(chat_bubble_me_text)
    .bind(chat_bubble_other_bg)
    .bind(chat_bubble_other_text)
    .bind(privacy_url)
    .bind(terms_url)
    .execute(&state.pool)
    .await;

    match updated {
        Ok(r) if r.rows_affected() == 0 => util::api_error(ApiErrorCode::NotFound),
        Ok(_) => {
            util::write_audit_log(
                &state.pool,
                org_id,
                Some(auth.user_id),
                "branding.updated",
                Some("branding_profile"),
                Some(org_id),
                serde_json::json!({}),
            )
            .await;
            (StatusCode::OK, Json(serde_json::json!({"status":"ok"}))).into_response()
        }
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
