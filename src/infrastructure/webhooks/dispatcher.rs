use std::sync::Arc;
use std::time::Duration;

use chrono::Utc;
use reqwest::Client;
use serde_json::json;
use tracing::{error, info, warn};
use uuid::Uuid;

use crate::config::WebhookConfig;
use crate::domain::models::webhook::{Webhook, WebhookDelivery};
use crate::domain::repositories::WebhookRepository;

use super::retry::RetryStrategy;
use super::signature::generate_signature;

/// Production-grade webhook dispatcher with retry logic
///
/// Features:
/// - Asynchronous delivery with timeout
/// - Exponential backoff retry
/// - HMAC signature for security
/// - Comprehensive error handling
/// - Metrics and tracing
pub struct WebhookDispatcher {
    client: Client,
    config: WebhookConfig,
    retry_strategy: RetryStrategy,
    webhook_repo: Arc<dyn WebhookRepository>,
}

impl WebhookDispatcher {
    pub fn new(config: WebhookConfig, webhook_repo: Arc<dyn WebhookRepository>) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(config.timeout_seconds))
            .pool_max_idle_per_host(10)
            .pool_idle_timeout(Duration::from_secs(90))
            .build()
            .expect("Failed to build HTTP client");

        let retry_strategy = RetryStrategy::new(config.max_retries);

        Self {
            client,
            config,
            retry_strategy,
            webhook_repo,
        }
    }

    /// Process pending webhook deliveries
    ///
    /// This should be called periodically by a background worker
    pub async fn process_pending_deliveries(&self) -> Result<usize, String> {
        let pending = self
            .webhook_repo
            .find_pending_deliveries()
            .await
            .map_err(|e| format!("Failed to fetch pending deliveries: {}", e))?;

        let count = pending.len();
        info!("Processing {} pending webhook deliveries", count);

        for delivery_id in pending {
            if let Err(e) = self.process_delivery(delivery_id).await {
                error!("Failed to process delivery {}: {}", delivery_id, e);
            }
        }

        Ok(count)
    }

    /// Process a single webhook delivery
    async fn process_delivery(&self, delivery_id: Uuid) -> Result<(), String> {
        let delivery = self
            .webhook_repo
            .find_delivery_by_id(delivery_id)
            .await
            .map_err(|e| format!("Failed to fetch delivery: {}", e))?
            .ok_or_else(|| "Delivery not found".to_string())?;

        let webhook = self
            .webhook_repo
            .find_by_id(delivery.webhook_id)
            .await
            .map_err(|e| format!("Failed to fetch webhook: {}", e))?
            .ok_or_else(|| "Webhook not found".to_string())?;

        // Check if webhook is still active
        if !webhook.is_active {
            warn!(
                "Skipping delivery {} - webhook {} is inactive",
                delivery_id, webhook.id
            );
            return Ok(());
        }

        // Check if we should retry
        if !self.retry_strategy.can_retry(delivery.attempt_count) {
            warn!(
                "Delivery {} exhausted after {} attempts",
                delivery_id, delivery.attempt_count
            );
            self.webhook_repo
                .mark_delivery_exhausted(delivery_id)
                .await
                .map_err(|e| format!("Failed to mark delivery exhausted: {}", e))?;
            return Ok(());
        }

        // Attempt delivery
        match self.send_webhook(&webhook, &delivery).await {
            Ok(response) => {
                info!(
                    "Webhook delivered successfully: {} -> {} (status: {})",
                    delivery_id, webhook.url, response.status_code
                );

                self.webhook_repo
                    .update_delivery_attempt(
                        delivery_id,
                        delivery.attempt_count + 1,
                        Some(response.status_code),
                        Some(response.body),
                        true,
                    )
                    .await
                    .map_err(|e| format!("Failed to update delivery: {}", e))?;

                Ok(())
            }
            Err(e) => {
                error!(
                    "Webhook delivery failed: {} -> {} - {}",
                    delivery_id, webhook.url, e
                );

                // Determine if we should retry
                let should_retry = self.should_retry(&e);

                if should_retry && self.retry_strategy.can_retry(delivery.attempt_count + 1) {
                    // Calculate next retry time
                    let next_retry = self
                        .retry_strategy
                        .next_retry_at(delivery.attempt_count + 1);

                    self.webhook_repo
                        .schedule_retry(delivery_id, delivery.attempt_count + 1, next_retry)
                        .await
                        .map_err(|e| format!("Failed to schedule retry: {}", e))?;

                    info!(
                        "Scheduled retry for delivery {} at {:?}",
                        delivery_id, next_retry
                    );
                } else {
                    // Mark as exhausted
                    self.webhook_repo
                        .mark_delivery_exhausted(delivery_id)
                        .await
                        .map_err(|e| format!("Failed to mark exhausted: {}", e))?;

                    warn!("Delivery {} marked as exhausted", delivery_id);
                }

                Err(e)
            }
        }
    }

    /// Send webhook HTTP request
    async fn send_webhook(
        &self,
        webhook: &Webhook,
        delivery: &WebhookDelivery,
    ) -> Result<WebhookResponse, String> {
        let timestamp = Utc::now().timestamp();

        // Serialize payload
        let payload_str = serde_json::to_string(&delivery.payload)
            .map_err(|e| format!("Failed to serialize payload: {}", e))?;

        // Generate signature
        let signature = generate_signature(&webhook.secret, &payload_str, timestamp)?;

        // Build request body with metadata
        let body = json!({
            "id": delivery.id,
            "event": delivery.event_type,
            "created_at": delivery.created_at,
            "data": delivery.payload,
        });

        // Send request
        let response = self
            .client
            .post(&webhook.url)
            .header("Content-Type", "application/json")
            .header("X-Webhook-Signature", signature)
            .header("X-Webhook-Timestamp", timestamp.to_string())
            .header("X-Webhook-ID", delivery.id.to_string())
            .header("X-Event-Type", &delivery.event_type)
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("HTTP request failed: {}", e))?;

        let status_code = response.status().as_u16() as i32;
        let body_text = response
            .text()
            .await
            .unwrap_or_else(|_| "Failed to read response".to_string());

        Ok(WebhookResponse {
            status_code,
            body: body_text,
        })
    }

    /// Determine if error is retryable
    fn should_retry(&self, error: &str) -> bool {
        self.retry_strategy.is_transient_error(error)
    }
}

#[derive(Debug)]
struct WebhookResponse {
    status_code: i32,
    body: String,
}

/// Background worker to process webhooks
///
/// Spawns a background task that processes pending deliveries every N seconds
pub fn spawn_webhook_worker(
    dispatcher: Arc<WebhookDispatcher>,
    interval_seconds: u64,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(interval_seconds));

        loop {
            interval.tick().await;

            match dispatcher.process_pending_deliveries().await {
                Ok(count) => {
                    if count > 0 {
                        info!("Processed {} webhook deliveries", count);
                    }
                }
                Err(e) => {
                    error!("Error processing webhook deliveries: {}", e);
                }
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dispatcher_creation() {
        // This test just ensures the types compile correctly
        // Full integration tests would require a test HTTP server
    }
}
