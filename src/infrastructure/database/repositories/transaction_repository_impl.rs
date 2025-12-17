use async_trait::async_trait;
use sqlx::PgPool;
use uuid::Uuid;

use crate::domain::models::{Account, Transaction};
use crate::domain::repositories::TransactionRepository;
use crate::utils::{AppError, AppResult};

pub struct PostgresTransactionRepository {
    pool: PgPool,
}

impl PostgresTransactionRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl TransactionRepository for PostgresTransactionRepository {
    async fn create(&self, transaction: &Transaction) -> AppResult<Transaction> {
        let result = sqlx::query_as::<_, Transaction>(
            r#"
            INSERT INTO transactions (
                id, idempotency_key, transaction_type, from_account_id, to_account_id,
                amount, currency, status, description, metadata, created_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
            RETURNING *
            "#,
        )
        .bind(transaction.id)
        .bind(&transaction.idempotency_key)
        .bind(&transaction.transaction_type)
        .bind(transaction.from_account_id)
        .bind(transaction.to_account_id)
        .bind(transaction.amount)
        .bind(&transaction.currency)
        .bind(&transaction.status)
        .bind(&transaction.description)
        .bind(&transaction.metadata)
        .bind(transaction.created_at)
        .fetch_one(&self.pool)
        .await?;

        Ok(result)
    }

    async fn find_by_id(&self, id: Uuid) -> AppResult<Option<Transaction>> {
        let result = sqlx::query_as::<_, Transaction>(
            r#"
            SELECT * FROM transactions WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(result)
    }

    async fn find_by_idempotency_key(&self, key: &str) -> AppResult<Option<Transaction>> {
        let result = sqlx::query_as::<_, Transaction>(
            r#"
            SELECT * FROM transactions WHERE idempotency_key = $1
            "#,
        )
        .bind(key)
        .fetch_optional(&self.pool)
        .await?;

        Ok(result)
    }

    async fn find_by_account(
        &self,
        account_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> AppResult<Vec<Transaction>> {
        let result = sqlx::query_as::<_, Transaction>(
            r#"
            SELECT * FROM transactions
            WHERE from_account_id = $1 OR to_account_id = $1
            ORDER BY created_at DESC
            LIMIT $2 OFFSET $3
            "#,
        )
        .bind(account_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        Ok(result)
    }

    async fn update_status(&self, id: Uuid, transaction: &Transaction) -> AppResult<()> {
        sqlx::query(
            r#"
            UPDATE transactions
            SET status = $1, completed_at = $2
            WHERE id = $3
            "#,
        )
        .bind(&transaction.status)
        .bind(transaction.completed_at)
        .bind(id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn execute_transfer(
        &self,
        transaction: &Transaction,
        from_account_id: Uuid,
        to_account_id: Uuid,
        amount: i64,
    ) -> AppResult<Transaction> {
        let mut tx = self.pool.begin().await?;

        let from_account = sqlx::query_as::<_, Account>(
            r#"
            SELECT * FROM accounts WHERE id = $1 FOR UPDATE
            "#,
        )
        .bind(from_account_id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| AppError::NotFound("Source account not found".to_string()))?;

        if !from_account.can_debit(amount) {
            return Err(AppError::InsufficientFunds {
                required: amount,
                available: from_account.balance,
            });
        }

        let to_account = sqlx::query_as::<_, Account>(
            r#"
            SELECT * FROM accounts WHERE id = $1 FOR UPDATE
            "#,
        )
        .bind(to_account_id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| AppError::NotFound("Destination account not found".to_string()))?;

        if !to_account.is_active() {
            return Err(AppError::BadRequest(
                "Destination account is not active".to_string(),
            ));
        }

        sqlx::query(
            r#"
            UPDATE accounts SET balance = balance - $1, updated_at = NOW()
            WHERE id = $2
            "#,
        )
        .bind(amount)
        .bind(from_account_id)
        .execute(&mut *tx)
        .await?;

        sqlx::query(
            r#"
            UPDATE accounts SET balance = balance + $1, updated_at = NOW()
            WHERE id = $2
            "#,
        )
        .bind(amount)
        .bind(to_account_id)
        .execute(&mut *tx)
        .await?;

        let mut completed_transaction = transaction.clone();
        completed_transaction.mark_completed();

        let result = sqlx::query_as::<_, Transaction>(
            r#"
            INSERT INTO transactions (
                id, idempotency_key, transaction_type, from_account_id, to_account_id,
                amount, currency, status, description, metadata, created_at, completed_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
            RETURNING *
            "#,
        )
        .bind(completed_transaction.id)
        .bind(&completed_transaction.idempotency_key)
        .bind(&completed_transaction.transaction_type)
        .bind(completed_transaction.from_account_id)
        .bind(completed_transaction.to_account_id)
        .bind(completed_transaction.amount)
        .bind(&completed_transaction.currency)
        .bind(&completed_transaction.status)
        .bind(&completed_transaction.description)
        .bind(&completed_transaction.metadata)
        .bind(completed_transaction.created_at)
        .bind(completed_transaction.completed_at)
        .fetch_one(&mut *tx)
        .await?;

        tx.commit().await?;

        Ok(result)
    }

    async fn execute_credit(
        &self,
        transaction: &Transaction,
        account_id: Uuid,
        amount: i64,
    ) -> AppResult<Transaction> {
        let mut tx = self.pool.begin().await?;

        let account = sqlx::query_as::<_, Account>(
            r#"
            SELECT * FROM accounts WHERE id = $1 FOR UPDATE
            "#,
        )
        .bind(account_id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| AppError::NotFound("Account not found".to_string()))?;

        if !account.is_active() {
            return Err(AppError::BadRequest("Account is not active".to_string()));
        }

        sqlx::query(
            r#"
            UPDATE accounts SET balance = balance + $1, updated_at = NOW()
            WHERE id = $2
            "#,
        )
        .bind(amount)
        .bind(account_id)
        .execute(&mut *tx)
        .await?;

        let mut completed_transaction = transaction.clone();
        completed_transaction.mark_completed();

        let result = sqlx::query_as::<_, Transaction>(
            r#"
            INSERT INTO transactions (
                id, idempotency_key, transaction_type, from_account_id, to_account_id,
                amount, currency, status, description, metadata, created_at, completed_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
            RETURNING *
            "#,
        )
        .bind(completed_transaction.id)
        .bind(&completed_transaction.idempotency_key)
        .bind(&completed_transaction.transaction_type)
        .bind(completed_transaction.from_account_id)
        .bind(completed_transaction.to_account_id)
        .bind(completed_transaction.amount)
        .bind(&completed_transaction.currency)
        .bind(&completed_transaction.status)
        .bind(&completed_transaction.description)
        .bind(&completed_transaction.metadata)
        .bind(completed_transaction.created_at)
        .bind(completed_transaction.completed_at)
        .fetch_one(&mut *tx)
        .await?;

        tx.commit().await?;

        Ok(result)
    }

    async fn execute_debit(
        &self,
        transaction: &Transaction,
        account_id: Uuid,
        amount: i64,
    ) -> AppResult<Transaction> {
        let mut tx = self.pool.begin().await?;

        let account = sqlx::query_as::<_, Account>(
            r#"
            SELECT * FROM accounts WHERE id = $1 FOR UPDATE
            "#,
        )
        .bind(account_id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| AppError::NotFound("Account not found".to_string()))?;

        if !account.can_debit(amount) {
            return Err(AppError::InsufficientFunds {
                required: amount,
                available: account.balance,
            });
        }

        sqlx::query(
            r#"
            UPDATE accounts SET balance = balance - $1, updated_at = NOW()
            WHERE id = $2
            "#,
        )
        .bind(amount)
        .bind(account_id)
        .execute(&mut *tx)
        .await?;

        let mut completed_transaction = transaction.clone();
        completed_transaction.mark_completed();

        let result = sqlx::query_as::<_, Transaction>(
            r#"
            INSERT INTO transactions (
                id, idempotency_key, transaction_type, from_account_id, to_account_id,
                amount, currency, status, description, metadata, created_at, completed_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
            RETURNING *
            "#,
        )
        .bind(completed_transaction.id)
        .bind(&completed_transaction.idempotency_key)
        .bind(&completed_transaction.transaction_type)
        .bind(completed_transaction.from_account_id)
        .bind(completed_transaction.to_account_id)
        .bind(completed_transaction.amount)
        .bind(&completed_transaction.currency)
        .bind(&completed_transaction.status)
        .bind(&completed_transaction.description)
        .bind(&completed_transaction.metadata)
        .bind(completed_transaction.created_at)
        .bind(completed_transaction.completed_at)
        .fetch_one(&mut *tx)
        .await?;

        tx.commit().await?;

        Ok(result)
    }
}
