use crate::{controllers, state::AppState};
use axum::Router;
use tower_http::trace::TraceLayer;

pub fn router(state: AppState) -> Router {
    Router::new()
        .nest("/", controllers::health::router())
        .with_state(state)
        .layer(TraceLayer::new_for_http())
}

