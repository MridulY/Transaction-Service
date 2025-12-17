use async_trait::async_trait;
use sqlx::PgPool;
use uuid::Uuid;

use crate::domain::models::Account;
use crate::domain::repositories::AccountRepository;
use crate::utils::AppResult;

pub struct PostgresAccountRepository {
    pool: PgPool,
}

impl PostgresAccountRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl AccountRepository for PostgresAccountRepository {
    async fn create(&self, account: &Account) -> AppResult<Account> {
        let result = sqlx::query_as::<_, Account>(
            r#"
            INSERT INTO accounts (id, business_name, email, balance, currency, status, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            RETURNING *
            "#,
        )
        .bind(account.id)
        .bind(&account.business_name)
        .bind(&account.email)
        .bind(account.balance)
        .bind(&account.currency)
        .bind(&account.status)
        .bind(account.created_at)
        .bind(account.updated_at)
        .fetch_one(&self.pool)
        .await?;

        Ok(result)
    }

    async fn find_by_id(&self, id: Uuid) -> AppResult<Option<Account>> {
        let result = sqlx::query_as::<_, Account>(
            r#"
            SELECT * FROM accounts WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(result)
    }

    async fn find_by_email(&self, email: &str) -> AppResult<Option<Account>> {
        let result = sqlx::query_as::<_, Account>(
            r#"
            SELECT * FROM accounts WHERE email = $1
            "#,
        )
        .bind(email)
        .fetch_optional(&self.pool)
        .await?;

        Ok(result)
    }

    async fn update_balance(&self, id: Uuid, new_balance: i64) -> AppResult<()> {
        sqlx::query(
            r#"
            UPDATE accounts SET balance = $1, updated_at = NOW()
            WHERE id = $2
            "#,
        )
        .bind(new_balance)
        .bind(id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn update(&self, account: &Account) -> AppResult<Account> {
        let result = sqlx::query_as::<_, Account>(
            r#"
            UPDATE accounts
            SET business_name = $1, email = $2, balance = $3, currency = $4, status = $5, updated_at = NOW()
            WHERE id = $6
            RETURNING *
            "#,
        )
        .bind(&account.business_name)
        .bind(&account.email)
        .bind(account.balance)
        .bind(&account.currency)
        .bind(&account.status)
        .bind(account.id)
        .fetch_one(&self.pool)
        .await?;

        Ok(result)
    }
}
