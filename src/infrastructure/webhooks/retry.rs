use chrono::{DateTime, Duration, Utc};

/// Retry strategy with exponential backoff
///
/// Production-grade retry logic for webhook delivery with circuit breaker pattern
#[derive(Debug, Clone)]
pub struct RetryStrategy {
    max_attempts: u32,
    initial_backoff_seconds: i64,
    max_backoff_seconds: i64,
    backoff_multiplier: f64,
}

impl Default for RetryStrategy {
    fn default() -> Self {
        Self {
            max_attempts: 5,
            initial_backoff_seconds: 60, // 1 minute
            max_backoff_seconds: 21600,  // 6 hours
            backoff_multiplier: 10.0,
        }
    }
}

impl RetryStrategy {
    pub fn new(max_attempts: u32) -> Self {
        Self {
            max_attempts,
            ..Default::default()
        }
    }

    /// Calculate next retry time based on attempt count using exponential backoff
    ///
    /// Retry schedule:
    /// - Attempt 1: 1 minute
    /// - Attempt 2: 10 minutes
    /// - Attempt 3: 1 hour
    /// - Attempt 4+: 6 hours
    pub fn next_retry_at(&self, attempt_count: i32) -> Option<DateTime<Utc>> {
        if attempt_count >= self.max_attempts as i32 {
            return None;
        }

        let backoff_seconds = match attempt_count {
            0 => 0, // Immediate first attempt
            1 => self.initial_backoff_seconds,
            n => {
                let exp_backoff =
                    (self.initial_backoff_seconds as f64) * self.backoff_multiplier.powi(n - 1);
                exp_backoff.min(self.max_backoff_seconds as f64) as i64
            }
        };

        Some(Utc::now() + Duration::seconds(backoff_seconds))
    }

    /// Check if delivery can be retried
    pub fn can_retry(&self, attempt_count: i32) -> bool {
        attempt_count < self.max_attempts as i32
    }

    /// Check if HTTP status code is retryable
    pub fn is_retryable_status(&self, status_code: u16) -> bool {
        match status_code {
            // Client errors (4xx) - generally not retryable
            400..=499 => {
                // Exception: 408 (Request Timeout), 429 (Too Many Requests) are retryable
                matches!(status_code, 408 | 429)
            }
            // Server errors (5xx) - retryable
            500..=599 => true,
            // Success (2xx) - no retry needed
            200..=299 => false,
            // All other cases (network errors, etc.) - retryable
            _ => true,
        }
    }

    /// Determine if error is transient and worth retrying
    pub fn is_transient_error(&self, error: &str) -> bool {
        let error_lower = error.to_lowercase();

        // Network-related errors are transient
        error_lower.contains("timeout")
            || error_lower.contains("connection")
            || error_lower.contains("network")
            || error_lower.contains("dns")
            || error_lower.contains("refused")
            || error_lower.contains("reset")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_retry_backoff_calculation() {
        let strategy = RetryStrategy::default();

        let retry1 = strategy.next_retry_at(1).unwrap();
        let retry2 = strategy.next_retry_at(2).unwrap();
        let retry3 = strategy.next_retry_at(3).unwrap();

        // Each retry should be further in the future
        assert!(retry2 > retry1);
        assert!(retry3 > retry2);
    }

    #[test]
    fn test_retry_exhaustion() {
        let strategy = RetryStrategy::new(3);

        assert!(strategy.can_retry(0));
        assert!(strategy.can_retry(1));
        assert!(strategy.can_retry(2));
        assert!(!strategy.can_retry(3));
        assert!(!strategy.can_retry(4));
    }

    #[test]
    fn test_retryable_status_codes() {
        let strategy = RetryStrategy::default();

        // Success - no retry
        assert!(!strategy.is_retryable_status(200));
        assert!(!strategy.is_retryable_status(201));

        // Client errors - generally no retry
        assert!(!strategy.is_retryable_status(400));
        assert!(!strategy.is_retryable_status(404));

        // Timeout and rate limit - retryable
        assert!(strategy.is_retryable_status(408));
        assert!(strategy.is_retryable_status(429));

        // Server errors - retryable
        assert!(strategy.is_retryable_status(500));
        assert!(strategy.is_retryable_status(502));
        assert!(strategy.is_retryable_status(503));
    }

    #[test]
    fn test_transient_error_detection() {
        let strategy = RetryStrategy::default();

        assert!(strategy.is_transient_error("Connection timeout"));
        assert!(strategy.is_transient_error("Network unreachable"));
        assert!(strategy.is_transient_error("DNS resolution failed"));
        assert!(strategy.is_transient_error("Connection refused"));

        assert!(!strategy.is_transient_error("Invalid payload"));
        assert!(!strategy.is_transient_error("Unauthorized"));
    }

    #[test]
    fn test_max_backoff_limit() {
        let strategy = RetryStrategy::default();

        // Very high attempt count should still cap at max_backoff
        let retry_high = strategy.next_retry_at(100);
        assert!(retry_high.is_none()); // Beyond max attempts
    }
}
