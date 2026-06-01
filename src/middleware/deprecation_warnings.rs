use anyhow::Result;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::models::deprecation_warnings::DeprecationConfig;
use crate::models::network_context_middleware::NetworkContext;

#[derive(Clone)]
pub struct DeprecationWarnings {
    config: DeprecationConfig,
    state: Arc<RwLock<u64>>,
}

impl DeprecationWarnings {
    pub fn new(config: DeprecationConfig) -> Self {
        Self {
            config,
            state: Arc::new(RwLock::new(0)),
        }
    }

    pub async fn process(&self, context: &NetworkContext) -> Result<()> {
        let mut warning_count = self.state.write().await;
        if !self.config.deprecated_versions.is_empty() {
            *warning_count += 1;
            tracing::warn!(
                network = ?context.network,
                deprecated_versions = ?self.config.deprecated_versions,
                sunset_date = ?self.config.sunset_date,
                "Deprecation warning: deprecated API versions in use"
            );
        } else {
            tracing::debug!(network = ?context.network, "No deprecated versions active");
        }
        Ok(())
    }
}
