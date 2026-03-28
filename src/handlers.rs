use axum::{extract::State, response::IntoResponse, Json};
use chrono::{DateTime, Utc};
use serde::Serialize;
use std::sync::Arc;

use crate::cache::CacheManager;
use crate::database::Database;
use crate::rpc::StellarRpcClient;
use crate::state::AppState;

use std::time::Instant;

#[derive(Serialize)]
pub struct HealthStatus {
    pub status: String,
    pub timestamp: DateTime<Utc>,
    pub checks: HealthChecks,
}

#[derive(Serialize)]
pub struct HealthChecks {
    pub database: ComponentHealth,
    pub cache: ComponentHealth,
    pub rpc: ComponentHealth,
}

#[derive(Serialize)]
pub struct ComponentHealth {
    pub healthy: bool,
    pub response_time_ms: Option<u64>,
    pub message: Option<String>,
}

/// Check database health
async fn check_database(db: &Arc<Database>) -> ComponentHealth {
    let start = Instant::now();
    match sqlx::query("SELECT 1").fetch_one(db.pool()).await {
        Ok(_) => ComponentHealth {
            healthy: true,
            response_time_ms: Some(start.elapsed().as_millis() as u64),
            message: None,
        },
        Err(e) => ComponentHealth {
            healthy: false,
            response_time_ms: Some(start.elapsed().as_millis() as u64),
            message: Some(format!("Database connection failed: {}", e)),
        },
    }
}

/// Check cache health
async fn check_cache(cache: &Arc<CacheManager>) -> ComponentHealth {
    let start = Instant::now();
    match cache.ping().await {
        Ok(_) => ComponentHealth {
            healthy: true,
            response_time_ms: Some(start.elapsed().as_millis() as u64),
            message: None,
        },
        Err(e) => ComponentHealth {
            healthy: false,
            response_time_ms: Some(start.elapsed().as_millis() as u64),
            message: Some(format!("Cache connection failed: {}", e)),
        },
    }
}

/// Check RPC health
async fn check_rpc(rpc: &Arc<StellarRpcClient>) -> ComponentHealth {
    let start = Instant::now();
    match rpc.check_health().await {
        Ok(_) => ComponentHealth {
            healthy: true,
            response_time_ms: Some(start.elapsed().as_millis() as u64),
            message: None,
        },
        Err(e) => ComponentHealth {
            healthy: false,
            response_time_ms: Some(start.elapsed().as_millis() as u64),
            message: Some(format!("RPC connection failed: {}", e)),
        },
    }
}

/// Detailed health check endpoint
pub async fn health_check(
    State(db): State<Arc<Database>>,
    State(cache): State<Arc<CacheManager>>,
    State(rpc): State<Arc<StellarRpcClient>>,
) -> Json<HealthStatus> {
    let db_health = check_database(&db).await;
    let cache_health = check_cache(&cache).await;
    let rpc_health = check_rpc(&rpc).await;

    let overall = if db_health.healthy && cache_health.healthy {
        "healthy"
    } else {
        "degraded"
    };

    Json(HealthStatus {
        status: overall.to_string(),
        timestamp: Utc::now(),
        checks: HealthChecks {
            database: db_health,
            cache: cache_health,
            rpc: rpc_health,
        },
    })
}

/// Database pool metrics endpoint
pub async fn pool_metrics(State(state): State<AppState>) -> impl IntoResponse {
    let metrics = state.db.pool_metrics();
    Json(metrics)
}

pub async fn ingestion_status(
    State(app_state): State<AppState>,
) -> crate::error::ApiResult<Json<crate::ingestion::IngestionStatus>> {
    let status = app_state.ingestion.get_ingestion_status().await?;
    Ok(Json(status))
}
