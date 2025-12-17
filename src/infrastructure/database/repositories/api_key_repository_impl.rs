use async_trait::async_trait;
use sqlx::PgPool;
use uuid::Uuid;

use crate::domain::models::ApiKey;
use crate::domain::repositories::ApiKeyRepository;
use crate::utils::AppResult;

pub struct PostgresApiKeyRepository {
    pool: PgPool,
}

impl PostgresApiKeyRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl ApiKeyRepository for PostgresApiKeyRepository {
    async fn create(&self, api_key: &ApiKey) -> AppResult<ApiKey> {
        let result = sqlx::query_as::<_, ApiKey>(
            r#"
            INSERT INTO api_keys (id, account_id, key_hash, name, is_active, created_at)
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING *
            "#,
        )
        .bind(api_key.id)
        .bind(api_key.account_id)
        .bind(&api_key.key_hash)
        .bind(&api_key.name)
        .bind(api_key.is_active)
        .bind(api_key.created_at)
        .fetch_one(&self.pool)
        .await?;

        Ok(result)
    }

    async fn find_by_key_hash(&self, key_hash: &str) -> AppResult<Option<ApiKey>> {
        let result = sqlx::query_as::<_, ApiKey>(
            r#"
            SELECT * FROM api_keys WHERE key_hash = $1
            "#,
        )
        .bind(key_hash)
        .fetch_optional(&self.pool)
        .await?;

        Ok(result)
    }

    async fn find_by_account_id(&self, account_id: Uuid) -> AppResult<Vec<ApiKey>> {
        let result = sqlx::query_as::<_, ApiKey>(
            r#"
            SELECT * FROM api_keys WHERE account_id = $1
            "#,
        )
        .bind(account_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(result)
    }

    async fn get_all_active(&self) -> AppResult<Vec<ApiKey>> {
        let result = sqlx::query_as::<_, ApiKey>(
            r#"
            SELECT * FROM api_keys WHERE is_active = TRUE
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(result)
    }

    async fn update_last_used(&self, id: Uuid) -> AppResult<()> {
        sqlx::query(
            r#"
            UPDATE api_keys SET last_used_at = NOW()
            WHERE id = $1
            "#,
        )
        .bind(id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn deactivate(&self, id: Uuid) -> AppResult<()> {
        sqlx::query(
            r#"
            UPDATE api_keys SET is_active = FALSE
            WHERE id = $1
            "#,
        )
        .bind(id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}
