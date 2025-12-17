use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

use crate::domain::models::{Account, AccountStatus};

#[derive(Debug, Serialize, Deserialize, Validate)]
pub struct CreateAccountRequest {
    #[validate(length(min = 1, max = 255))]
    pub business_name: String,
    #[validate(email)]
    pub email: String,
    #[validate(length(equal = 3))]
    pub currency: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AccountResponse {
    pub id: Uuid,
    pub business_name: String,
    pub email: String,
    pub balance: i64,
    pub currency: String,
    pub status: AccountStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<Account> for AccountResponse {
    fn from(account: Account) -> Self {
        Self {
            id: account.id,
            business_name: account.business_name,
            email: account.email,
            balance: account.balance,
            currency: account.currency,
            status: account.status,
            created_at: account.created_at,
            updated_at: account.updated_at,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateAccountResponse {
    pub account: AccountResponse,
    pub api_key: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BalanceResponse {
    pub account_id: Uuid,
    pub balance: i64,
    pub currency: String,
    pub available_balance: i64,
}
