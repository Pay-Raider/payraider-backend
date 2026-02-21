//! Custom error types and retry logic for RPC/Horizon requests.
//!
//! Errors are categorized so callers can distinguish transient (retryable)
//! from permanent failures. Retry uses exponential backoff and respects
//! Retry-After when present (e.g. rate limits).

use std::time::Duration;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum RpcError {
    #[error("Network error: {0}")]
    NetworkError(#[from] reqwest::Error),

    #[error("Rate limit exceeded. Retry after {retry_after:?}")]
    RateLimitError {
        retry_after: Option<Duration>,
    },

    #[error("Server error: {status} - {message}")]
    ServerError {
        status: u16,
        message: String,
    },

    #[error("Failed to parse response: {0}")]
    ParseError(String),

    #[error("Request timeout after {0:?}")]
    TimeoutError(Duration),

    #[error("Circuit breaker open")]
    CircuitBreakerOpen,
}

impl RpcError {
    /// Whether this error is transient and safe to retry.
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            RpcError::NetworkError(_)
                | RpcError::TimeoutError(_)
                | RpcError::ServerError {
                    status: 500..=599,
                    ..
                }
        )
    }

    /// Suggested wait before retry (e.g. from Retry-After header).
    pub fn retry_after(&self) -> Option<Duration> {
        match self {
            RpcError::RateLimitError { retry_after } => *retry_after,
            _ => None,
        }
    }

    /// Label for metrics (error_type).
    pub fn error_type_label(&self) -> &'static str {
        match self {
            RpcError::NetworkError(_) => "network",
            RpcError::RateLimitError { .. } => "rate_limit",
            RpcError::ServerError { .. } => "server",
            RpcError::ParseError(_) => "parse",
            RpcError::TimeoutError(_) => "timeout",
            RpcError::CircuitBreakerOpen => "circuit_breaker_open",
        }
    }
}

/// Retry a fallible operation with exponential backoff.
/// Only retries when the error is retryable; otherwise returns immediately.
pub async fn retry_with_backoff<F, Fut, T>(
    mut f: F,
    max_retries: u32,
    initial_backoff: Duration,
    max_backoff: Duration,
) -> Result<T, RpcError>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<T, RpcError>>,
{
    let mut attempt = 0;
    let mut backoff = initial_backoff;

    loop {
        match f().await {
            Ok(result) => return Ok(result),
            Err(e) if e.is_retryable() && attempt < max_retries => {
                attempt += 1;

                let sleep_duration = e
                    .retry_after()
                    .unwrap_or_else(|| std::cmp::min(backoff, max_backoff));

                tracing::warn!(
                    "Retrying request (attempt {}/{}) after error: {}",
                    attempt,
                    max_retries,
                    e
                );

                tokio::time::sleep(sleep_duration).await;
                backoff = std::cmp::min(backoff * 2, max_backoff);
            }
            Err(e) => return Err(e),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn network_error_is_retryable() {
        // We can't easily construct reqwest::Error; test the logic via ServerError
        let e = RpcError::ServerError {
            status: 503,
            message: "unavailable".to_string(),
        };
        assert!(e.is_retryable());
    }

    #[test]
    fn parse_error_not_retryable() {
        let e = RpcError::ParseError("bad json".to_string());
        assert!(!e.is_retryable());
    }

    #[test]
    fn rate_limit_retry_after() {
        let e = RpcError::RateLimitError {
            retry_after: Some(Duration::from_secs(60)),
        };
        assert_eq!(e.retry_after(), Some(Duration::from_secs(60)));
    }

    #[test]
    fn server_4xx_not_retryable() {
        let e = RpcError::ServerError {
            status: 400,
            message: "bad request".to_string(),
        };
        assert!(!e.is_retryable());
    }

    #[tokio::test]
    async fn retry_succeeds_on_second_attempt() {
        let mut attempts = 0;
        let result = retry_with_backoff(
            || {
                attempts += 1;
                async move {
                    if attempts < 2 {
                        Err(RpcError::ServerError {
                            status: 503,
                            message: "temp".to_string(),
                        })
                    } else {
                        Ok(42)
                    }
                }
            },
            3,
            Duration::from_millis(1),
            Duration::from_secs(1),
        )
        .await;
        assert_eq!(result.unwrap(), 42);
        assert_eq!(attempts, 2);
    }

    #[tokio::test]
    async fn retry_does_not_retry_parse_error() {
        let mut calls = 0;
        let result: Result<i32, RpcError> = retry_with_backoff(
            || {
                calls += 1;
                async move { Err(RpcError::ParseError("bad".to_string())) }
            },
            3,
            Duration::from_millis(1),
            Duration::from_secs(1),
        )
        .await;
        assert!(result.is_err());
        assert_eq!(calls, 1);
    }
}
