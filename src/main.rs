use std::sync::Arc;
use transaction_service::{
    api::routes::create_router,
    config::Config,
    domain::{
        repositories::{
            AccountRepository, ApiKeyRepository, TransactionRepository, WebhookRepository,
        },
        services::{AccountService, TransactionService, WebhookService},
    },
    infrastructure::{
        database::{
            create_pool, run_migrations, PostgresAccountRepository, PostgresApiKeyRepository,
            PostgresTransactionRepository, PostgresWebhookRepository,
        },
        rate_limiter::RateLimiter,
        telemetry::{metrics::MetricsRecorder, tracing::init_tracing},
        webhooks::dispatcher::{spawn_webhook_worker, WebhookDispatcher},
    },
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load configuration
    let config = Config::from_env()?;

    // Initialize telemetry (tracing + metrics)
    init_tracing(&config.telemetry)?;
    let _metrics_recorder = MetricsRecorder::init()?;

    tracing::info!(
        "Starting Transaction Service v{}",
        env!("CARGO_PKG_VERSION")
    );

    // Create database connection pool
    let pool = create_pool(&config.database.url, config.database.max_connections).await?;

    tracing::info!("Connected to database");

    // Run migrations
    run_migrations(&pool).await?;
    tracing::info!("Database migrations completed");

    // Initialize repositories
    let account_repo: Arc<dyn AccountRepository> =
        Arc::new(PostgresAccountRepository::new(pool.clone()));
    let api_key_repo: Arc<dyn ApiKeyRepository> =
        Arc::new(PostgresApiKeyRepository::new(pool.clone()));
    let transaction_repo: Arc<dyn TransactionRepository> =
        Arc::new(PostgresTransactionRepository::new(pool.clone()));
    let webhook_repo: Arc<dyn WebhookRepository> =
        Arc::new(PostgresWebhookRepository::new(pool.clone()));

    // Initialize services
    let account_service = Arc::new(AccountService::new(
        account_repo.clone(),
        api_key_repo.clone(),
    ));

    let transaction_service = Arc::new(TransactionService::new(transaction_repo.clone()));

    let webhook_service = Arc::new(WebhookService::new(webhook_repo.clone()));

    // Initialize rate limiter
    let rate_limiter = Arc::new(RateLimiter::new(config.rate_limit.requests_per_minute));

    tracing::info!(
        "Rate limiter initialized: {} requests/minute",
        config.rate_limit.requests_per_minute
    );

    // Initialize webhook dispatcher and background worker
    let webhook_dispatcher = Arc::new(WebhookDispatcher::new(
        config.webhook.clone(),
        webhook_repo.clone(),
    ));

    // Spawn webhook worker (processes deliveries every 30 seconds)
    let _webhook_worker_handle = spawn_webhook_worker(webhook_dispatcher.clone(), 30);

    tracing::info!("Webhook worker started (30s interval)");

    // Create router with all services
    let app = create_router(
        account_service,
        transaction_service,
        webhook_service,
        api_key_repo,
        rate_limiter,
    );

    // Start server
    let addr = format!("{}:{}", config.server.host, config.server.port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;

    tracing::info!("Server listening on {}", addr);
    tracing::info!("Metrics available at http://{}/metrics", addr);
    tracing::info!("Health check at http://{}/health", addr);

    // Graceful shutdown handling
    let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<()>();

    // Spawn server task
    let server = tokio::spawn(async move {
        axum::serve(listener, app)
            .with_graceful_shutdown(async {
                shutdown_rx.await.ok();
            })
            .await
    });

    // Wait for shutdown signal
    tokio::select! {
        _ = tokio::signal::ctrl_c() => {
            tracing::info!("Received shutdown signal (Ctrl+C)");
        }
        result = server => {
            match result {
                Ok(Ok(())) => tracing::info!("Server stopped normally"),
                Ok(Err(e)) => tracing::error!("Server error: {}", e),
                Err(e) => tracing::error!("Server task panicked: {}", e),
            }
        }
    }

    // Trigger graceful shutdown
    shutdown_tx.send(()).ok();

    // Shutdown telemetry
    transaction_service::infrastructure::telemetry::tracing::shutdown_tracing();

    tracing::info!("Transaction Service shutdown complete");

    Ok(())
}
