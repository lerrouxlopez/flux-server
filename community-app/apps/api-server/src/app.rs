use crate::{controllers, middleware, state::AppState};
use axum::{
    extract::DefaultBodyLimit,
    middleware::{from_fn, from_fn_with_state},
    Router,
};
use std::time::Duration;
use tower_http::{
    cors::{Any, CorsLayer},
    request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer},
    timeout::TimeoutLayer,
    trace::TraceLayer,
};

pub fn router(state: AppState) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let timeout_seconds: u64 = config::parse("HTTP_TIMEOUT_SECONDS")
        .ok()
        .flatten()
        .unwrap_or(15);

    let json_limit_bytes: usize = config::parse("JSON_BODY_LIMIT_BYTES")
        .ok()
        .flatten()
        .unwrap_or(1_048_576);

    Router::new()
        .merge(controllers::health::router())
        .merge(controllers::auth::router())
        .merge(controllers::orgs::router())
        .merge(controllers::channels::router())
        .merge(controllers::messages::router())
        .with_state(state.clone())
        .layer(from_fn_with_state(state, middleware::auth::auth_extractor))
        .layer(DefaultBodyLimit::max(json_limit_bytes))
        .layer(TimeoutLayer::with_status_code(
            axum::http::StatusCode::GATEWAY_TIMEOUT,
            Duration::from_secs(timeout_seconds),
        ))
        .layer(SetRequestIdLayer::x_request_id(MakeRequestUuid))
        .layer(PropagateRequestIdLayer::x_request_id())
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .layer(from_fn(middleware::errors::json_error_mapper))
}
