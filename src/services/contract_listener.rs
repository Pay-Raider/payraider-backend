Self::new(config, db, alert_service)pub fn from_env(
    db: Arc<Database>,
    alert_service: Arc<AlertService>,
) -> Result<Self>let listener = ContractEventListener::from_env(
    db,
    Arc::new(AlertService::default()),
).unwrap();