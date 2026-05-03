use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ApiErrorCode {
    ValidationError,
    Unauthenticated,
    PermissionDenied,
    NotFound,
    Conflict,
    RateLimited,
    InternalError,
}

impl ApiErrorCode {
    pub fn status(self) -> StatusCode {
        match self {
            ApiErrorCode::ValidationError => StatusCode::BAD_REQUEST,
            ApiErrorCode::Unauthenticated => StatusCode::UNAUTHORIZED,
            ApiErrorCode::PermissionDenied => StatusCode::FORBIDDEN,
            ApiErrorCode::NotFound => StatusCode::NOT_FOUND,
            ApiErrorCode::Conflict => StatusCode::CONFLICT,
            ApiErrorCode::RateLimited => StatusCode::TOO_MANY_REQUESTS,
            ApiErrorCode::InternalError => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    pub fn default_message(self) -> &'static str {
        match self {
            ApiErrorCode::ValidationError => "The request was invalid.",
            ApiErrorCode::Unauthenticated => "Authentication is required.",
            ApiErrorCode::PermissionDenied => "You do not have permission to perform this action.",
            ApiErrorCode::NotFound => "The requested resource was not found.",
            ApiErrorCode::Conflict => "The request could not be completed due to a conflict.",
            ApiErrorCode::RateLimited => "Too many requests. Please try again later.",
            ApiErrorCode::InternalError => "An internal error occurred.",
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ApiErrorBody {
    pub code: ApiErrorCode,
    pub message: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ApiErrorResponse {
    pub error: ApiErrorBody,
}

#[derive(Debug, Clone)]
pub struct ApiError {
    pub code: ApiErrorCode,
    pub message: String,
}

impl ApiError {
    pub fn new(code: ApiErrorCode) -> Self {
        Self {
            code,
            message: code.default_message().to_string(),
        }
    }

    pub fn with_message(code: ApiErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }

    pub fn into_response(self) -> Response {
        let status = self.code.status();
        (
            status,
            Json(ApiErrorResponse {
                error: ApiErrorBody {
                    code: self.code,
                    message: self.message,
                },
            }),
        )
            .into_response()
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        self.into_response()
    }
}
