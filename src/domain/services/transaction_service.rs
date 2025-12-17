use std::sync::Arc;
use uuid::Uuid;

use crate::domain::models::Transaction;
use crate::domain::repositories::TransactionRepository;
use crate::utils::{AppError, AppResult};

pub struct TransactionService {
    transaction_repo: Arc<dyn TransactionRepository>,
}

impl TransactionService {
    pub fn new(transaction_repo: Arc<dyn TransactionRepository>) -> Self {
        Self { transaction_repo }
    }

    pub async fn create_credit(
        &self,
        account_id: Uuid,
        amount: i64,
        currency: String,
        description: Option<String>,
        idempotency_key: Option<String>,
    ) -> AppResult<Transaction> {
        if let Some(ref key) = idempotency_key {
            if let Some(existing) = self.transaction_repo.find_by_idempotency_key(key).await? {
                return Ok(existing);
            }
        }

        if amount <= 0 {
            return Err(AppError::Validation("Amount must be positive".to_string()));
        }

        let transaction =
            Transaction::new_credit(account_id, amount, currency, description, idempotency_key);

        self.transaction_repo
            .execute_credit(&transaction, account_id, amount)
            .await
    }

    pub async fn create_debit(
        &self,
        account_id: Uuid,
        amount: i64,
        currency: String,
        description: Option<String>,
        idempotency_key: Option<String>,
    ) -> AppResult<Transaction> {
        if let Some(ref key) = idempotency_key {
            if let Some(existing) = self.transaction_repo.find_by_idempotency_key(key).await? {
                return Ok(existing);
            }
        }

        if amount <= 0 {
            return Err(AppError::Validation("Amount must be positive".to_string()));
        }

        let transaction =
            Transaction::new_debit(account_id, amount, currency, description, idempotency_key);

        self.transaction_repo
            .execute_debit(&transaction, account_id, amount)
            .await
    }

    pub async fn create_transfer(
        &self,
        from_account_id: Uuid,
        to_account_id: Uuid,
        amount: i64,
        currency: String,
        description: Option<String>,
        idempotency_key: Option<String>,
    ) -> AppResult<Transaction> {
        if let Some(ref key) = idempotency_key {
            if let Some(existing) = self.transaction_repo.find_by_idempotency_key(key).await? {
                return Ok(existing);
            }
        }

        if amount <= 0 {
            return Err(AppError::Validation("Amount must be positive".to_string()));
        }

        if from_account_id == to_account_id {
            return Err(AppError::Validation(
                "Cannot transfer to the same account".to_string(),
            ));
        }

        let transaction = Transaction::new_transfer(
            from_account_id,
            to_account_id,
            amount,
            currency,
            description,
            idempotency_key,
        );

        self.transaction_repo
            .execute_transfer(&transaction, from_account_id, to_account_id, amount)
            .await
    }

    pub async fn get_transaction(&self, transaction_id: Uuid) -> AppResult<Transaction> {
        self.transaction_repo
            .find_by_id(transaction_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Transaction not found".to_string()))
    }

    pub async fn list_transactions(
        &self,
        account_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> AppResult<Vec<Transaction>> {
        self.transaction_repo
            .find_by_account(account_id, limit, offset)
            .await
    }
}
