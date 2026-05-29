use crate::{util, AppState, AuthContext};
use api::ApiErrorCode;
use axum::{
    extract::{Json, Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
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
        .route("/orgs/{org_id}/branding/preview", post(preview_org_branding))
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

// Patch semantics:
// - missing field => keep existing value
// - explicit null => clear (for nullable fields)
// - provided value => update
//
// We parse raw JSON to distinguish missing vs null.

async fn patch_org_branding(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Path(org_id): Path<Uuid>,
    Json(req): Json<serde_json::Value>,
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

    let patch = match BrandingPatch::from_json(req) {
        Ok(p) => p,
        Err(e) => return e,
    };

    // Validate tokens/urls/contrast using a simulated merged profile.
    let current = match fetch_branding_row(&state.pool, org_id).await {
        Ok(v) => v,
        Err(e) => return e,
    };
    let merged = match apply_patch_to_row(&state.pool, &current, &patch).await {
        Ok(v) => v,
        Err(e) => return e,
    };
    if let Err(e) = validate_branding_row(&merged) {
        return e;
    }

    let updated = sqlx::query(
        r#"
        update branding_profiles
        set
          app_name = case when $2 then $3 else app_name end,
          theme = case when $4 then $5 else theme end,
          ui_mode = case when $6 then $7 else ui_mode end,
          ui_theme = case when $8 then $9 else ui_theme end,
          preset_id = case when $10 then $11 else preset_id end,
          tokens = case when $12 then $13 else tokens end,

          logo_url = case when $14 then $15 else logo_url end,
          icon_url = case when $16 then $17 else icon_url end,
          primary_color = case when $18 then $19 else primary_color end,
          secondary_color = case when $20 then $21 else secondary_color end,
          bg_color = case when $22 then $23 else bg_color end,
          surface_color = case when $24 then $25 else surface_color end,
          text_color = case when $26 then $27 else text_color end,
          muted_color = case when $28 then $29 else muted_color end,
          border_color = case when $30 then $31 else border_color end,
          selection_bg = case when $32 then $33 else selection_bg end,
          selection_text = case when $34 then $35 else selection_text end,
          dropdown_bg = case when $36 then $37 else dropdown_bg end,
          dropdown_text = case when $38 then $39 else dropdown_text end,
          chat_bubble_me_bg = case when $40 then $41 else chat_bubble_me_bg end,
          chat_bubble_me_text = case when $42 then $43 else chat_bubble_me_text end,
          chat_bubble_other_bg = case when $44 then $45 else chat_bubble_other_bg end,
          chat_bubble_other_text = case when $46 then $47 else chat_bubble_other_text end,
          privacy_url = case when $48 then $49 else privacy_url end,
          terms_url = case when $50 then $51 else terms_url end,
          updated_at = now()
        where organization_id = $1
        "#,
    )
    .bind(org_id)
    .bind(patch.app_name.present)
    .bind(patch.app_name.value)
    .bind(patch.theme.present)
    .bind(patch.theme.value)
    .bind(patch.ui_mode.present)
    .bind(patch.ui_mode.value)
    .bind(patch.ui_theme.present)
    .bind(patch.ui_theme.value)
    .bind(patch.preset_id.present)
    .bind(patch.preset_id.value)
    .bind(patch.tokens.present)
    .bind(patch.tokens.value)

    .bind(patch.logo_url.present)
    .bind(patch.logo_url.value)
    .bind(patch.icon_url.present)
    .bind(patch.icon_url.value)
    .bind(patch.primary_color.present)
    .bind(patch.primary_color.value)
    .bind(patch.secondary_color.present)
    .bind(patch.secondary_color.value)
    .bind(patch.bg_color.present)
    .bind(patch.bg_color.value)
    .bind(patch.surface_color.present)
    .bind(patch.surface_color.value)
    .bind(patch.text_color.present)
    .bind(patch.text_color.value)
    .bind(patch.muted_color.present)
    .bind(patch.muted_color.value)
    .bind(patch.border_color.present)
    .bind(patch.border_color.value)
    .bind(patch.selection_bg.present)
    .bind(patch.selection_bg.value)
    .bind(patch.selection_text.present)
    .bind(patch.selection_text.value)
    .bind(patch.dropdown_bg.present)
    .bind(patch.dropdown_bg.value)
    .bind(patch.dropdown_text.present)
    .bind(patch.dropdown_text.value)
    .bind(patch.chat_bubble_me_bg.present)
    .bind(patch.chat_bubble_me_bg.value)
    .bind(patch.chat_bubble_me_text.present)
    .bind(patch.chat_bubble_me_text.value)
    .bind(patch.chat_bubble_other_bg.present)
    .bind(patch.chat_bubble_other_bg.value)
    .bind(patch.chat_bubble_other_text.present)
    .bind(patch.chat_bubble_other_text.value)
    .bind(patch.privacy_url.present)
    .bind(patch.privacy_url.value)
    .bind(patch.terms_url.present)
    .bind(patch.terms_url.value)
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

async fn preview_org_branding(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Path(org_id): Path<Uuid>,
    Json(req): Json<serde_json::Value>,
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

    let patch = match BrandingPatch::from_json(req) {
        Ok(p) => p,
        Err(e) => return e,
    };
    let current = match fetch_branding_row(&state.pool, org_id).await {
        Ok(v) => v,
        Err(e) => return e,
    };
    let merged = match apply_patch_to_row(&state.pool, &current, &patch).await {
        Ok(v) => v,
        Err(e) => return e,
    };
    if let Err(e) = validate_branding_row(&merged) {
        return e;
    }

    (StatusCode::OK, Json(to_response(&merged))).into_response()
}

#[derive(Debug, Clone)]
struct FieldPatch<T> {
    present: bool,
    value: Option<T>,
}

#[derive(Debug, Clone)]
struct BrandingPatch {
    app_name: FieldPatch<String>,      // required (no clear)
    theme: FieldPatch<String>,         // required (no clear)
    ui_mode: FieldPatch<String>,       // required (no clear)
    ui_theme: FieldPatch<String>,      // required (no clear)
    preset_id: FieldPatch<String>,     // nullable
    tokens: FieldPatch<serde_json::Value>, // jsonb

    logo_url: FieldPatch<String>,
    icon_url: FieldPatch<String>,
    primary_color: FieldPatch<String>,
    secondary_color: FieldPatch<String>,
    bg_color: FieldPatch<String>,
    surface_color: FieldPatch<String>,
    text_color: FieldPatch<String>,
    muted_color: FieldPatch<String>,
    border_color: FieldPatch<String>,
    selection_bg: FieldPatch<String>,
    selection_text: FieldPatch<String>,
    dropdown_bg: FieldPatch<String>,
    dropdown_text: FieldPatch<String>,
    chat_bubble_me_bg: FieldPatch<String>,
    chat_bubble_me_text: FieldPatch<String>,
    chat_bubble_other_bg: FieldPatch<String>,
    chat_bubble_other_text: FieldPatch<String>,
    privacy_url: FieldPatch<String>,
    terms_url: FieldPatch<String>,
}

impl BrandingPatch {
    fn from_json(v: serde_json::Value) -> Result<Self, axum::response::Response> {
        let obj = v
            .as_object()
            .ok_or_else(|| util::api_error(ApiErrorCode::ValidationError))?;

        let tokens = json_patch(obj, "tokens")?;
        if tokens.present {
            if let Some(ref t) = tokens.value {
                validate_tokens_object(t)?;
            }
        }

        Ok(Self {
            app_name: required_string_patch(obj, "app_name")?,
            theme: required_enum_patch(obj, "theme", &["dark", "light"])?,
            ui_mode: required_enum_patch(obj, "ui_mode", &["work", "play"])?,
            ui_theme: required_string_patch(obj, "ui_theme")?,
            preset_id: nullable_string_patch(obj, "preset_id")?,
            tokens,

            logo_url: nullable_logo_patch(obj, "logo_url")?,
            icon_url: nullable_logo_patch(obj, "icon_url")?,
            primary_color: nullable_color_patch(obj, "primary_color")?,
            secondary_color: nullable_color_patch(obj, "secondary_color")?,
            bg_color: nullable_color_patch(obj, "bg_color")?,
            surface_color: nullable_color_patch(obj, "surface_color")?,
            text_color: nullable_color_patch(obj, "text_color")?,
            muted_color: nullable_color_patch(obj, "muted_color")?,
            border_color: nullable_color_patch(obj, "border_color")?,
            selection_bg: nullable_color_patch(obj, "selection_bg")?,
            selection_text: nullable_color_patch(obj, "selection_text")?,
            dropdown_bg: nullable_color_patch(obj, "dropdown_bg")?,
            dropdown_text: nullable_color_patch(obj, "dropdown_text")?,
            chat_bubble_me_bg: nullable_color_patch(obj, "chat_bubble_me_bg")?,
            chat_bubble_me_text: nullable_color_patch(obj, "chat_bubble_me_text")?,
            chat_bubble_other_bg: nullable_color_patch(obj, "chat_bubble_other_bg")?,
            chat_bubble_other_text: nullable_color_patch(obj, "chat_bubble_other_text")?,
            privacy_url: nullable_url_patch(obj, "privacy_url")?,
            terms_url: nullable_url_patch(obj, "terms_url")?,
        })
    }
}

#[derive(Debug, Clone)]
struct BrandingRow {
    organization_id: Uuid,
    app_name: String,
    theme: String,
    ui_mode: String,
    ui_theme: String,
    preset_id: Option<String>,
    tokens: serde_json::Value,

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

async fn fetch_branding_row(pool: &sqlx::PgPool, org_id: Uuid) -> Result<BrandingRow, axum::response::Response> {
    let row = sqlx::query(
        r#"
        select organization_id, app_name, theme, ui_mode, ui_theme,
               preset_id, tokens,
               logo_url, icon_url, primary_color, secondary_color,
               bg_color, surface_color, text_color, muted_color, border_color,
               selection_bg, selection_text, dropdown_bg, dropdown_text,
               chat_bubble_me_bg, chat_bubble_me_text, chat_bubble_other_bg, chat_bubble_other_text,
               privacy_url, terms_url, updated_at
        from branding_profiles
        where organization_id = $1
        "#,
    )
    .bind(org_id)
    .fetch_optional(pool)
    .await
    .map_err(|_| util::api_error(ApiErrorCode::InternalError))?;

    let Some(row) = row else {
        return Err(util::api_error(ApiErrorCode::NotFound));
    };

    Ok(BrandingRow {
        organization_id: row.get("organization_id"),
        app_name: row.get("app_name"),
        theme: row.get("theme"),
        ui_mode: row.get("ui_mode"),
        ui_theme: row.get("ui_theme"),
        preset_id: row.try_get("preset_id").ok(),
        tokens: row
            .try_get::<serde_json::Value, _>("tokens")
            .unwrap_or(serde_json::json!({})),
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
    })
}

async fn apply_patch_to_row(
    pool: &sqlx::PgPool,
    current: &BrandingRow,
    patch: &BrandingPatch,
) -> Result<BrandingRow, axum::response::Response> {
    let mut out = current.clone();

    if patch.app_name.present {
        let Some(v) = patch.app_name.value.clone() else {
            return Err(util::api_error(ApiErrorCode::ValidationError));
        };
        if v.trim().is_empty() {
            return Err(util::api_error(ApiErrorCode::ValidationError));
        }
        out.app_name = v.trim().to_string();
    }
    if patch.theme.present {
        let Some(v) = patch.theme.value.clone() else {
            return Err(util::api_error(ApiErrorCode::ValidationError));
        };
        out.theme = v;
    }
    if patch.ui_mode.present {
        let Some(v) = patch.ui_mode.value.clone() else {
            return Err(util::api_error(ApiErrorCode::ValidationError));
        };
        out.ui_mode = v;
    }
    if patch.ui_theme.present {
        let Some(v) = patch.ui_theme.value.clone() else {
            return Err(util::api_error(ApiErrorCode::ValidationError));
        };
        out.ui_theme = v;
    }

    if patch.preset_id.present {
        out.preset_id = patch.preset_id.value.clone();
        if let Some(ref pid) = out.preset_id {
            // Validate preset exists.
            let exists: Option<i64> = sqlx::query_scalar(
                r#"select 1::bigint from brand_presets where id = $1"#,
            )
            .bind(pid)
            .fetch_optional(pool)
            .await
            .map_err(|_| util::api_error(ApiErrorCode::InternalError))?;
            if exists.is_none() {
                return Err(util::api_error(ApiErrorCode::ValidationError));
            }
        }
    }
    if patch.tokens.present {
        out.tokens = patch.tokens.value.clone().unwrap_or(serde_json::json!({}));
    }

    apply_nullable(&mut out.logo_url, &patch.logo_url);
    apply_nullable(&mut out.icon_url, &patch.icon_url);
    apply_nullable(&mut out.primary_color, &patch.primary_color);
    apply_nullable(&mut out.secondary_color, &patch.secondary_color);
    apply_nullable(&mut out.bg_color, &patch.bg_color);
    apply_nullable(&mut out.surface_color, &patch.surface_color);
    apply_nullable(&mut out.text_color, &patch.text_color);
    apply_nullable(&mut out.muted_color, &patch.muted_color);
    apply_nullable(&mut out.border_color, &patch.border_color);
    apply_nullable(&mut out.selection_bg, &patch.selection_bg);
    apply_nullable(&mut out.selection_text, &patch.selection_text);
    apply_nullable(&mut out.dropdown_bg, &patch.dropdown_bg);
    apply_nullable(&mut out.dropdown_text, &patch.dropdown_text);
    apply_nullable(&mut out.chat_bubble_me_bg, &patch.chat_bubble_me_bg);
    apply_nullable(&mut out.chat_bubble_me_text, &patch.chat_bubble_me_text);
    apply_nullable(&mut out.chat_bubble_other_bg, &patch.chat_bubble_other_bg);
    apply_nullable(&mut out.chat_bubble_other_text, &patch.chat_bubble_other_text);
    apply_nullable(&mut out.privacy_url, &patch.privacy_url);
    apply_nullable(&mut out.terms_url, &patch.terms_url);

    // If preset_id set/kept, compute token-derived fields (preset + tokens patch) as suggestions.
    if let Some(ref pid) = out.preset_id {
        let preset_tokens: Option<serde_json::Value> = sqlx::query_scalar(
            r#"select tokens from brand_presets where id = $1"#,
        )
        .bind(pid)
        .fetch_optional(pool)
        .await
        .map_err(|_| util::api_error(ApiErrorCode::InternalError))?
        .flatten();

        if let Some(preset_tokens) = preset_tokens {
            let merged = merge_tokens(&preset_tokens, &out.tokens);
            // Apply mapped tokens if those specific fields are not explicitly patched in this request.
            if !patch.primary_color.present {
                out.primary_color = token_str(&merged, "primary_color").or(out.primary_color);
            }
            if !patch.secondary_color.present {
                out.secondary_color = token_str(&merged, "secondary_color").or(out.secondary_color);
            }
            if !patch.bg_color.present {
                out.bg_color = token_str(&merged, "bg_color").or(out.bg_color);
            }
            if !patch.surface_color.present {
                out.surface_color = token_str(&merged, "surface_color").or(out.surface_color);
            }
            if !patch.border_color.present {
                out.border_color = token_str(&merged, "border_color").or(out.border_color);
            }
            if !patch.text_color.present {
                out.text_color = token_str(&merged, "text_color").or(out.text_color);
            }
            if !patch.muted_color.present {
                out.muted_color = token_str(&merged, "muted_color").or(out.muted_color);
            }
        }
    }

    Ok(out)
}

fn apply_nullable(target: &mut Option<String>, patch: &FieldPatch<String>) {
    if !patch.present {
        return;
    }
    *target = patch.value.clone();
}

fn merge_tokens(base: &serde_json::Value, patch: &serde_json::Value) -> serde_json::Value {
    let mut out = base.clone();
    if let (Some(a), Some(b)) = (out.as_object_mut(), patch.as_object()) {
        for (k, v) in b.iter() {
            a.insert(k.clone(), v.clone());
        }
    }
    out
}

fn token_str(tokens: &serde_json::Value, key: &str) -> Option<String> {
    tokens.get(key).and_then(|v| v.as_str()).map(|s| s.to_string())
}

fn to_response(r: &BrandingRow) -> BrandingResponse {
    BrandingResponse {
        organization_id: r.organization_id,
        app_name: r.app_name.clone(),
        theme: r.theme.clone(),
        ui_mode: r.ui_mode.clone(),
        ui_theme: r.ui_theme.clone(),
        logo_url: r.logo_url.clone(),
        icon_url: r.icon_url.clone(),
        primary_color: r.primary_color.clone(),
        secondary_color: r.secondary_color.clone(),
        bg_color: r.bg_color.clone(),
        surface_color: r.surface_color.clone(),
        text_color: r.text_color.clone(),
        muted_color: r.muted_color.clone(),
        border_color: r.border_color.clone(),
        selection_bg: r.selection_bg.clone(),
        selection_text: r.selection_text.clone(),
        dropdown_bg: r.dropdown_bg.clone(),
        dropdown_text: r.dropdown_text.clone(),
        chat_bubble_me_bg: r.chat_bubble_me_bg.clone(),
        chat_bubble_me_text: r.chat_bubble_me_text.clone(),
        chat_bubble_other_bg: r.chat_bubble_other_bg.clone(),
        chat_bubble_other_text: r.chat_bubble_other_text.clone(),
        privacy_url: r.privacy_url.clone(),
        terms_url: r.terms_url.clone(),
        updated_at: r.updated_at,
    }
}

fn required_string_patch(
    obj: &serde_json::Map<String, serde_json::Value>,
    key: &str,
) -> Result<FieldPatch<String>, axum::response::Response> {
    if !obj.contains_key(key) {
        return Ok(FieldPatch {
            present: false,
            value: None,
        });
    }
    let v = &obj[key];
    if v.is_null() {
        return Ok(FieldPatch {
            present: true,
            value: None,
        });
    }
    let Some(s) = v.as_str() else {
        return Err(util::api_error(ApiErrorCode::ValidationError));
    };
    let s = s.trim().to_string();
    if s.is_empty() {
        return Err(util::api_error(ApiErrorCode::ValidationError));
    }
    Ok(FieldPatch {
        present: true,
        value: Some(s),
    })
}

fn required_enum_patch(
    obj: &serde_json::Map<String, serde_json::Value>,
    key: &str,
    allowed: &[&str],
) -> Result<FieldPatch<String>, axum::response::Response> {
    if !obj.contains_key(key) {
        return Ok(FieldPatch {
            present: false,
            value: None,
        });
    }
    let v = &obj[key];
    if v.is_null() {
        return Ok(FieldPatch {
            present: true,
            value: None,
        });
    }
    let Some(s) = v.as_str() else {
        return Err(util::api_error(ApiErrorCode::ValidationError));
    };
    let s = s.trim().to_lowercase();
    if !allowed.contains(&s.as_str()) {
        return Err(util::api_error(ApiErrorCode::ValidationError));
    }
    Ok(FieldPatch {
        present: true,
        value: Some(s),
    })
}

fn nullable_string_patch(
    obj: &serde_json::Map<String, serde_json::Value>,
    key: &str,
) -> Result<FieldPatch<String>, axum::response::Response> {
    if !obj.contains_key(key) {
        return Ok(FieldPatch {
            present: false,
            value: None,
        });
    }
    let v = &obj[key];
    if v.is_null() {
        return Ok(FieldPatch {
            present: true,
            value: None,
        });
    }
    let Some(s) = v.as_str() else {
        return Err(util::api_error(ApiErrorCode::ValidationError));
    };
    let s = s.trim().to_string();
    if s.is_empty() {
        return Ok(FieldPatch {
            present: true,
            value: None,
        });
    }
    Ok(FieldPatch {
        present: true,
        value: Some(s),
    })
}

fn json_patch(
    obj: &serde_json::Map<String, serde_json::Value>,
    key: &str,
) -> Result<FieldPatch<serde_json::Value>, axum::response::Response> {
    if !obj.contains_key(key) {
        return Ok(FieldPatch {
            present: false,
            value: None,
        });
    }
    let v = &obj[key];
    if v.is_null() {
        return Ok(FieldPatch {
            present: true,
            value: Some(serde_json::json!({})),
        });
    }
    if !v.is_object() {
        return Err(util::api_error(ApiErrorCode::ValidationError));
    }
    Ok(FieldPatch {
        present: true,
        value: Some(v.clone()),
    })
}

fn validate_tokens_object(v: &serde_json::Value) -> Result<(), axum::response::Response> {
    let Some(map) = v.as_object() else {
        return Err(util::api_error(ApiErrorCode::ValidationError));
    };
    for (k, val) in map.iter() {
        // Known token keys (server-validated)
        let is_color = matches!(
            k.as_str(),
            "primary_color"
                | "secondary_color"
                | "bg_color"
                | "surface_color"
                | "border_color"
                | "text_color"
                | "muted_color"
                | "selection_bg"
                | "selection_text"
                | "dropdown_bg"
                | "dropdown_text"
                | "chat_bubble_me_bg"
                | "chat_bubble_me_text"
                | "chat_bubble_other_bg"
                | "chat_bubble_other_text"
        );
        let is_url = matches!(k.as_str(), "logo_url" | "icon_url" | "privacy_url" | "terms_url");

        if is_color {
            let Some(s) = val.as_str() else {
                return Err(util::api_error(ApiErrorCode::ValidationError));
            };
            if !is_hex_color(s) {
                return Err(util::api_error(ApiErrorCode::ValidationError));
            }
        }
        if is_url {
            if val.is_null() {
                continue;
            }
            let Some(s) = val.as_str() else {
                return Err(util::api_error(ApiErrorCode::ValidationError));
            };
            if !is_safe_url(s) {
                return Err(util::api_error(ApiErrorCode::ValidationError));
            }
        }
    }
    Ok(())
}

fn nullable_url_patch(
    obj: &serde_json::Map<String, serde_json::Value>,
    key: &str,
) -> Result<FieldPatch<String>, axum::response::Response> {
    let p = nullable_string_patch(obj, key)?;
    if !p.present {
        return Ok(p);
    }
    if let Some(ref url) = p.value {
        if !is_safe_url(url) {
            return Err(util::api_error(ApiErrorCode::ValidationError));
        }
    }
    Ok(p)
}

// Like nullable_url_patch but also accepts data URLs for logo/icon fields.
fn nullable_logo_patch(
    obj: &serde_json::Map<String, serde_json::Value>,
    key: &str,
) -> Result<FieldPatch<String>, axum::response::Response> {
    let p = nullable_string_patch(obj, key)?;
    if !p.present {
        return Ok(p);
    }
    if let Some(ref url) = p.value {
        if !is_safe_url(url) && !is_image_data_url(url) {
            return Err(util::api_error(ApiErrorCode::ValidationError));
        }
        // Enforce a 1.6 MB cap on data URLs to stay within reasonable DB column size.
        if url.starts_with("data:") && url.len() > 1_600_000 {
            return Err(util::api_error(ApiErrorCode::ValidationError));
        }
    }
    Ok(p)
}

fn nullable_color_patch(
    obj: &serde_json::Map<String, serde_json::Value>,
    key: &str,
) -> Result<FieldPatch<String>, axum::response::Response> {
    let p = nullable_string_patch(obj, key)?;
    if !p.present {
        return Ok(p);
    }
    if let Some(ref c) = p.value {
        if !is_hex_color(c) {
            return Err(util::api_error(ApiErrorCode::ValidationError));
        }
    }
    Ok(p)
}

fn is_safe_url(url: &str) -> bool {
    let u = url.trim();
    if u.is_empty() {
        return false;
    }
    // Allow http(s) only.
    u.starts_with("http://") || u.starts_with("https://")
}

fn is_image_data_url(url: &str) -> bool {
    url.starts_with("data:image/")
}

fn is_hex_color(s: &str) -> bool {
    let t = s.trim();
    if !t.starts_with('#') {
        return false;
    }
    let hex = &t[1..];
    (hex.len() == 6 || hex.len() == 8) && hex.chars().all(|c| c.is_ascii_hexdigit())
}

fn validate_branding_row(r: &BrandingRow) -> Result<(), axum::response::Response> {
    // Contrast checks: ensure text has sufficient contrast against bg/surface.
    // We keep this soft (2.5) to avoid breaking existing themes but still catch egregious combos.
    let min_ratio = 2.5;
    if let (Some(bg), Some(text)) = (r.bg_color.as_deref(), r.text_color.as_deref()) {
        if contrast_ratio(bg, text).unwrap_or(10.0) < min_ratio {
            return Err(util::api_error_msg(
                ApiErrorCode::ValidationError,
                "text_color has insufficient contrast against bg_color",
            ));
        }
    }
    if let (Some(surface), Some(text)) = (r.surface_color.as_deref(), r.text_color.as_deref()) {
        if contrast_ratio(surface, text).unwrap_or(10.0) < min_ratio {
            return Err(util::api_error_msg(
                ApiErrorCode::ValidationError,
                "text_color has insufficient contrast against surface_color",
            ));
        }
    }
    Ok(())
}

fn contrast_ratio(a: &str, b: &str) -> Option<f64> {
    let la = rel_luminance(parse_rgb(a)?);
    let lb = rel_luminance(parse_rgb(b)?);
    let (hi, lo) = if la > lb { (la, lb) } else { (lb, la) };
    Some((hi + 0.05) / (lo + 0.05))
}

fn parse_rgb(s: &str) -> Option<(u8, u8, u8)> {
    let t = s.trim().trim_start_matches('#');
    if t.len() != 6 && t.len() != 8 {
        return None;
    }
    let r = u8::from_str_radix(&t[0..2], 16).ok()?;
    let g = u8::from_str_radix(&t[2..4], 16).ok()?;
    let b = u8::from_str_radix(&t[4..6], 16).ok()?;
    Some((r, g, b))
}

fn rel_luminance((r, g, b): (u8, u8, u8)) -> f64 {
    fn to_linear(u: u8) -> f64 {
        let c = (u as f64) / 255.0;
        if c <= 0.04045 {
            c / 12.92
        } else {
            ((c + 0.055) / 1.055).powf(2.4)
        }
    }
    0.2126 * to_linear(r) + 0.7152 * to_linear(g) + 0.0722 * to_linear(b)
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
