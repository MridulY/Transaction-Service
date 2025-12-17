use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    Extension, Json,
};
use validator::Validate;

use crate::api::dto::{AccountResponse, BalanceResponse, CreateAccountRequest, CreateAccountResponse};
use crate::api::routes::AppState;
use crate::middleware::auth::AuthenticatedAccount;
use crate::utils::AppError;

pub async fn create_account(
    State(state): State<AppState>,
    Json(payload): Json<CreateAccountRequest>,
) -> Result<impl IntoResponse, AppError> {
    payload
        .validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;

    let (account, api_key) = state.account_service
        .create_account(payload.business_name, payload.email, payload.currency)
        .await?;

    let response = CreateAccountResponse {
        account: AccountResponse::from(account),
        api_key,
    };

    Ok((StatusCode::CREATED, Json(response)))
}

pub async fn get_account(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthenticatedAccount>,
) -> Result<impl IntoResponse, AppError> {
    let account = state.account_service.get_account(auth.account_id).await?;
    Ok(Json(AccountResponse::from(account)))
}

pub async fn get_account_balance(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthenticatedAccount>,
) -> Result<impl IntoResponse, AppError> {
    let account = state.account_service.get_account(auth.account_id).await?;

    let response = BalanceResponse {
        account_id: account.id,
        balance: account.balance,
        currency: account.currency.clone(),
        available_balance: account.balance,
    };

    Ok(Json(response))
}
