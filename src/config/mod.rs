// Application configuration
use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub webhook: WebhookConfig,
    pub telemetry: TelemetryConfig,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
}

#[derive(Debug, Deserialize, Clone)]
pub struct DatabaseConfig {
    pub url: String,
    pub max_connections: u32,
}

#[derive(Debug, Deserialize, Clone)]
pub struct WebhookConfig {
    pub retry_attempts: u32,
    pub timeout_seconds: u64,
}

#[derive(Debug, Deserialize, Clone)]
pub struct TelemetryConfig {
    pub enabled: bool,
    pub endpoint: Option<String>,
}
