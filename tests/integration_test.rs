use std::sync::Arc;
use sqlx::PgPool;
use uuid::Uuid;

// Integration tests for the transaction service
//
// These tests require a running PostgreSQL instance
// Set DATABASE_URL environment variable before running:
// export DATABASE_URL="postgresql://postgres:password@localhost:5432/transaction_service_test"
//
// Run with: cargo test --test integration_test -- --test-threads=1

#[cfg(test)]
mod integration_tests {
    use super::*;

    async fn setup_test_db() -> PgPool {
        let database_url = std::env::var("DATABASE_URL")
            .unwrap_or_else(|_| "postgresql://postgres:password@localhost:5432/transaction_service_test".to_string());

        let pool = sqlx::PgPool::connect(&database_url)
            .await
            .expect("Failed to connect to test database");

        // Run migrations
        sqlx::migrate!("./migrations")
            .run(&pool)
            .await
            .expect("Failed to run migrations");

        pool
    }

    async fn cleanup_test_db(pool: &PgPool) {
        // Clean up test data
        sqlx::query("TRUNCATE accounts, api_keys, transactions, webhooks, webhook_deliveries CASCADE")
            .execute(pool)
            .await
            .ok();
    }

    #[tokio::test]
    async fn test_account_creation() {
        let pool = setup_test_db().await;

        // Create account
        let account_id = Uuid::new_v4();
        let result = sqlx::query(
            "INSERT INTO accounts (id, name, email, currency, balance, created_at, updated_at)
             VALUES ($1, $2, $3, $4, $5, NOW(), NOW())"
        )
        .bind(account_id)
        .bind("Test Account")
        .bind("test@example.com")
        .bind("USD")
        .bind(0i64)
        .execute(&pool)
        .await;

        assert!(result.is_ok());

        // Verify account exists
        let row: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM accounts WHERE id = $1")
            .bind(account_id)
            .fetch_one(&pool)
            .await
            .unwrap();

        assert_eq!(row.0, 1);

        cleanup_test_db(&pool).await;
    }

    #[tokio::test]
    async fn test_transaction_atomicity() {
        let pool = setup_test_db().await;

        // Create two accounts
        let account1_id = Uuid::new_v4();
        let account2_id = Uuid::new_v4();

        for (id, name) in [(account1_id, "Account 1"), (account2_id, "Account 2")] {
            sqlx::query(
                "INSERT INTO accounts (id, name, email, currency, balance, created_at, updated_at)
                 VALUES ($1, $2, $3, $4, $5, NOW(), NOW())"
            )
            .bind(id)
            .bind(name)
            .bind(format!("{}@example.com", name.to_lowercase().replace(" ", "")))
            .bind("USD")
            .bind(100000i64) // $1000.00
            .execute(&pool)
            .await
            .unwrap();
        }

        // Perform transfer in transaction
        let mut tx = pool.begin().await.unwrap();

        let transfer_amount = 50000i64; // $500.00

        // Debit from account1
        sqlx::query(
            "UPDATE accounts SET balance = balance - $1, updated_at = NOW()
             WHERE id = $2 AND balance >= $1"
        )
        .bind(transfer_amount)
        .bind(account1_id)
        .execute(&mut *tx)
        .await
        .unwrap();

        // Credit to account2
        sqlx::query(
            "UPDATE accounts SET balance = balance + $1, updated_at = NOW()
             WHERE id = $2"
        )
        .bind(transfer_amount)
        .bind(account2_id)
        .execute(&mut *tx)
        .await
        .unwrap();

        tx.commit().await.unwrap();

        // Verify balances
        let balance1: (i64,) = sqlx::query_as("SELECT balance FROM accounts WHERE id = $1")
            .bind(account1_id)
            .fetch_one(&pool)
            .await
            .unwrap();

        let balance2: (i64,) = sqlx::query_as("SELECT balance FROM accounts WHERE id = $1")
            .bind(account2_id)
            .fetch_one(&pool)
            .await
            .unwrap();

        assert_eq!(balance1.0, 50000); // $500.00 remaining
        assert_eq!(balance2.0, 150000); // $1500.00 total

        cleanup_test_db(&pool).await;
    }

