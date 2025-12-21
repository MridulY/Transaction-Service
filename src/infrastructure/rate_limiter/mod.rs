use governor::{
    clock::DefaultClock,
    state::{InMemoryState, NotKeyed},
    Quota, RateLimiter as GovernorRateLimiter,
};
use std::collections::HashMap;
use std::num::NonZeroU32;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

type ApiKeyLimiter = Arc<GovernorRateLimiter<NotKeyed, InMemoryState, DefaultClock>>;

#[derive(Clone)]
pub struct RateLimiter {
    limiters: Arc<RwLock<HashMap<Uuid, ApiKeyLimiter>>>,
    quota: Quota,
}

impl RateLimiter {
    pub fn new(requests_per_minute: u32) -> Self {
        let quota = Quota::per_minute(
            NonZeroU32::new(requests_per_minute).expect("requests_per_minute must be > 0"),
        );

        Self {
            limiters: Arc::new(RwLock::new(HashMap::new())),
            quota,
        }
    }

    pub async fn check(&self, api_key_id: Uuid) -> bool {
        {
            let limiters = self.limiters.read().await;
            if let Some(limiter) = limiters.get(&api_key_id) {
                return limiter.check().is_ok();
            }
        }

        let mut limiters = self.limiters.write().await;
        let limiter = limiters
            .entry(api_key_id)
            .or_insert_with(|| Arc::new(GovernorRateLimiter::direct(self.quota)));

        limiter.check().is_ok()
    }

    pub async fn remaining(&self, api_key_id: Uuid) -> Option<u32> {
        let limiters = self.limiters.read().await;
        limiters
            .get(&api_key_id)
            .map(|_limiter| self.quota.burst_size().get())
    }

    pub async fn cleanup(&self) {
        let mut limiters = self.limiters.write().await;
        limiters.retain(|_key, _limiter| true);
    }

    pub async fn tracked_keys(&self) -> usize {
        let limiters = self.limiters.read().await;
        limiters.len()
    }
}

#[derive(Debug)]
pub struct RateLimitInfo {
    pub allowed: bool,
    pub limit: u32,
    pub remaining: u32,
    pub reset_after_seconds: u64,
}

impl RateLimiter {
    pub async fn check_with_info(&self, api_key_id: Uuid) -> RateLimitInfo {
        let allowed = self.check(api_key_id).await;
        let limit = self.quota.burst_size().get();
        let remaining = if allowed { limit - 1 } else { 0 };

        RateLimitInfo {
            allowed,
            limit,
            remaining,
            reset_after_seconds: 60, // 1 minute window
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_rate_limit_allows_within_quota() {
        let limiter = RateLimiter::new(10);
        let api_key_id = Uuid::new_v4();

        // Should allow first 10 requests
        for _ in 0..10 {
            assert!(limiter.check(api_key_id).await);
        }
    }

    #[tokio::test]
    async fn test_rate_limit_blocks_over_quota() {
        let limiter = RateLimiter::new(5);
        let api_key_id = Uuid::new_v4();

        // Exhaust quota
        for _ in 0..5 {
            limiter.check(api_key_id).await;
        }

        // Next request should be blocked
        assert!(!limiter.check(api_key_id).await);
    }

    #[tokio::test]
    async fn test_rate_limit_per_api_key() {
        let limiter = RateLimiter::new(5);
        let api_key_1 = Uuid::new_v4();
        let api_key_2 = Uuid::new_v4();

        // Exhaust quota for key 1
        for _ in 0..5 {
            limiter.check(api_key_1).await;
        }

        // Key 1 should be blocked
        assert!(!limiter.check(api_key_1).await);

        // Key 2 should still be allowed
        assert!(limiter.check(api_key_2).await);
    }

    #[tokio::test]
    async fn test_rate_limit_info() {
        let limiter = RateLimiter::new(10);
        let api_key_id = Uuid::new_v4();

        let info = limiter.check_with_info(api_key_id).await;

        assert!(info.allowed);
        assert_eq!(info.limit, 10);
        assert_eq!(info.reset_after_seconds, 60);
    }

    #[tokio::test]
    async fn test_tracked_keys() {
        let limiter = RateLimiter::new(10);
        let key1 = Uuid::new_v4();
        let key2 = Uuid::new_v4();

        limiter.check(key1).await;
        limiter.check(key2).await;

        assert_eq!(limiter.tracked_keys().await, 2);
    }
}
