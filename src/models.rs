use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub mod corridor;

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum SortBy {
    #[default]
    SuccessRate,
    Volume,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Anchor {
    pub id: Uuid,
    pub name: String,
    pub stellar_account: String,
    pub home_domain: Option<String>,
    pub total_transactions: i64,
    pub successful_transactions: i64,
    pub failed_transactions: i64,
    pub total_volume_usd: f64,
    pub avg_settlement_time_ms: i32,
    pub reliability_score: f64,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Asset {
    pub id: Uuid,
    pub anchor_id: Uuid,
    pub asset_code: String,
    pub asset_issuer: String,
    pub total_supply: Option<f64>,
    pub num_holders: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct AnchorMetricsHistory {
    pub id: Uuid,
    pub anchor_id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub success_rate: f64,
    pub failure_rate: f64,
    pub reliability_score: f64,
    pub total_transactions: i64,
    pub successful_transactions: i64,
    pub failed_transactions: i64,
    pub avg_settlement_time_ms: Option<i32>,
    pub volume_usd: Option<f64>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnchorMetrics {
    pub success_rate: f64,
    pub failure_rate: f64,
    pub reliability_score: f64,
    pub total_transactions: i64,
    pub successful_transactions: i64,
    pub failed_transactions: i64,
    pub avg_settlement_time_ms: Option<i32>,
    pub status: AnchorStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AnchorStatus {
    Green,  // >98% success, <1% failures
    Yellow, // 95-98% success, 1-5% failures
    Red,    // <95% success, >5% failures
}

impl AnchorStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            AnchorStatus::Green => "green",
            AnchorStatus::Yellow => "yellow",
            AnchorStatus::Red => "red",
        }
    }

    pub fn from_metrics(success_rate: f64, failure_rate: f64) -> Self {
        if success_rate > 98.0 && failure_rate < 1.0 {
            AnchorStatus::Green
        } else if success_rate >= 95.0 && failure_rate <= 5.0 {
            AnchorStatus::Yellow
        } else {
            AnchorStatus::Red
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnchorWithAssets {
    #[serde(flatten)]
    pub anchor: Anchor,
    pub assets: Vec<Asset>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnchorDetailResponse {
    pub anchor: Anchor,
    pub assets: Vec<Asset>,
    pub metrics_history: Vec<AnchorMetricsHistory>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateAnchorRequest {
    pub name: String,
    pub stellar_account: String,
    pub home_domain: Option<String>,
}

// =========================
// Corridor domain (new)
// =========================

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Corridor {
    pub id: Uuid,
    pub name: Option<String>,
    pub source_asset_code: String,
    pub source_asset_issuer: String,
    pub dest_asset_code: String,
    pub dest_asset_issuer: String,
    pub total_transactions: i64,
    pub successful_transactions: i64,
    pub failed_transactions: i64,
    pub avg_settlement_latency_ms: i32,
    pub liquidity_depth_usd: f64,
    pub success_rate: f64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorridorMetrics {
    pub success_rate: f64,
    pub avg_settlement_latency_ms: Option<i32>,
    pub liquidity_depth_usd: f64,
    pub total_transactions: i64,
    pub successful_transactions: i64,
    pub failed_transactions: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct CorridorMetricsHistory {
    pub id: Uuid,
    pub corridor_id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub success_rate: f64,
    pub avg_settlement_latency_ms: i32,
    pub liquidity_depth_usd: f64,
    pub total_transactions: i64,
    pub successful_transactions: i64,
    pub failed_transactions: i64,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateCorridorRequest {
    pub name: Option<String>,
    pub source_asset_code: String,
    pub source_asset_issuer: String,
    pub dest_asset_code: String,
    pub dest_asset_issuer: String,
}