    #[tokio::test]
    async fn test_insufficient_balance_prevents_transaction() {
        let pool = setup_test_db().await;

        let account_id = Uuid::new_v4();

        // Create account with low balance
        sqlx::query(
            "INSERT INTO accounts (id, name, email, currency, balance, created_at, updated_at)
             VALUES ($1, $2, $3, $4, $5, NOW(), NOW())"
        )
        .bind(account_id)
        .bind("Test Account")
        .bind("test@example.com")
        .bind("USD")
        .bind(10000i64) // $100.00
        .execute(&pool)
        .await
        .unwrap();

        // Try to debit more than balance
        let result = sqlx::query(
            "UPDATE accounts SET balance = balance - $1, updated_at = NOW()
             WHERE id = $2 AND balance >= $1"
        )
        .bind(20000i64) // Try to debit $200.00
        .bind(account_id)
        .execute(&pool)
        .await
        .unwrap();

        // No rows should be affected
        assert_eq!(result.rows_affected(), 0);

        // Balance should remain unchanged
        let balance: (i64,) = sqlx::query_as("SELECT balance FROM accounts WHERE id = $1")
            .bind(account_id)
            .fetch_one(&pool)
            .await
            .unwrap();

        assert_eq!(balance.0, 10000);

        cleanup_test_db(&pool).await;
    }

    #[tokio::test]
    async fn test_idempotency_key_enforcement() {
        let pool = setup_test_db().await;

        let account_id = Uuid::new_v4();
        let idempotency_key = "test-idempotency-key-12345";

        // Create account
        sqlx::query(
            "INSERT INTO accounts (id, name, email, currency, balance, created_at, updated_at)
             VALUES ($1, $2, $3, $4, $5, NOW(), NOW())"
        )
        .bind(account_id)
        .bind("Test Account")
        .bind("test@example.com")
        .bind("USD")
        .bind(100000i64)
        .execute(&pool)
        .await
        .unwrap();

        // Create first transaction
        let tx1_id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO transactions (id, account_id, transaction_type, amount, currency, balance_after, idempotency_key, status, created_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, NOW())"
        )
        .bind(tx1_id)
        .bind(account_id)
        .bind("credit")
        .bind(5000i64)
        .bind("USD")
        .bind(105000i64)
        .bind(idempotency_key)
        .bind("completed")
        .execute(&pool)
        .await
        .unwrap();

        // Try to create duplicate with same idempotency key
        let tx2_id = Uuid::new_v4();
        let result = sqlx::query(
            "INSERT INTO transactions (id, account_id, transaction_type, amount, currency, balance_after, idempotency_key, status, created_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, NOW())"
        )
        .bind(tx2_id)
        .bind(account_id)
        .bind("credit")
        .bind(5000i64)
        .bind("USD")
        .bind(105000i64)
        .bind(idempotency_key)
        .bind("completed")
        .execute(&pool)
        .await;

        // Should fail due to unique constraint
        assert!(result.is_err());

        cleanup_test_db(&pool).await;
    }

    #[tokio::test]
    async fn test_webhook_creation_and_retrieval() {
        let pool = setup_test_db().await;

        let account_id = Uuid::new_v4();

        // Create account
        sqlx::query(
            "INSERT INTO accounts (id, name, email, currency, balance, created_at, updated_at)
             VALUES ($1, $2, $3, $4, $5, NOW(), NOW())"
        )
        .bind(account_id)
        .bind("Test Account")
        .bind("test@example.com")
        .bind("USD")
        .bind(0i64)
        .execute(&pool)
        .await
        .unwrap();

        // Create webhook
        let webhook_id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO webhooks (id, account_id, url, secret, events, is_active, created_at, updated_at)
             VALUES ($1, $2, $3, $4, $5, $6, NOW(), NOW())"
        )
        .bind(webhook_id)
        .bind(account_id)
        .bind("https://example.com/webhook")
        .bind("secret_key_12345")
        .bind(vec!["transaction.completed", "transaction.failed"])
        .bind(true)
        .execute(&pool)
        .await
        .unwrap();

        // Retrieve webhook
        let webhook: (String,) = sqlx::query_as("SELECT url FROM webhooks WHERE id = $1")
            .bind(webhook_id)
            .fetch_one(&pool)
            .await
            .unwrap();

        assert_eq!(webhook.0, "https://example.com/webhook");

        cleanup_test_db(&pool).await;
    }
}
