use async_trait::async_trait;
use sqlx::PgPool;
use uuid::Uuid;

use crate::domain::models::{Webhook, WebhookDelivery};
use crate::domain::repositories::WebhookRepository;
use crate::utils::AppResult;

pub struct PostgresWebhookRepository {
    pool: PgPool,
}

impl PostgresWebhookRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl WebhookRepository for PostgresWebhookRepository {
    async fn create(&self, webhook: &Webhook) -> AppResult<Webhook> {
        let result = sqlx::query_as::<_, Webhook>(
            r#"
            INSERT INTO webhooks (id, account_id, url, secret, events, is_active, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            RETURNING *
            "#,
        )
        .bind(webhook.id)
        .bind(webhook.account_id)
        .bind(&webhook.url)
        .bind(&webhook.secret)
        .bind(&webhook.events)
        .bind(webhook.is_active)
        .bind(webhook.created_at)
        .bind(webhook.updated_at)
        .fetch_one(&self.pool)
        .await?;

        Ok(result)
    }

    async fn find_by_id(&self, id: Uuid) -> AppResult<Option<Webhook>> {
        let result = sqlx::query_as::<_, Webhook>(
            r#"
            SELECT * FROM webhooks WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(result)
    }

    async fn find_by_account_id(&self, account_id: Uuid) -> AppResult<Vec<Webhook>> {
        let result = sqlx::query_as::<_, Webhook>(
            r#"
            SELECT * FROM webhooks WHERE account_id = $1
            "#,
        )
        .bind(account_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(result)
    }

    async fn find_active_by_account_and_event(
        &self,
        account_id: Uuid,
        event: &str,
    ) -> AppResult<Vec<Webhook>> {
        let result = sqlx::query_as::<_, Webhook>(
            r#"
            SELECT * FROM webhooks
            WHERE account_id = $1 AND is_active = true AND $2 = ANY(events)
            "#,
        )
        .bind(account_id)
        .bind(event)
        .fetch_all(&self.pool)
        .await?;

        Ok(result)
    }

    async fn delete(&self, id: Uuid) -> AppResult<()> {
        sqlx::query(
            r#"
            DELETE FROM webhooks WHERE id = $1
            "#,
        )
        .bind(id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn create_delivery(&self, delivery: &WebhookDelivery) -> AppResult<WebhookDelivery> {
        let result = sqlx::query_as::<_, WebhookDelivery>(
            r#"
            INSERT INTO webhook_deliveries (
                id, webhook_id, transaction_id, event_type, payload, status,
                attempt_count, next_retry_at, created_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            RETURNING *
            "#,
        )
        .bind(delivery.id)
        .bind(delivery.webhook_id)
        .bind(delivery.transaction_id)
        .bind(&delivery.event_type)
        .bind(&delivery.payload)
        .bind(&delivery.status)
        .bind(delivery.attempt_count)
        .bind(delivery.next_retry_at)
        .bind(delivery.created_at)
        .fetch_one(&self.pool)
        .await?;

        Ok(result)
    }

    async fn find_pending_deliveries(&self) -> AppResult<Vec<Uuid>> {
        let result = sqlx::query_scalar::<_, Uuid>(
            r#"
            SELECT id FROM webhook_deliveries
            WHERE status = 'pending' AND next_retry_at <= NOW()
            ORDER BY next_retry_at ASC
            LIMIT 100
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(result)
    }

    async fn find_delivery_by_id(&self, id: Uuid) -> AppResult<Option<WebhookDelivery>> {
        let result =
            sqlx::query_as::<_, WebhookDelivery>("SELECT * FROM webhook_deliveries WHERE id = $1")
                .bind(id)
                .fetch_optional(&self.pool)
                .await?;

        Ok(result)
    }

    async fn update_delivery_attempt(
        &self,
        delivery_id: Uuid,
        attempt_count: i32,
        response_code: Option<i32>,
        response_body: Option<String>,
        success: bool,
    ) -> AppResult<()> {
        let status = if success { "delivered" } else { "pending" };

        sqlx::query(
            r#"
            UPDATE webhook_deliveries
            SET attempt_count = $1, last_attempt_at = NOW(),
                response_code = $2, response_body = $3, status = $4,
                next_retry_at = CASE WHEN $5 THEN NULL ELSE next_retry_at END
            WHERE id = $6
            "#,
        )
        .bind(attempt_count)
        .bind(response_code)
        .bind(response_body)
        .bind(status)
        .bind(success)
        .bind(delivery_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn schedule_retry(
        &self,
        delivery_id: Uuid,
        attempt_count: i32,
        next_retry_at: Option<chrono::DateTime<chrono::Utc>>,
    ) -> AppResult<()> {
        sqlx::query(
            r#"
            UPDATE webhook_deliveries
            SET attempt_count = $1, next_retry_at = $2, last_attempt_at = NOW()
            WHERE id = $3
            "#,
        )
        .bind(attempt_count)
        .bind(next_retry_at)
        .bind(delivery_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn mark_delivery_exhausted(&self, delivery_id: Uuid) -> AppResult<()> {
        sqlx::query(
            r#"
            UPDATE webhook_deliveries
            SET status = 'exhausted', next_retry_at = NULL
            WHERE id = $1
            "#,
        )
        .bind(delivery_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn update_delivery(&self, delivery: &WebhookDelivery) -> AppResult<()> {
        sqlx::query(
            r#"
            UPDATE webhook_deliveries
            SET status = $1, attempt_count = $2, last_attempt_at = $3,
                next_retry_at = $4, response_code = $5, response_body = $6
            WHERE id = $7
            "#,
        )
        .bind(&delivery.status)
        .bind(delivery.attempt_count)
        .bind(delivery.last_attempt_at)
        .bind(delivery.next_retry_at)
        .bind(delivery.response_code)
        .bind(&delivery.response_body)
        .bind(delivery.id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}
