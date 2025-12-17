use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::models::{Webhook, WebhookDelivery};
use crate::utils::AppResult;

#[async_trait]
pub trait WebhookRepository: Send + Sync {
    async fn create(&self, webhook: &Webhook) -> AppResult<Webhook>;
    async fn find_by_id(&self, id: Uuid) -> AppResult<Option<Webhook>>;
    async fn find_by_account_id(&self, account_id: Uuid) -> AppResult<Vec<Webhook>>;
    async fn find_active_by_account_and_event(
        &self,
        account_id: Uuid,
        event: &str,
    ) -> AppResult<Vec<Webhook>>;
    async fn delete(&self, id: Uuid) -> AppResult<()>;

    async fn create_delivery(&self, delivery: &WebhookDelivery) -> AppResult<WebhookDelivery>;
    async fn find_pending_deliveries(&self) -> AppResult<Vec<Uuid>>;
    async fn find_delivery_by_id(&self, id: Uuid) -> AppResult<Option<WebhookDelivery>>;
    async fn update_delivery_attempt(
        &self,
        delivery_id: Uuid,
        attempt_count: i32,
        response_code: Option<i32>,
        response_body: Option<String>,
        success: bool,
    ) -> AppResult<()>;
    async fn schedule_retry(
        &self,
        delivery_id: Uuid,
        attempt_count: i32,
        next_retry_at: Option<chrono::DateTime<chrono::Utc>>,
    ) -> AppResult<()>;
    async fn mark_delivery_exhausted(&self, delivery_id: Uuid) -> AppResult<()>;
    async fn update_delivery(&self, delivery: &WebhookDelivery) -> AppResult<()>;
}
