use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use uuid::Uuid;
use validator::Validate;

use crate::domain::models::{Transaction, TransactionStatus, TransactionType};

#[derive(Debug, Serialize, Deserialize, Validate)]
pub struct CreateCreditRequest {
    pub account_id: Uuid,
    #[validate(range(min = 1))]
    pub amount: i64,
    #[validate(length(equal = 3))]
    pub currency: String,
    pub description: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Validate)]
pub struct CreateDebitRequest {
    pub account_id: Uuid,
    #[validate(range(min = 1))]
    pub amount: i64,
    #[validate(length(equal = 3))]
    pub currency: String,
    pub description: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Validate)]
pub struct CreateTransferRequest {
    pub from_account_id: Uuid,
    pub to_account_id: Uuid,
    #[validate(range(min = 1))]
    pub amount: i64,
    #[validate(length(equal = 3))]
    pub currency: String,
    pub description: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TransactionResponse {
    pub id: Uuid,
    #[serde(rename = "type")]
    pub transaction_type: TransactionType,
    pub from_account_id: Option<Uuid>,
    pub to_account_id: Option<Uuid>,
    pub amount: i64,
    pub currency: String,
    pub status: TransactionStatus,
    pub description: Option<String>,
    pub metadata: Option<JsonValue>,
    pub created_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}

impl From<Transaction> for TransactionResponse {
    fn from(transaction: Transaction) -> Self {
        Self {
            id: transaction.id,
            transaction_type: transaction.transaction_type,
            from_account_id: transaction.from_account_id,
            to_account_id: transaction.to_account_id,
            amount: transaction.amount,
            currency: transaction.currency,
            status: transaction.status,
            description: transaction.description,
            metadata: transaction.metadata,
            created_at: transaction.created_at,
            completed_at: transaction.completed_at,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ListTransactionsResponse {
    pub transactions: Vec<TransactionResponse>,
    pub total: usize,
    pub limit: i64,
    pub offset: i64,
}
