use crate::{
    models::{auth, orgs},
    state::AppState,
};
use axum::{
    http::HeaderMap,
    routing::{get, post},
    Json, Router,
};
use uuid::Uuid;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/orgs", post(create_org))
        .route("/orgs/current", get(current_org))
}

async fn create_org(
    axum::extract::State(state): axum::extract::State<AppState>,
    auth::CurrentAuth(ctx): auth::CurrentAuth,
    Json(req): Json<orgs::CreateOrgRequest>,
) -> Result<Json<orgs::CreateOrgResponse>, auth::ApiError> {
    let organization = state.orgs_service.create_org(ctx.user_id, req).await?;
    Ok(Json(orgs::CreateOrgResponse { organization }))
}

async fn current_org(
    axum::extract::State(state): axum::extract::State<AppState>,
    auth::CurrentAuth(ctx): auth::CurrentAuth,
    headers: HeaderMap,
) -> Result<Json<orgs::CurrentOrgResponse>, auth::ApiError> {
    let requested_org_id = parse_org_id_header(&headers)?;
    let organization = state
        .orgs_service
        .resolve_org_for_user(ctx.user_id, requested_org_id)
        .await?;
    Ok(Json(orgs::CurrentOrgResponse { organization }))
}

fn parse_org_id_header(headers: &HeaderMap) -> Result<Option<Uuid>, auth::ApiError> {
    let Some(v) = headers.get("x-organization-id") else {
        return Ok(None);
    };
    let s = v.to_str().map_err(|_| auth::ApiError::bad_request())?;
    let id = Uuid::parse_str(s).map_err(|_| auth::ApiError::bad_request())?;
    Ok(Some(id))
}
