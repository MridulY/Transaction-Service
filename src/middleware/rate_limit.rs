use axum::{
    extract::{Request, State},
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
};
use std::sync::Arc;

use crate::infrastructure::rate_limiter::RateLimiter;
use crate::middleware::auth::AuthenticatedAccount;

/// Rate limiting middleware
///
/// Enforces per-API-key rate limits using token bucket algorithm
/// Must be applied after authentication middleware to access AuthenticatedAccount
pub async fn rate_limit_middleware(
    State(rate_limiter): State<Arc<RateLimiter>>,
    request: Request,
    next: Next,
) -> Result<Response, RateLimitResponse> {
    // Extract authenticated account from request extensions
    let auth_account = request
        .extensions()
        .get::<AuthenticatedAccount>()
        .ok_or_else(|| RateLimitResponse::error("Authentication required for rate limiting"))?;

    // Check rate limit
    let limit_info = rate_limiter.check_with_info(auth_account.api_key_id).await;

    if !limit_info.allowed {
        return Err(RateLimitResponse::rate_limited(
            limit_info.limit,
            limit_info.remaining,
            limit_info.reset_after_seconds,
        ));
    }

    // Add rate limit headers to response
    let mut response = next.run(request).await;

    let headers = response.headers_mut();
    headers.insert(
        "X-RateLimit-Limit",
        limit_info.limit.to_string().parse().unwrap(),
    );
    headers.insert(
        "X-RateLimit-Remaining",
        limit_info.remaining.to_string().parse().unwrap(),
    );
    headers.insert(
        "X-RateLimit-Reset",
        limit_info.reset_after_seconds.to_string().parse().unwrap(),
    );

    Ok(response)
}

/// Rate limit response with proper status code and headers
#[derive(Debug)]
pub struct RateLimitResponse {
    status: StatusCode,
    message: String,
    limit: Option<u32>,
    remaining: Option<u32>,
    reset_after: Option<u64>,
}

impl RateLimitResponse {
    pub fn rate_limited(limit: u32, remaining: u32, reset_after: u64) -> Self {
        Self {
            status: StatusCode::TOO_MANY_REQUESTS,
            message: "Rate limit exceeded".to_string(),
            limit: Some(limit),
            remaining: Some(remaining),
            reset_after: Some(reset_after),
        }
    }

    pub fn error(message: &str) -> Self {
        Self {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            message: message.to_string(),
            limit: None,
            remaining: None,
            reset_after: None,
        }
    }
}

impl IntoResponse for RateLimitResponse {
    fn into_response(self) -> Response {
        let mut response = (
            self.status,
            serde_json::json!({
                "error": self.message,
            })
            .to_string(),
        )
            .into_response();

        let headers = response.headers_mut();

        if let Some(limit) = self.limit {
            headers.insert("X-RateLimit-Limit", limit.to_string().parse().unwrap());
        }

        if let Some(remaining) = self.remaining {
            headers.insert(
                "X-RateLimit-Remaining",
                remaining.to_string().parse().unwrap(),
            );
        }

        if let Some(reset_after) = self.reset_after {
            headers.insert(
                "X-RateLimit-Reset",
                reset_after.to_string().parse().unwrap(),
            );
            headers.insert("Retry-After", reset_after.to_string().parse().unwrap());
        }

        response
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rate_limit_response_creation() {
        let response = RateLimitResponse::rate_limited(100, 0, 60);
        assert_eq!(response.status, StatusCode::TOO_MANY_REQUESTS);
        assert_eq!(response.limit, Some(100));
        assert_eq!(response.remaining, Some(0));
        assert_eq!(response.reset_after, Some(60));
    }

    #[test]
    fn test_error_response_creation() {
        let response = RateLimitResponse::error("Test error");
        assert_eq!(response.status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(response.message, "Test error");
    }
}
