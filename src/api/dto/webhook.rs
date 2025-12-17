use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

use crate::domain::models::Webhook;

#[derive(Debug, Serialize, Deserialize, Validate)]
pub struct CreateWebhookRequest {
    #[validate(url)]
    pub url: String,
    #[validate(length(min = 16, max = 255))]
    pub secret: String,
    #[validate(length(min = 1))]
    pub events: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WebhookResponse {
    pub id: Uuid,
    pub account_id: Uuid,
    pub url: String,
    pub events: Vec<String>,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<Webhook> for WebhookResponse {
    fn from(webhook: Webhook) -> Self {
        Self {
            id: webhook.id,
            account_id: webhook.account_id,
            url: webhook.url,
            events: webhook.events,
            is_active: webhook.is_active,
            created_at: webhook.created_at,
            updated_at: webhook.updated_at,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ListWebhooksResponse {
    pub webhooks: Vec<WebhookResponse>,
}
