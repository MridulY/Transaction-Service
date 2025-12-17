use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    Extension, Json,
};
use serde::Deserialize;
use uuid::Uuid;
use validator::Validate;

use crate::api::dto::{
    CreateCreditRequest, CreateDebitRequest, CreateTransferRequest, ListTransactionsResponse,
    TransactionResponse,
};
use crate::api::routes::AppState;
use crate::middleware::auth::AuthenticatedAccount;
use crate::utils::AppError;

#[derive(Deserialize)]
pub struct ListQuery {
    #[serde(default = "default_limit")]
    limit: i64,
    #[serde(default)]
    offset: i64,
}

fn default_limit() -> i64 {
    50
}

pub async fn create_credit(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthenticatedAccount>,
    headers: HeaderMap,
    Json(payload): Json<CreateCreditRequest>,
) -> Result<impl IntoResponse, AppError> {
    payload
        .validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;

    // Verify the account making the request owns the target account
    if payload.account_id != auth.account_id {
        return Err(AppError::Unauthorized(
            "Cannot credit another account".to_string(),
        ));
    }

    let idempotency_key = headers
        .get("Idempotency-Key")
        .and_then(|h| h.to_str().ok())
        .map(|s| s.to_string());

    let transaction = state.transaction_service
        .create_credit(
            payload.account_id,
            payload.amount,
            payload.currency,
            payload.description,
            idempotency_key,
        )
        .await?;

    // Queue webhook
    state.webhook_service
        .queue_webhook(&transaction, "transaction.completed")
        .await
        .ok();

    Ok((StatusCode::CREATED, Json(TransactionResponse::from(transaction))))
}

pub async fn create_debit(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthenticatedAccount>,
    headers: HeaderMap,
    Json(payload): Json<CreateDebitRequest>,
) -> Result<impl IntoResponse, AppError> {
    payload
        .validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;

    if payload.account_id != auth.account_id {
        return Err(AppError::Unauthorized(
            "Cannot debit another account".to_string(),
        ));
    }

    let idempotency_key = headers
        .get("Idempotency-Key")
        .and_then(|h| h.to_str().ok())
        .map(|s| s.to_string());

    let transaction = state.transaction_service
        .create_debit(
            payload.account_id,
            payload.amount,
            payload.currency,
            payload.description,
            idempotency_key,
        )
        .await?;

    state.webhook_service
        .queue_webhook(&transaction, "transaction.completed")
        .await
        .ok();

    Ok((StatusCode::CREATED, Json(TransactionResponse::from(transaction))))
}

pub async fn create_transfer(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthenticatedAccount>,
    headers: HeaderMap,
    Json(payload): Json<CreateTransferRequest>,
) -> Result<impl IntoResponse, AppError> {
    payload
        .validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;

    if payload.from_account_id != auth.account_id {
        return Err(AppError::Unauthorized(
            "Can only transfer from your own account".to_string(),
        ));
    }

    let idempotency_key = headers
        .get("Idempotency-Key")
        .and_then(|h| h.to_str().ok())
        .map(|s| s.to_string());

    let transaction = state.transaction_service
        .create_transfer(
            payload.from_account_id,
            payload.to_account_id,
            payload.amount,
            payload.currency,
            payload.description,
            idempotency_key,
        )
        .await?;

    state.webhook_service
        .queue_webhook(&transaction, "transaction.completed")
        .await
        .ok();

    Ok((StatusCode::CREATED, Json(TransactionResponse::from(transaction))))
}

pub async fn get_transaction(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthenticatedAccount>,
    Path(transaction_id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    let transaction = state.transaction_service.get_transaction(transaction_id).await?;

    // Verify the user has access to this transaction
    if transaction.from_account_id != Some(auth.account_id)
        && transaction.to_account_id != Some(auth.account_id)
    {
        return Err(AppError::Unauthorized(
            "Cannot access this transaction".to_string(),
        ));
    }

    Ok(Json(TransactionResponse::from(transaction)))
}

pub async fn list_transactions(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthenticatedAccount>,
    Query(query): Query<ListQuery>,
) -> Result<impl IntoResponse, AppError> {
    let transactions = state.transaction_service
        .list_transactions(auth.account_id, query.limit, query.offset)
        .await?;

    let total = transactions.len();
    let response = ListTransactionsResponse {
        transactions: transactions.into_iter().map(TransactionResponse::from).collect(),
        total,
        limit: query.limit,
        offset: query.offset,
    };

    Ok(Json(response))
}
