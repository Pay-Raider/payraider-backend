//! # RPC and Horizon client with error handling
//!
//! ## Error handling strategy
//!
//! - **Error types** (`rpc::error::RpcError`): Errors are categorized as network, rate limit,
//!   server (5xx), parse, timeout, or circuit breaker open. Use `RpcError::is_retryable()` and
//!   `RpcError::retry_after()` for behavior.
//! - **Retry**: Transient errors (network, timeout, 5xx) are retried with exponential backoff
//!   (configurable via `RPC_MAX_RETRIES`, `RPC_INITIAL_BACKOFF_MS`, `RPC_MAX_BACKOFF_MS`).
//!   Rate limits use `Retry-After` when present. Parse and 4xx errors are not retried.
//! - **Circuit breaker**: After a configurable number of failures (`RPC_CIRCUIT_BREAKER_*`),
//!   the circuit opens and requests fail fast. After a timeout it moves to half-open and
//!   a few successes close it again.
//! - **Metrics**: `rpc_errors_total` (by error_type, endpoint) and `circuit_breaker_state`
//!   (0=closed, 1=open, 2=half-open) are exposed for alerting.
//! - **API**: Handlers convert RPC failures to HTTP 500 with a message; list corridors
//!   fails the request on payment fetch failure; list anchors falls back to DB when RPC fails.

pub mod circuit_breaker;
mod config;
pub mod error;
mod metrics;
pub mod stellar;

pub use circuit_breaker::{CircuitBreaker, CircuitBreakerConfig};
pub use error::{retry_with_backoff, RpcError};
pub use stellar::{
    Asset, FeeBumpTransactionInfo, GetLedgersResult, HealthResponse, HorizonAsset, HorizonEffect,
    HorizonLiquidityPool, HorizonOperation, HorizonPoolReserve, HorizonTransaction,
    InnerTransaction, LedgerInfo, OrderBook, OrderBookEntry, Payment, Price, RpcLedger,
    StellarRpcClient, Trade,
};
