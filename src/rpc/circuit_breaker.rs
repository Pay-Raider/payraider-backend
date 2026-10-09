//! Circuit breaker to avoid hammering failing RPC/Horizon endpoints.
//! Uses the failsafe crate for battle-tested reliability.

use failsafe::{backoff, failure_policy, Config, StateMachine};
use std::sync::{Arc, OnceLock};
use std::time::Duration;

/// Concrete circuit breaker type using a fixed backoff and consecutive-failure policy.
pub type CircuitBreaker =
    StateMachine<failure_policy::ConsecutiveFailures<std::iter::Repeat<Duration>>, ()>;
pub type SharedCircuitBreaker = Arc<CircuitBreaker>;

fn new_breaker() -> SharedCircuitBreaker {
    let config = CircuitBreakerConfig::default();
    let backoff = backoff::constant(config.timeout_duration);
    let policy = failure_policy::consecutive_failures(config.failure_threshold, backoff);
    let cb: CircuitBreaker = Config::new().failure_policy(policy).build();
    Arc::new(cb)
}

/// Breaker for Horizon, which serves payments, ledgers, trades and accounts:
/// everything the corridor and pre-payment endpoints need.
pub fn rpc_circuit_breaker() -> SharedCircuitBreaker {
    static BREAKER: OnceLock<SharedCircuitBreaker> = OnceLock::new();
    BREAKER.get_or_init(new_breaker).clone()
}

/// Separate breaker for the Soroban RPC endpoint. A misconfigured or failing
/// RPC URL must not open the Horizon breaker and take every Horizon-backed
/// endpoint down with it.
pub fn soroban_circuit_breaker() -> SharedCircuitBreaker {
    static BREAKER: OnceLock<SharedCircuitBreaker> = OnceLock::new();
    BREAKER.get_or_init(new_breaker).clone()
}

/// Configuration for the circuit breaker.
///
/// The circuit breaker protects against cascading failures by automatically opening
/// when failures exceed a threshold, preventing further requests until recovery.
///
/// # Circuit States
///
/// 1. **Closed** - Normal operation, requests pass through
/// 2. **Open** - Failure threshold exceeded, requests are blocked
/// 3. **Half-Open** - Testing if service has recovered
///
/// # Configuration Parameters
///
/// * `failure_threshold` - Number of consecutive failures before opening
/// * `success_threshold` - Successes needed to close circuit (legacy, unused)
/// * `timeout_duration` - How long circuit stays open before testing recovery
///
/// # Example
///
/// ```rust,no_run
/// use payraider_backend::rpc::circuit_breaker::{CircuitBreakerConfig, rpc_circuit_breaker};
/// use std::time::Duration;
///
/// let config = CircuitBreakerConfig {
///     failure_threshold: 5,           // Open after 5 consecutive failures
///     success_threshold: 2,           // (unused in failsafe 1.3)
///     timeout_duration: Duration::from_secs(30), // Stay open for 30 seconds
/// };
///
/// let breaker = rpc_circuit_breaker();
/// // Use breaker for RPC calls...
/// ```
#[derive(Debug, Clone)]
pub struct CircuitBreakerConfig {
    /// Consecutive retryable failures required to trip the circuit open.
    pub failure_threshold: u32,
    /// Retained for config compatibility; not supported by failsafe 1.3.
    pub success_threshold: u32,
    /// How long the circuit stays open before attempting recovery.
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

#[cfg(test)]
mod tests {
    use super::*;
    use failsafe::CircuitBreaker as _;

    #[test]
    fn soroban_and_horizon_use_separate_breakers() {
        assert!(!Arc::ptr_eq(
            &soroban_circuit_breaker(),
            &rpc_circuit_breaker()
        ));
    }

    // Uses fresh breakers: the global Horizon breaker is shared with tests
    // that trip it deliberately, so its state is not stable under parallel runs.
    #[test]
    fn tripping_one_breaker_leaves_another_closed() {
        let soroban = new_breaker();
        let horizon = new_breaker();

        for _ in 0..CircuitBreakerConfig::default().failure_threshold {
            let _ = soroban.call(|| Err::<(), ()>(()));
        }

        assert!(
            !soroban.is_call_permitted(),
            "tripped breaker should be open"
        );
        assert!(
            horizon.is_call_permitted(),
            "other breaker must stay closed"
        );
    }
}
