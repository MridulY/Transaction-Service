use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::models::ApiKey;
use crate::utils::AppResult;

#[async_trait]
pub trait ApiKeyRepository: Send + Sync {
    async fn create(&self, api_key: &ApiKey) -> AppResult<ApiKey>;
    async fn find_by_key_hash(&self, key_hash: &str) -> AppResult<Option<ApiKey>>;
    async fn find_by_account_id(&self, account_id: Uuid) -> AppResult<Vec<ApiKey>>;
    async fn get_all_active(&self) -> AppResult<Vec<ApiKey>>;
    async fn update_last_used(&self, id: Uuid) -> AppResult<()>;
    async fn deactivate(&self, id: Uuid) -> AppResult<()>;
}
