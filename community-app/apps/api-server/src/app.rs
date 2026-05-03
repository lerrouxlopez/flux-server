use crate::{AppState, AuthContext};
use axum::{
    http::{HeaderMap, StatusCode},
    middleware,
    response::Response,
    routing::get,
    Router,
};
use tower::ServiceBuilder;
use tower_http::{
    catch_panic::CatchPanicLayer,
    cors::{Any, CorsLayer},
    request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer},
    timeout::TimeoutLayer,
    trace::TraceLayer,
};
use tracing::Span;
use uuid::Uuid;

use api::{ApiError, ApiErrorCode};

pub fn build_app(cfg: &config::AppConfig, state: AppState) -> Router {
    let auth_state = state.clone();

    let request_id_header = axum::http::HeaderName::from_static("x-request-id");
    let cors = cors_layer(cfg);

    Router::new()
        .nest("/auth", crate::routes_auth::router())
        .nest("/orgs", crate::routes_orgs::router())
        .merge(crate::routes_channels::router())
        .merge(crate::routes_messages::router())
        .merge(crate::routes_media::router())
        .merge(crate::routes_branding::router())
        .route("/healthz", get(|| async { "ok" }))
        .route("/readyz", get(crate::readyz))
        .layer(
            ServiceBuilder::new()
                .layer(SetRequestIdLayer::new(
                    request_id_header.clone(),
                    MakeRequestUuid,
                ))
                .layer(PropagateRequestIdLayer::new(request_id_header))
                .layer(CatchPanicLayer::new())
                .layer(
                    TraceLayer::new_for_http()
                        .make_span_with(|req: &axum::http::Request<_>| {
                            let request_id = req
                                .headers()
                                .get("x-request-id")
                                .and_then(|v| v.to_str().ok())
                                .unwrap_or("");
                            tracing::info_span!(
                                "http.request",
                                request_id = %request_id,
                                method = %req.method(),
                                path = %req.uri().path(),
                                status = tracing::field::Empty,
                                latency_ms = tracing::field::Empty,
                                user_id = tracing::field::Empty,
                                organization_id = tracing::field::Empty,
                            )
                        })
                        .on_response(
                            |res: &axum::http::Response<_>,
                             latency: std::time::Duration,
                             span: &Span| {
                                span.record("status", res.status().as_u16());
                                span.record("latency_ms", latency.as_millis() as u64);
                                tracing::info!(
                                    parent: span,
                                    status = %res.status().as_u16(),
                                    latency_ms = %(latency.as_millis() as u64),
                                    "request finished"
                                );
                            },
                        ),
                )
                .layer(cors)
                .layer(axum::extract::DefaultBodyLimit::max(1024 * 1024))
                .layer(TimeoutLayer::with_status_code(
                    StatusCode::REQUEST_TIMEOUT,
                    std::time::Duration::from_secs(10),
                ))
                .into_inner(),
        )
        .layer(middleware::from_fn_with_state(auth_state, auth_middleware))
        .layer(middleware::from_fn(error_middleware))
        .with_state(state)
}

async fn auth_middleware(
    axum::extract::State(state): axum::extract::State<AppState>,
    mut req: axum::http::Request<axum::body::Body>,
    next: middleware::Next,
) -> Response {
    if let Some(auth) = bearer_token(req.headers()) {
        if let Ok(claims) = auth::decode_access_token(&state.auth_cfg, auth) {
            if let Ok(user_id) = Uuid::parse_str(&claims.sub) {
                req.extensions_mut().insert(AuthContext { user_id });
                Span::current().record("user_id", tracing::field::display(user_id));
            }
        }
    }
    next.run(req).await
}

fn bearer_token(headers: &HeaderMap) -> Option<&str> {
    let value = headers
        .get(axum::http::header::AUTHORIZATION)?
        .to_str()
        .ok()?;
    value
        .strip_prefix("Bearer ")
        .or_else(|| value.strip_prefix("bearer "))
}

async fn error_middleware(
    req: axum::http::Request<axum::body::Body>,
    next: middleware::Next,
) -> Response {
    let res = next.run(req).await;
    if res.status() == StatusCode::PAYLOAD_TOO_LARGE {
        return ApiError::with_message(ApiErrorCode::ValidationError, "Request body too large.")
            .into_response();
    }
    res
}

fn cors_layer(cfg: &config::AppConfig) -> CorsLayer {
    let raw = std::env::var("CORS_ALLOW_ORIGINS").unwrap_or_default();
    if raw.trim().is_empty() || cfg.app_env == "local" {
        return CorsLayer::new()
            .allow_origin(Any)
            .allow_methods(Any)
            .allow_headers(Any);
    }

    let mut origins = Vec::new();
    for part in raw.split(',') {
        let o = part.trim();
        if o.is_empty() {
            continue;
        }
        if let Ok(v) = o.parse::<axum::http::HeaderValue>() {
            origins.push(v);
        }
    }

    if origins.is_empty() {
        CorsLayer::new()
            .allow_origin(Any)
            .allow_methods(Any)
            .allow_headers(Any)
    } else {
        CorsLayer::new()
            .allow_origin(origins)
            .allow_methods(Any)
            .allow_headers(Any)
    }
}
