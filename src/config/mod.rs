use serde::Deserialize;
use std::env;

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub rate_limit: RateLimitConfig,
    pub idempotency: IdempotencyConfig,
    pub webhook: WebhookConfig,
    pub telemetry: TelemetryConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub log_level: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DatabaseConfig {
    pub url: String,
    pub max_connections: u32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RateLimitConfig {
    pub requests_per_minute: u32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct IdempotencyConfig {
    pub expiry_hours: i64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct WebhookConfig {
    pub timeout_seconds: u64,
    pub max_retries: u32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TelemetryConfig {
    pub otel_endpoint: Option<String>,
    pub service_name: String,
}

impl Config {
    pub fn from_env() -> Result<Self, anyhow::Error> {
        dotenvy::dotenv().ok();

        let server = ServerConfig {
            host: env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string()),
            port: env::var("PORT")
                .unwrap_or_else(|_| "3000".to_string())
                .parse()?,
            log_level: env::var("LOG_LEVEL").unwrap_or_else(|_| "info".to_string()),
        };

        let database = DatabaseConfig {
            url: env::var("DATABASE_URL").expect("DATABASE_URL must be set"),
            max_connections: env::var("DATABASE_MAX_CONNECTIONS")
                .unwrap_or_else(|_| "20".to_string())
                .parse()?,
        };

        let rate_limit = RateLimitConfig {
            requests_per_minute: env::var("RATE_LIMIT_PER_MINUTE")
                .unwrap_or_else(|_| "100".to_string())
                .parse()?,
        };

        let idempotency = IdempotencyConfig {
            expiry_hours: env::var("IDEMPOTENCY_KEY_EXPIRY_HOURS")
                .unwrap_or_else(|_| "24".to_string())
                .parse()?,
        };

        let webhook = WebhookConfig {
            timeout_seconds: env::var("WEBHOOK_TIMEOUT_SECONDS")
                .unwrap_or_else(|_| "30".to_string())
                .parse()?,
            max_retries: env::var("WEBHOOK_MAX_RETRIES")
                .unwrap_or_else(|_| "5".to_string())
                .parse()?,
        };

        let telemetry = TelemetryConfig {
            otel_endpoint: env::var("OTEL_EXPORTER_OTLP_ENDPOINT").ok(),
            service_name: env::var("OTEL_SERVICE_NAME")
                .unwrap_or_else(|_| "transaction-service".to_string()),
        };

        Ok(Config {
            server,
            database,
            rate_limit,
            idempotency,
            webhook,
            telemetry,
        })
    }
}
