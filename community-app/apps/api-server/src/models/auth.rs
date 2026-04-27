use axum::{
    extract::FromRequestParts,
    http::{request::Parts, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

#[derive(Debug, Deserialize)]
pub struct RegisterRequest {
    pub email: String,
    pub display_name: String,
    pub password: String,
}

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Deserialize)]
pub struct RefreshRequest {
    pub refresh_token: String,
}

#[derive(Debug, Serialize)]
pub struct LogoutResponse {
    pub ok: bool,
}

#[derive(Debug, Serialize)]
pub struct AuthResponse {
    pub access_token: String,
    pub refresh_token: String,
    pub user: UserView,
}

#[derive(Debug, Serialize)]
pub struct UserView {
    pub id: Uuid,
    pub email: String,
    pub display_name: String,
    pub created_at: OffsetDateTime,
}

#[derive(Debug, Clone)]
pub struct AuthContext {
    pub user_id: Uuid,
    pub session_id: Uuid,
}

#[derive(Debug)]
pub struct CurrentAuth(pub AuthContext);

#[derive(Debug, Serialize)]
struct ErrorBody {
    error: &'static str,
}

#[derive(Debug)]
pub struct ApiError {
    pub status: StatusCode,
    pub error: &'static str,
}

impl ApiError {
    pub fn bad_request() -> Self {
        Self {
            status: StatusCode::BAD_REQUEST,
            error: "bad_request",
        }
    }

    pub fn unauthorized() -> Self {
        Self {
            status: StatusCode::UNAUTHORIZED,
            error: "unauthorized",
        }
    }

    pub fn conflict() -> Self {
        Self {
            status: StatusCode::CONFLICT,
            error: "conflict",
        }
    }

    pub fn internal() -> Self {
        Self {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            error: "internal",
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (self.status, Json(ErrorBody { error: self.error })).into_response()
    }
}

impl FromRequestParts<crate::state::AppState> for CurrentAuth {
    type Rejection = ApiError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &crate::state::AppState,
    ) -> Result<Self, Self::Rejection> {
        let auth_header = parts
            .headers
            .get(axum::http::header::AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .ok_or_else(ApiError::unauthorized)?;

        let token = auth_header
            .strip_prefix("Bearer ")
            .ok_or_else(ApiError::unauthorized)?;

        let ctx = state.auth_service.authenticate(token).await?;
        Ok(CurrentAuth(ctx))
    }
}
