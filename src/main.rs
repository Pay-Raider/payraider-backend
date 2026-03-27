use tower_http::cors::{AllowOrigin, CorsLayer};let app_state = AppState::new(
    db.clone(),
    cache.clone(),
    ws_state,
    ingestion,
    rpc_client.clone(),
);