use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use sqlx::FromRow;
use uuid::Uuid;

use super::types::{TransactionStatus, TransactionType};

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Transaction {
    pub id: Uuid,
    pub idempotency_key: Option<String>,
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

impl Transaction {
    pub fn new_credit(
        to_account_id: Uuid,
        amount: i64,
        currency: String,
        description: Option<String>,
        idempotency_key: Option<String>,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            idempotency_key,
            transaction_type: TransactionType::Credit,
            from_account_id: None,
            to_account_id: Some(to_account_id),
            amount,
            currency,
            status: TransactionStatus::Pending,
            description,
            metadata: None,
            created_at: Utc::now(),
            completed_at: None,
        }
    }

    pub fn new_debit(
        from_account_id: Uuid,
        amount: i64,
        currency: String,
        description: Option<String>,
        idempotency_key: Option<String>,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            idempotency_key,
            transaction_type: TransactionType::Debit,
            from_account_id: Some(from_account_id),
            to_account_id: None,
            amount,
            currency,
            status: TransactionStatus::Pending,
            description,
            metadata: None,
            created_at: Utc::now(),
            completed_at: None,
        }
    }

    pub fn new_transfer(
        from_account_id: Uuid,
        to_account_id: Uuid,
        amount: i64,
        currency: String,
        description: Option<String>,
        idempotency_key: Option<String>,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            idempotency_key,
            transaction_type: TransactionType::Transfer,
            from_account_id: Some(from_account_id),
            to_account_id: Some(to_account_id),
            amount,
            currency,
            status: TransactionStatus::Pending,
            description,
            metadata: None,
            created_at: Utc::now(),
            completed_at: None,
        }
    }

    pub fn mark_completed(&mut self) {
        self.status = TransactionStatus::Completed;
        self.completed_at = Some(Utc::now());
    }

    pub fn mark_failed(&mut self) {
        self.status = TransactionStatus::Failed;
        self.completed_at = Some(Utc::now());
    }
}
