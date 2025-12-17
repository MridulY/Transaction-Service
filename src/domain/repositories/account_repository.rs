use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::models::Account;
use crate::utils::AppResult;

#[async_trait]
pub trait AccountRepository: Send + Sync {
    async fn create(&self, account: &Account) -> AppResult<Account>;
    async fn find_by_id(&self, id: Uuid) -> AppResult<Option<Account>>;
    async fn find_by_email(&self, email: &str) -> AppResult<Option<Account>>;
    async fn update_balance(&self, id: Uuid, new_balance: i64) -> AppResult<()>;
    async fn update(&self, account: &Account) -> AppResult<Account>;
}
