use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Unauthorized: {0}")]
    Unauthorized(String),

    #[error("Bad request: {0}")]
    BadRequest(String),

    #[error("Insufficient funds: required {required}, available {available}")]
    InsufficientFunds { required: i64, available: i64 },

    #[error("Duplicate idempotency key with different request")]
    DuplicateIdempotencyKey,

    #[error("Rate limit exceeded")]
    RateLimitExceeded,

    #[error("Internal server error: {0}")]
    Internal(String),

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Webhook delivery failed: {0}")]
    WebhookDeliveryFailed(String),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ErrorResponse {
    pub error: ErrorDetail,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ErrorDetail {
    pub code: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<HashMap<String, serde_json::Value>>,
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, code, message, details) = match self {
            AppError::Database(ref e) => {
                tracing::error!("Database error: {:?}", e);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "DATABASE_ERROR",
                    "An internal database error occurred",
                    None,
                )
            }
            AppError::NotFound(ref msg) => {
                (StatusCode::NOT_FOUND, "RESOURCE_NOT_FOUND", msg.as_str(), None)
            }
            AppError::Unauthorized(ref msg) => {
                (StatusCode::UNAUTHORIZED, "UNAUTHORIZED", msg.as_str(), None)
            }
            AppError::BadRequest(ref msg) => {
                (StatusCode::BAD_REQUEST, "INVALID_REQUEST", msg.as_str(), None)
            }
            AppError::InsufficientFunds { required, available } => {
                let mut details = HashMap::new();
                details.insert("required".to_string(), serde_json::json!(required));
                details.insert("available".to_string(), serde_json::json!(available));
                (
                    StatusCode::BAD_REQUEST,
                    "INSUFFICIENT_FUNDS",
                    "Account does not have sufficient balance",
                    Some(details),
                )
            }
            AppError::DuplicateIdempotencyKey => (
                StatusCode::CONFLICT,
                "DUPLICATE_IDEMPOTENCY_KEY",
                "Idempotency key already used with different request",
                None,
            ),
            AppError::RateLimitExceeded => (
                StatusCode::TOO_MANY_REQUESTS,
                "RATE_LIMIT_EXCEEDED",
                "Rate limit exceeded, please try again later",
                None,
            ),
            AppError::Internal(ref msg) => {
                tracing::error!("Internal error: {}", msg);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "INTERNAL_ERROR",
                    "An internal error occurred",
                    None,
                )
            }
            AppError::Validation(ref msg) => {
                (StatusCode::BAD_REQUEST, "VALIDATION_ERROR", msg.as_str(), None)
            }
            AppError::WebhookDeliveryFailed(ref msg) => {
                tracing::warn!("Webhook delivery failed: {}", msg);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "WEBHOOK_DELIVERY_FAILED",
                    msg.as_str(),
                    None,
                )
            }
        };

        let error_response = ErrorResponse {
            error: ErrorDetail {
                code: code.to_string(),
                message: message.to_string(),
                details,
            },
        };

        (status, Json(error_response)).into_response()
    }
}

pub type AppResult<T> = Result<T, AppError>;
