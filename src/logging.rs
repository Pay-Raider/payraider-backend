//! Simple stdout-only logging. For production with log rotation (daily rotation,
//! bounded retention), use `observability::tracing::init_tracing` and set `LOG_DIR`.

use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

/// Initializes logging to stdout only. No file output or rotation.
/// See `observability::tracing::init_tracing` for rotating file logs when `LOG_DIR` is set.
pub fn init_logging() {
    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    let fmt_layer = tracing_subscriber::fmt::layer()
        .json()
        .with_target(true)
        .with_level(true);

    tracing_subscriber::registry()
        .with(env_filter)
        .with(fmt_layer)
        .init();
}
