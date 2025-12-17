use axum::{
    extract::{Request, State},
    middleware::Next,
    response::Response,
};
use std::sync::Arc;

use crate::domain::repositories::ApiKeyRepository;
use crate::utils::AppError;

#[derive(Clone)]
pub struct AuthenticatedAccount {
    pub account_id: uuid::Uuid,
    pub api_key_id: uuid::Uuid,
}

pub async fn auth_middleware(
    State(api_key_repo): State<Arc<dyn ApiKeyRepository>>,
    mut request: Request,
    next: Next,
) -> Result<Response, AppError> {
    let auth_header = request
        .headers()
        .get("Authorization")
        .and_then(|h| h.to_str().ok())
        .ok_or_else(|| AppError::Unauthorized("Missing Authorization header".to_string()))?;

    let api_key = auth_header
        .strip_prefix("Bearer ")
        .ok_or_else(|| AppError::Unauthorized("Invalid Authorization format".to_string()))?;

    // Find all API keys and verify
    // Note: In production, consider caching this or using a more efficient lookup
    let found_key = find_api_key_by_value(&*api_key_repo, api_key).await?;

    if !found_key.is_valid() {
        return Err(AppError::Unauthorized("API key is not valid".to_string()));
    }

    // Update last used timestamp
    api_key_repo.update_last_used(found_key.id).await?;

    let auth_account = AuthenticatedAccount {
        account_id: found_key.account_id,
        api_key_id: found_key.id,
    };

    request.extensions_mut().insert(auth_account);

    Ok(next.run(request).await)
}

async fn find_api_key_by_value(
    repo: &dyn ApiKeyRepository,
    api_key: &str,
) -> Result<crate::domain::models::ApiKey, AppError> {
    // Since bcrypt generates different hashes each time, we can't hash the input and match
    // Instead, we need to fetch all active keys and verify against each one using bcrypt::verify
    // In production, you'd optimize this with:
    // 1. Store a prefix of the key for quick lookup to reduce candidates
    // 2. Cache active keys in memory
    // 3. Use a faster hash function with a salt for comparison

    let active_keys = repo.get_all_active().await?;

    for key in active_keys {
        if bcrypt::verify(api_key, &key.key_hash).unwrap_or(false) {
            return Ok(key);
        }
    }

    Err(AppError::Unauthorized("Invalid API key".to_string()))
}
