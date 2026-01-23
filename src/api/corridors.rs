use axum::{
    extract::{Path, Query, State},
    Json,
};
use chrono::{Utc, Duration};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::database::Database;
use crate::handlers::{ApiError, ApiResult};
use crate::models::corridor::{Corridor, CorridorAnalytics, CorridorMetrics};
use crate::models::SortBy;

// Response DTOs matching frontend TypeScript interfaces

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorridorResponse {
    pub id: String,
    pub source_asset: String,
    pub destination_asset: String,
    pub success_rate: f64,
    pub total_attempts: i64,
    pub successful_payments: i64,
    pub failed_payments: i64,
    pub average_latency_ms: f64,
    pub median_latency_ms: f64,
    pub p95_latency_ms: f64,
    pub p99_latency_ms: f64,
    pub liquidity_depth_usd: f64,
    pub liquidity_volume_24h_usd: f64,
    pub liquidity_trend: String,
    pub health_score: f64,
    pub last_updated: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuccessRateDataPoint {
    pub timestamp: String,
    pub success_rate: f64,
    pub attempts: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LatencyDataPoint {
    pub latency_bucket_ms: i32,
    pub count: i64,
    pub percentage: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiquidityDataPoint {
    pub timestamp: String,
    pub liquidity_usd: f64,
    pub volume_24h_usd: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorridorDetailResponse {
    pub corridor: CorridorResponse,
    pub historical_success_rate: Vec<SuccessRateDataPoint>,
    pub latency_distribution: Vec<LatencyDataPoint>,
    pub liquidity_trends: Vec<LiquidityDataPoint>,
    pub related_corridors: Option<Vec<CorridorResponse>>,
}

#[derive(Debug, Deserialize)]
pub struct ListCorridorsQuery {
    #[serde(default = "default_limit")]
    pub limit: i64,
    #[serde(default)]
    pub offset: i64,
    #[serde(default)]
    pub sort_by: SortBy,
}

fn default_limit() -> i64 {
    50
}

/// Calculate health score based on success rate, volume, and transaction count
fn calculate_health_score(success_rate: f64, total_transactions: i64, volume_usd: f64) -> f64 {
    let success_weight = 0.6;
    let volume_weight = 0.2;
    let transaction_weight = 0.2;
    
    // Normalize volume and transactions (using logarithmic scale)
    let volume_score = if volume_usd > 0.0 {
        ((volume_usd.ln() / 15.0) * 100.0).min(100.0)
    } else {
        0.0
    };
    
    let transaction_score = if total_transactions > 0 {
        ((total_transactions as f64).ln() / 10.0 * 100.0).min(100.0)  
    } else {
        0.0
    };
    
    (success_rate * success_weight + volume_score * volume_weight + transaction_score * transaction_weight)
}

/// Determine liquidity trend (simple heuristic based on recent data)
fn get_liquidity_trend(volume_usd: f64) -> String {
    // In real implementation, compare with historical data
    // For now, simple heuristic based on volume
    if volume_usd > 10_000_000.0 {
        "increasing".to_string()
    } else if volume_usd > 1_000_000.0 {
        "stable".to_string()
    } else {
        "decreasing".to_string()
    }
}

/// GET /api/corridors - List all corridors
pub async fn list_corridors(
    State(db): State<Arc<Database>>,
    Query(_params): Query<ListCorridorsQuery>,
) -> ApiResult<Json<Vec<CorridorResponse>>> {
    // Get today's date
    let today = Utc::now().date_naive();
    
    // Fetch corridor metrics from database
    let metrics = db.corridor_aggregates()
        .get_corridor_metrics_for_date(today)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to fetch corridors: {}", e)))?;
    
    // Convert to response format
    let corridors: Vec<CorridorResponse> = metrics
        .iter()
        .map(|m| {
            let health_score = calculate_health_score(
                m.success_rate,
                m.total_transactions,
                m.volume_usd,
            );
            
            let liquidity_trend = get_liquidity_trend(m.volume_usd);
            
            // Generate mock latency values (in real impl, get from database)
            let avg_latency = 400.0 + (m.success_rate * 2.0);
            let median_latency = avg_latency * 0.75;
            let p95_latency = avg_latency * 2.5;
            let p99_latency = avg_latency * 4.0;
            
            // Estimate 24h volume as 10% of total volume
            let volume_24h = m.volume_usd * 0.1;
            
            CorridorResponse {
                id: m.corridor_key.clone(),
                source_asset: m.asset_a_code.clone(),
                destination_asset: m.asset_b_code.clone(),
                success_rate: m.success_rate,
                total_attempts: m.total_transactions,
                successful_payments: m.successful_transactions,
                failed_payments: m.failed_transactions,
                average_latency_ms: avg_latency,
                median_latency_ms: median_latency,
                p95_latency_ms: p95_latency,
                p99_latency_ms: p99_latency,
                liquidity_depth_usd: m.volume_usd,
                liquidity_volume_24h_usd: volume_24h,
                liquidity_trend,
                health_score,
                last_updated: m.updated_at.to_rfc3339(),
            }
        })
        .collect();
    
    Ok(Json(corridors))
}

/// GET /api/corridors/:corridor_key - Get detailed corridor information
pub async fn get_corridor_detail(
    State(db): State<Arc<Database>>,
    Path(corridor_key): Path<String>,
) -> ApiResult<Json<CorridorDetailResponse>> {
    // Parse corridor key to get corridor structure
    let parts: Vec<&str> = corridor_key.split("->").collect();
    if parts.len() != 2 {
        return Err(ApiError::BadRequest("Invalid corridor key format".to_string()));
    }
    
    let asset_a_parts: Vec<&str> = parts[0].split(':').collect();
    let asset_b_parts: Vec<&str> = parts[1].split(':').collect();
    
    if asset_a_parts.len() != 2 || asset_b_parts.len() != 2 {
        return Err(ApiError::BadRequest("Invalid corridor key format".to_string()));
    }
    
    let corridor = crate::models::corridor::Corridor::new(
        asset_a_parts[0].to_string(),
        asset_a_parts[1].to_string(),
        asset_b_parts[0].to_string(),
        asset_b_parts[1].to_string(),
    );
    
    // Get 30 days of historical data
    let end_date = Utc::now().date_naive();
    let start_date = end_date - Duration::days(30);
    
    let metrics = db.corridor_aggregates()
        .get_corridor_metrics(&corridor, start_date, end_date)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to fetch corridor detail: {}", e)))?;
    
    if metrics.is_empty() {
        return Err(ApiError::NotFound(format!("Corridor {} not found", corridor_key)));
    }
    
    // Get latest metrics for main corridor data
    let latest = metrics.first().unwrap();
    
    let health_score = calculate_health_score(
        latest.success_rate,
        latest.total_transactions,
        latest.volume_usd,
    );
    
    let liquidity_trend = get_liquidity_trend(latest.volume_usd);
    
    let avg_latency = 400.0 + (latest.success_rate * 2.0);
    let median_latency = avg_latency * 0.75;
    let p95_latency = avg_latency * 2.5;
    let p99_latency = avg_latency * 4.0;
    let volume_24h = latest.volume_usd * 0.1;
    
    let corridor_response = CorridorResponse {
        id: latest.corridor_key.clone(),
        source_asset: latest.asset_a_code.clone(),
        destination_asset: latest.asset_b_code.clone(),
        success_rate: latest.success_rate,
        total_attempts: latest.total_transactions,
        successful_payments: latest.successful_transactions,
        failed_payments: latest.failed_transactions,
        average_latency_ms: avg_latency,
        median_latency_ms: median_latency,
        p95_latency_ms: p95_latency,
        p99_latency_ms: p99_latency,
        liquidity_depth_usd: latest.volume_usd,
        liquidity_volume_24h_usd: volume_24h,
        liquidity_trend,
        health_score,
        last_updated: latest.updated_at.to_rfc3339(),
    };
    
    // Historical success rate
    let historical_success_rate: Vec<SuccessRateDataPoint> = metrics
        .iter()
        .rev() // Oldest first
        .map(|m| SuccessRateDataPoint {
            timestamp: m.date.format("%Y-%m-%d").to_string(),
            success_rate: m.success_rate,
            attempts: m.total_transactions,
        })
        .collect();
    
    // Mock latency distribution (in real impl, get from database)
    let latency_distribution = vec![
        LatencyDataPoint { latency_bucket_ms: 100, count: 250, percentage: 15.0 },
        LatencyDataPoint { latency_bucket_ms: 250, count: 520, percentage: 31.0 },
        LatencyDataPoint { latency_bucket_ms: 500, count: 580, percentage: 35.0 },
        LatencyDataPoint { latency_bucket_ms: 1000, count: 280, percentage: 17.0 },
        LatencyDataPoint { latency_bucket_ms: 2000, count: 50, percentage: 3.0 },
    ];
    
    // Liquidity trends from historical data
    let liquidity_trends: Vec<LiquidityDataPoint> = metrics
        .iter()
        .rev() // Oldest first
        .map(|m| LiquidityDataPoint {
            timestamp: m.date.format("%Y-%m-%d").to_string(),
            liquidity_usd: m.volume_usd,
            volume_24h_usd: m.volume_usd * 0.1,
        })
        .collect();
    
    // Get related corridors (top 3 by volume on the same day)
    let related_metrics = db.corridor_aggregates()
        .get_top_corridors_by_volume(end_date, 4)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to fetch related corridors: {}", e)))?;
    
    let related_corridors: Vec<CorridorResponse> = related_metrics
        .iter()
        .filter(|m| m.corridor_key != corridor_key) // Exclude current corridor
        .take(3)
        .map(|m| {
            let health_score = calculate_health_score(m.success_rate, m.total_transactions, m.volume_usd);
            let liquidity_trend = get_liquidity_trend(m.volume_usd);
            let avg_latency = 400.0 + (m.success_rate * 2.0);
            
            CorridorResponse {
                id: m.corridor_key.clone(),
                source_asset: m.asset_a_code.clone(),
                destination_asset: m.asset_b_code.clone(),
                success_rate: m.success_rate,
                total_attempts: m.total_transactions,
                successful_payments: m.successful_transactions,
                failed_payments: m.failed_transactions,
                average_latency_ms: avg_latency,
                median_latency_ms: avg_latency * 0.75,
                p95_latency_ms: avg_latency * 2.5,
                p99_latency_ms: avg_latency * 4.0,
                liquidity_depth_usd: m.volume_usd,
                liquidity_volume_24h_usd: m.volume_usd * 0.1,
                liquidity_trend,
                health_score,
                last_updated: m.updated_at.to_rfc3339(),
            }
        })
        .collect();
    
    Ok(Json(CorridorDetailResponse {
        corridor: corridor_response,
        historical_success_rate,
        latency_distribution,
        liquidity_trends,
        related_corridors: Some(related_corridors),
    }))
}

fn parse_asset_pair(asset_pair: &str) -> ApiResult<String> {
    // Expected format: "USDC:issuer1->EURC:issuer2" or "USDC:issuer1 -> EURC:issuer2"
    let normalized = asset_pair.replace(" ", "");
    
    if !normalized.contains("->") {
        return Err(ApiError::BadRequest(
            "Invalid asset pair format. Expected: 'ASSET_A:ISSUER_A->ASSET_B:ISSUER_B'".to_string(),
        ));
    }

    let parts: Vec<&str> = normalized.split("->").collect();
    if parts.len() != 2 {
        return Err(ApiError::BadRequest(
            "Invalid asset pair format. Expected: 'ASSET_A:ISSUER_A->ASSET_B:ISSUER_B'".to_string(),
        ));
    }

    let asset_a_parts: Vec<&str> = parts[0].split(':').collect();
    let asset_b_parts: Vec<&str> = parts[1].split(':').collect();

    if asset_a_parts.len() != 2 || asset_b_parts.len() != 2 {
        return Err(ApiError::BadRequest(
            "Invalid asset format. Each asset must be in format 'CODE:ISSUER'".to_string(),
        ));
    }

    // Create normalized corridor key using the Corridor struct
    let corridor = Corridor::new(
        asset_a_parts[0].to_string(),
        asset_a_parts[1].to_string(),
        asset_b_parts[0].to_string(),
        asset_b_parts[1].to_string(),
    );

    Ok(corridor.to_string_key())
}
