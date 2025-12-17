use std::sync::Arc;
use uuid::Uuid;

use crate::domain::models::{Transaction, Webhook, WebhookDelivery};
use crate::domain::repositories::WebhookRepository;
use crate::utils::{AppError, AppResult};

pub struct WebhookService {
    webhook_repo: Arc<dyn WebhookRepository>,
}

impl WebhookService {
    pub fn new(webhook_repo: Arc<dyn WebhookRepository>) -> Self {
        Self { webhook_repo }
    }

    pub async fn create_webhook(
        &self,
        account_id: Uuid,
        url: String,
        secret: String,
        events: Vec<String>,
    ) -> AppResult<Webhook> {
        if !url.starts_with("https://") && !url.starts_with("http://") {
            return Err(AppError::Validation(
                "Webhook URL must be a valid HTTP/HTTPS URL".to_string(),
            ));
        }

        let webhook = Webhook::new(account_id, url, secret, events);
        self.webhook_repo.create(&webhook).await
    }

    pub async fn get_webhook(&self, webhook_id: Uuid) -> AppResult<Webhook> {
        self.webhook_repo
            .find_by_id(webhook_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Webhook not found".to_string()))
    }

    pub async fn list_webhooks(&self, account_id: Uuid) -> AppResult<Vec<Webhook>> {
        self.webhook_repo.find_by_account_id(account_id).await
    }

    pub async fn delete_webhook(&self, webhook_id: Uuid) -> AppResult<()> {
        self.webhook_repo.delete(webhook_id).await
    }

    pub async fn queue_webhook(
        &self,
        transaction: &Transaction,
        event_type: &str,
    ) -> AppResult<()> {
        let account_id = transaction
            .from_account_id
            .or(transaction.to_account_id)
            .ok_or_else(|| {
                AppError::Internal("Transaction has no associated account".to_string())
            })?;

        let webhooks = self
            .webhook_repo
            .find_active_by_account_and_event(account_id, event_type)
            .await?;

        for webhook in webhooks {
            let payload = serde_json::json!({
                "id": Uuid::new_v4(),
                "event": event_type,
                "created_at": chrono::Utc::now(),
                "data": {
                    "transaction": transaction
                }
            });

            let delivery =
                WebhookDelivery::new(webhook.id, transaction.id, event_type.to_string(), payload);

            self.webhook_repo.create_delivery(&delivery).await?;
        }

        Ok(())
    }
}
