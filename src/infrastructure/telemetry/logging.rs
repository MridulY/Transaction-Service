use tracing_subscriber::{fmt, prelude::*, EnvFilter};

/// Initialize structured logging
///
/// Production-grade logging configuration:
/// - JSON format for structured logs
/// - Configurable via RUST_LOG environment variable
/// - Includes file, line number, and thread information
/// - Suitable for log aggregation systems (ELK, Datadog, etc.)
pub fn init_logging() {
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,transaction_service=debug"));

    let fmt_layer = fmt::layer()
        .with_target(true)
        .with_thread_ids(true)
        .with_line_number(true)
        .with_file(true)
        .json();

    tracing_subscriber::registry()
        .with(env_filter)
        .with(fmt_layer)
        .init();
}

/// Initialize logging with custom format
///
/// For development environments, you may prefer human-readable output
pub fn init_logging_pretty() {
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,transaction_service=debug"));

    let fmt_layer = fmt::layer()
        .with_target(true)
        .with_thread_ids(false)
        .with_line_number(true)
        .pretty();

    tracing_subscriber::registry()
        .with(env_filter)
        .with(fmt_layer)
        .init();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_logging_functions_exist() {
        // These functions should exist and have correct signatures
        // Can't actually init in tests due to global state
    }
}
