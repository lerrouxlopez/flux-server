use crate::{models::auth, state::AppState};
use axum::{routing::post, Json, Router};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/auth/register", post(register))
        .route("/auth/login", post(login))
        .route("/auth/refresh", post(refresh))
        .route("/auth/logout", post(logout))
}

async fn register(
    axum::extract::State(state): axum::extract::State<AppState>,
    Json(req): Json<auth::RegisterRequest>,
) -> Result<Json<auth::AuthResponse>, auth::ApiError> {
    let res = state.auth_service.register(req).await?;
    Ok(Json(res))
}

async fn login(
    axum::extract::State(state): axum::extract::State<AppState>,
    Json(req): Json<auth::LoginRequest>,
) -> Result<Json<auth::AuthResponse>, auth::ApiError> {
    let res = state.auth_service.login(req).await?;
    Ok(Json(res))
}

async fn refresh(
    axum::extract::State(state): axum::extract::State<AppState>,
    Json(req): Json<auth::RefreshRequest>,
) -> Result<Json<auth::AuthResponse>, auth::ApiError> {
    let res = state.auth_service.refresh(req).await?;
    Ok(Json(res))
}

async fn logout(
    axum::extract::State(state): axum::extract::State<AppState>,
    auth::CurrentAuth(ctx): auth::CurrentAuth,
) -> Result<Json<auth::LogoutResponse>, auth::ApiError> {
    state.auth_service.logout(ctx.session_id).await?;
    Ok(Json(auth::LogoutResponse { ok: true }))
}
