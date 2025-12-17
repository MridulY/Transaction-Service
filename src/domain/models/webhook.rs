use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use sqlx::FromRow;
use uuid::Uuid;

use super::types::WebhookDeliveryStatus;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Webhook {
    pub id: Uuid,
    pub account_id: Uuid,
    pub url: String,
    pub secret: String,
    pub events: Vec<String>,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Webhook {
    pub fn new(account_id: Uuid, url: String, secret: String, events: Vec<String>) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            account_id,
            url,
            secret,
            events,
            is_active: true,
            created_at: now,
            updated_at: now,
        }
    }

    pub fn should_notify(&self, event: &str) -> bool {
        self.is_active && self.events.contains(&event.to_string())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct WebhookDelivery {
    pub id: Uuid,
    pub webhook_id: Uuid,
    pub transaction_id: Uuid,
    pub event_type: String,
    pub payload: JsonValue,
    pub status: WebhookDeliveryStatus,
    pub attempt_count: i32,
    pub last_attempt_at: Option<DateTime<Utc>>,
    pub next_retry_at: Option<DateTime<Utc>>,
    pub response_code: Option<i32>,
    pub response_body: Option<String>,
    pub created_at: DateTime<Utc>,
}

impl WebhookDelivery {
    pub fn new(webhook_id: Uuid, transaction_id: Uuid, event_type: String, payload: JsonValue) -> Self {
        Self {
            id: Uuid::new_v4(),
            webhook_id,
            transaction_id,
            event_type,
            payload,
            status: WebhookDeliveryStatus::Pending,
            attempt_count: 0,
            last_attempt_at: None,
            next_retry_at: Some(Utc::now()),
            response_code: None,
            response_body: None,
            created_at: Utc::now(),
        }
    }

    pub fn can_retry(&self, max_attempts: u32) -> bool {
        self.status == WebhookDeliveryStatus::Pending
            && self.attempt_count < max_attempts as i32
    }

    pub fn record_attempt(&mut self, response_code: i32, response_body: String, success: bool) {
        self.attempt_count += 1;
        self.last_attempt_at = Some(Utc::now());
        self.response_code = Some(response_code);
        self.response_body = Some(response_body);

        if success {
            self.status = WebhookDeliveryStatus::Delivered;
            self.next_retry_at = None;
        }
    }

    pub fn mark_exhausted(&mut self) {
        self.status = WebhookDeliveryStatus::Exhausted;
        self.next_retry_at = None;
    }

    pub fn calculate_next_retry(&mut self) {
        let backoff_seconds = match self.attempt_count {
            0 => 0,
            1 => 60,           // 1 minute
            2 => 600,          // 10 minutes
            3 => 3600,         // 1 hour
            _ => 21600,        // 6 hours
        };

        self.next_retry_at = Some(Utc::now() + chrono::Duration::seconds(backoff_seconds));
    }
}
