use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::models::Transaction;
use crate::utils::AppResult;

#[async_trait]
pub trait TransactionRepository: Send + Sync {
    async fn create(&self, transaction: &Transaction) -> AppResult<Transaction>;
    async fn find_by_id(&self, id: Uuid) -> AppResult<Option<Transaction>>;
    async fn find_by_idempotency_key(&self, key: &str) -> AppResult<Option<Transaction>>;
    async fn find_by_account(
        &self,
        account_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> AppResult<Vec<Transaction>>;
    async fn update_status(&self, id: Uuid, transaction: &Transaction) -> AppResult<()>;
    async fn execute_transfer(
        &self,
        transaction: &Transaction,
        from_account_id: Uuid,
        to_account_id: Uuid,
        amount: i64,
    ) -> AppResult<Transaction>;
    async fn execute_credit(
        &self,
        transaction: &Transaction,
        account_id: Uuid,
        amount: i64,
    ) -> AppResult<Transaction>;
    async fn execute_debit(
        &self,
        transaction: &Transaction,
        account_id: Uuid,
        amount: i64,
    ) -> AppResult<Transaction>;
}
