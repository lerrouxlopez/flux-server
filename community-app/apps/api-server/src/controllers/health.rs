use crate::{models::health::HealthResponse, state::AppState};
use axum::{extract::State, http::StatusCode, routing::get, Json, Router};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/healthz", get(healthz))
        .route("/readyz", get(readyz))
}

async fn healthz() -> Json<HealthResponse> {
    Json(HealthResponse { status: "ok" })
}

async fn readyz(State(state): State<AppState>) -> Result<Json<HealthResponse>, StatusCode> {
    state
        .readiness_service
        .check()
        .await
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    Ok(Json(HealthResponse { status: "ok" }))
}
