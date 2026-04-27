use axum::{
    http::{Request, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;

#[derive(Serialize)]
pub struct ErrorBody {
    error: &'static str,
}

pub async fn json_error_mapper(
    req: Request<axum::body::Body>,
    next: Next,
) -> Response {
    let res = next.run(req).await;
    let code = res.status();

    // Normalize some common non-JSON errors into a small JSON body.
    // (TimeoutLayer returns an empty body by default.)
    if code == StatusCode::GATEWAY_TIMEOUT {
        return (code, Json(ErrorBody { error: "timeout" })).into_response();
    }

    if code == StatusCode::PAYLOAD_TOO_LARGE {
        return (code, Json(ErrorBody { error: "payload_too_large" })).into_response();
    }

    if code == StatusCode::NOT_FOUND {
        return (code, Json(ErrorBody { error: "not_found" })).into_response();
    }

    if code == StatusCode::METHOD_NOT_ALLOWED {
        return (code, Json(ErrorBody { error: "method_not_allowed" })).into_response();
    }

    // Preserve any other response (including JSON errors from handlers).
    res
}
