//! Circuit breaker to avoid hammering failing RPC/Horizon endpoints.
//! Uses the failsafe crate for battle-tested reliability.

use std::sync::{Arc, OnceLock};
use std::time::Duration;
use failsafe::{Config, breaker::CircuitBreaker, backoff::Exponential, failure_policy::ConsecutiveFailures};
pub type SharedCircuitBreaker = Arc<CircuitBreaker<Exponential, ConsecutiveFailures>>;

pub fn rpc_circuit_breaker() -> SharedCircuitBreaker {

    static BREAKER: OnceLock<SharedCircuitBreaker> = OnceLock::new();
    BREAKER.get_or_init(|| {
        let cb = Config::new()
            .failure_threshold(5)
            .success_threshold(2)
            .timeout(Duration::from_secs(30))
            .build();
        Arc::new(cb)
    }).clone()
}

// Re-export or provide a compatible Config if needed, but we'll prefer failsafe's direct Config.
#[derive(Debug, Clone)]
pub struct CircuitBreakerConfig {
    pub failure_threshold: u32,
    pub success_threshold: u32,
    pub timeout_duration: Duration,
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            failure_threshold: 5,
            success_threshold: 2,
            timeout_duration: Duration::from_secs(30),
        }
    }
}
