use crate::{models::auth::ApiError, state::AppState};
use axum::{
    extract::State,
    http::{header, Request},
    middleware::Next,
    response::Response,
};

pub async fn auth_extractor(
    State(state): State<AppState>,
    mut req: Request<axum::body::Body>,
    next: Next,
) -> Result<Response, ApiError> {
    let Some(auth_header) = req
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
    else {
        return Ok(next.run(req).await);
    };

    let Some(token) = auth_header.strip_prefix("Bearer ") else {
        return Err(ApiError::unauthorized());
    };

    let ctx = state.auth_service.authenticate(token).await?;
    req.extensions_mut().insert(ctx);
    Ok(next.run(req).await)
}
