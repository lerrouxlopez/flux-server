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
        .route("/orgs", get(list_orgs))
        .route("/orgs/current", get(current_org))
        .route("/orgs/:org_id", get(get_org))
        .route("/orgs/:org_id/members", get(list_members))
        .route("/orgs/:org_id/invites", post(create_invite))
        .route("/orgs/:org_id/members", post(add_member))
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

async fn list_orgs(
    axum::extract::State(state): axum::extract::State<AppState>,
    auth::CurrentAuth(ctx): auth::CurrentAuth,
) -> Result<Json<orgs::ListOrgsResponse>, auth::ApiError> {
    let organizations = state.orgs_service.list_orgs(ctx.user_id).await?;
    Ok(Json(orgs::ListOrgsResponse { organizations }))
}

async fn get_org(
    axum::extract::State(state): axum::extract::State<AppState>,
    auth::CurrentAuth(ctx): auth::CurrentAuth,
    axum::extract::Path(org_id): axum::extract::Path<Uuid>,
) -> Result<Json<orgs::GetOrgResponse>, auth::ApiError> {
    let organization = state.orgs_service.get_org(ctx.user_id, org_id).await?;
    Ok(Json(orgs::GetOrgResponse { organization }))
}

async fn list_members(
    axum::extract::State(state): axum::extract::State<AppState>,
    auth::CurrentAuth(ctx): auth::CurrentAuth,
    axum::extract::Path(org_id): axum::extract::Path<Uuid>,
) -> Result<Json<orgs::ListMembersResponse>, auth::ApiError> {
    let members = state.orgs_service.list_members(ctx.user_id, org_id).await?;
    Ok(Json(orgs::ListMembersResponse { members }))
}

async fn create_invite(
    axum::extract::State(state): axum::extract::State<AppState>,
    auth::CurrentAuth(ctx): auth::CurrentAuth,
    axum::extract::Path(org_id): axum::extract::Path<Uuid>,
    Json(req): Json<orgs::CreateInviteRequest>,
) -> Result<Json<orgs::CreateInviteResponse>, auth::ApiError> {
    let invite = state.orgs_service.create_invite(ctx.user_id, org_id, req).await?;
    Ok(Json(invite))
}

async fn add_member(
    axum::extract::State(state): axum::extract::State<AppState>,
    auth::CurrentAuth(ctx): auth::CurrentAuth,
    axum::extract::Path(org_id): axum::extract::Path<Uuid>,
    Json(req): Json<orgs::AddMemberRequest>,
) -> Result<Json<orgs::AddMemberResponse>, auth::ApiError> {
    state.orgs_service.add_member(ctx.user_id, org_id, req).await?;
    Ok(Json(orgs::AddMemberResponse { ok: true }))
}

fn parse_org_id_header(headers: &HeaderMap) -> Result<Option<Uuid>, auth::ApiError> {
    let Some(v) = headers.get("x-organization-id") else {
        return Ok(None);
    };
    let s = v.to_str().map_err(|_| auth::ApiError::bad_request())?;
    let id = Uuid::parse_str(s).map_err(|_| auth::ApiError::bad_request())?;
    Ok(Some(id))
}
