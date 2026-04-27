use crate::{models::health::HealthResponse, state::AppState};
use axum::{extract::State, http::StatusCode, routing::get, Json, Router};

pub fn router() -> Router<AppState> {
    Router::new().route("/health", get(health))
}

async fn health(State(state): State<AppState>) -> Result<Json<HealthResponse>, StatusCode> {
    state
        .health_service
        .check()
        .await
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    Ok(Json(HealthResponse { status: "ok" }))
}

