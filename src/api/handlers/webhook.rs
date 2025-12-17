use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Extension, Json,
};
use uuid::Uuid;
use validator::Validate;

use crate::api::dto::{CreateWebhookRequest, ListWebhooksResponse, WebhookResponse};
use crate::api::routes::AppState;
use crate::middleware::auth::AuthenticatedAccount;
use crate::utils::AppError;

pub async fn create_webhook(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthenticatedAccount>,
    Json(payload): Json<CreateWebhookRequest>,
) -> Result<impl IntoResponse, AppError> {
    payload
        .validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;

    let webhook = state
        .webhook_service
        .create_webhook(auth.account_id, payload.url, payload.secret, payload.events)
        .await?;

    Ok((StatusCode::CREATED, Json(WebhookResponse::from(webhook))))
}

pub async fn list_webhooks(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthenticatedAccount>,
) -> Result<impl IntoResponse, AppError> {
    let webhooks = state.webhook_service.list_webhooks(auth.account_id).await?;

    let response = ListWebhooksResponse {
        webhooks: webhooks.into_iter().map(WebhookResponse::from).collect(),
    };

    Ok(Json(response))
}

pub async fn delete_webhook(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthenticatedAccount>,
    Path(webhook_id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    // First verify the webhook belongs to this account
    let webhook = state.webhook_service.get_webhook(webhook_id).await?;

    if webhook.account_id != auth.account_id {
        return Err(AppError::Unauthorized(
            "Cannot delete another account's webhook".to_string(),
        ));
    }

    state.webhook_service.delete_webhook(webhook_id).await?;

    Ok(StatusCode::NO_CONTENT)
}
