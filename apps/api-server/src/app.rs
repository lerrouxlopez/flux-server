use crate::{controllers, state::AppState};
use axum::Router;
use tower_http::trace::TraceLayer;

pub fn router(state: AppState) -> Router {
    Router::new()
        .merge(controllers::health::router())
        .merge(controllers::auth::router())
        .with_state(state)
        .layer(TraceLayer::new_for_http())
}
