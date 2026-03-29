/*
Temporarily disabled due to compilation issues.
*/
use anyhow::Result;
use std::sync::Arc;
use crate::database::Database;
use crate::services::alert_service::AlertService;

#[derive(Debug, Clone, serde::Deserialize)]
pub struct ListenerConfig {
    pub rpc_url: String,
    pub contract_id: String,
    pub poll_interval_secs: u64,
    pub start_ledger: Option<u64>,
}

pub struct ContractEventListener;
impl ContractEventListener {
    pub fn new(_config: ListenerConfig, _db: Arc<Database>, _alert_service: Arc<AlertService>) -> Result<Self> {
        Ok(Self)
    }
    pub fn from_env(_db: Arc<Database>, _alert_service: Arc<AlertService>) -> Result<Self> {
        Ok(Self)
    }
    pub async fn start_listening(&mut self) -> Result<()> {
        Err(anyhow::anyhow!("Contract listener is temporarily disabled"))
    }
}
