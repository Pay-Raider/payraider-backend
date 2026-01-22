use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::database::Database;
use crate::models::corridor::{Corridor, CorridorAnalytics, CorridorMetrics};
use crate::models::SortBy;

pub type ApiResult<T> = Result<T, ApiError>;

#[derive(Debug)]
pub enum ApiError {
    NotFound(String),
    BadRequest(String),
    InternalError(String),
}

impl IntoResponse for ApiError {
    fn into_response(self) -> axum::response::Response {
        let (status, message) = match self {
            ApiError::NotFound(msg) => (StatusCode::NOT_FOUND, msg),
            ApiError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg),
            ApiError::InternalError(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
        };

        (status, Json(serde_json::json!({ "error": message }))).into_response()
    }
}

impl From<anyhow::Error> for ApiError {
    fn from(err: anyhow::Error) -> Self {
        ApiError::InternalError(err.to_string())
    }
}

impl From<sqlx::Error> for ApiError {
    fn from(err: sqlx::Error) -> Self {
        ApiError::InternalError(err.to_string())
    }
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

#[derive(Debug, Serialize)]
pub struct CorridorResponse {
    pub asset_pair: String,
    pub asset_a_code: String,
    pub asset_a_issuer: String,
    pub asset_b_code: String,
    pub asset_b_issuer: String,
    pub success_rate: f64,
    pub total_transactions: i64,
    pub successful_transactions: i64,
    pub failed_transactions: i64,
    pub volume_usd: f64,
    pub last_updated: String,
}

#[derive(Debug, Serialize)]
pub struct CorridorsResponse {
    pub corridors: Vec<CorridorResponse>,
    pub total: usize,
}

impl From<CorridorMetrics> for CorridorResponse {
    fn from(metrics: CorridorMetrics) -> Self {
        let asset_pair = format!(
            "{}:{} -> {}:{}",
            metrics.asset_a_code,
            metrics.asset_a_issuer,
            metrics.asset_b_code,
            metrics.asset_b_issuer
        );

        CorridorResponse {
            asset_pair,
            asset_a_code: metrics.asset_a_code,
            asset_a_issuer: metrics.asset_a_issuer,
            asset_b_code: metrics.asset_b_code,
            asset_b_issuer: metrics.asset_b_issuer,
            success_rate: metrics.success_rate,
            total_transactions: metrics.total_transactions,
            successful_transactions: metrics.successful_transactions,
            failed_transactions: metrics.failed_transactions,
            volume_usd: metrics.volume_usd,
            last_updated: metrics.updated_at.format("%Y-%m-%d %H:%M:%S UTC").to_string(),
        }
    }
}

/// GET /api/corridors - List all corridors with their metrics
pub async fn get_corridors(
    State(db): State<Arc<Database>>,
    Query(params): Query<ListCorridorsQuery>,
) -> ApiResult<Json<CorridorsResponse>> {
    let corridors = db
        .list_corridor_metrics(params.limit, params.offset, params.sort_by)
        .await?;

    let corridor_responses: Vec<CorridorResponse> = corridors
        .into_iter()
        .map(CorridorResponse::from)
        .collect();

    let total = corridor_responses.len();

    Ok(Json(CorridorsResponse {
        corridors: corridor_responses,
        total,
    }))
}

/// GET /api/corridors/{asset_pair} - Get specific corridor metrics
pub async fn get_corridor_by_asset_pair(
    State(db): State<Arc<Database>>,
    Path(asset_pair): Path<String>,
) -> ApiResult<Json<CorridorResponse>> {
    // Parse asset_pair format: "USDC:issuer1->EURC:issuer2"
    let corridor_key = parse_asset_pair(&asset_pair)?;
    
    let corridor_metrics = db
        .get_corridor_metrics_by_key(&corridor_key)
        .await?
        .ok_or_else(|| {
            ApiError::NotFound(format!("Corridor with asset pair {} not found", asset_pair))
        })?;

    Ok(Json(CorridorResponse::from(corridor_metrics)))
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

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use uuid::Uuid;

    #[test]
    fn test_corridor_response_from_metrics() {
        let metrics = CorridorMetrics {
            id: Uuid::new_v4(),
            corridor_key: "EURC:issuer2->USDC:issuer1".to_string(),
            asset_a_code: "EURC".to_string(),
            asset_a_issuer: "issuer2".to_string(),
            asset_b_code: "USDC".to_string(),
            asset_b_issuer: "issuer1".to_string(),
            date: Utc::now(),
            total_transactions: 1000,
            successful_transactions: 950,
            failed_transactions: 50,
            success_rate: 95.0,
            volume_usd: 1000000.0,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let response = CorridorResponse::from(metrics);

        assert_eq!(response.asset_a_code, "EURC");
        assert_eq!(response.asset_b_code, "USDC");
        assert_eq!(response.success_rate, 95.0);
        assert_eq!(response.total_transactions, 1000);
        assert_eq!(response.volume_usd, 1000000.0);
        assert!(response.asset_pair.contains("EURC:issuer2"));
        assert!(response.asset_pair.contains("USDC:issuer1"));
    }

    #[test]
    fn test_parse_asset_pair_valid() {
        let asset_pair = "USDC:issuer1->EURC:issuer2";
        let result = parse_asset_pair(asset_pair);
        assert!(result.is_ok());
        
        let corridor_key = result.unwrap();
        // Should be normalized (EURC comes before USDC alphabetically)
        assert!(corridor_key.contains("EURC:issuer2"));
        assert!(corridor_key.contains("USDC:issuer1"));
    }

    #[test]
    fn test_parse_asset_pair_with_spaces() {
        let asset_pair = "USDC:issuer1 -> EURC:issuer2";
        let result = parse_asset_pair(asset_pair);
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_asset_pair_invalid_format() {
        let asset_pair = "USDC-EURC";
        let result = parse_asset_pair(asset_pair);
        assert!(result.is_err());
        
        if let Err(ApiError::BadRequest(msg)) = result {
            assert!(msg.contains("Invalid asset pair format"));
        }
    }

    #[test]
    fn test_parse_asset_pair_missing_issuer() {
        let asset_pair = "USDC->EURC:issuer2";
        let result = parse_asset_pair(asset_pair);
        assert!(result.is_err());
        
        if let Err(ApiError::BadRequest(msg)) = result {
            assert!(msg.contains("Invalid asset format"));
        }
    }

    #[test]
    fn test_sort_by_deserialization() {
        // Test default
        let query: ListCorridorsQuery = serde_json::from_str("{}").unwrap();
        assert!(matches!(query.sort_by, SortBy::SuccessRate));

        // Test success_rate
        let query: ListCorridorsQuery = serde_json::from_str(r#"{"sort_by": "success_rate"}"#).unwrap();
        assert!(matches!(query.sort_by, SortBy::SuccessRate));

        // Test volume
        let query: ListCorridorsQuery = serde_json::from_str(r#"{"sort_by": "volume"}"#).unwrap();
        assert!(matches!(query.sort_by, SortBy::Volume));
    }

    #[test]
    fn test_list_corridors_query_defaults() {
        let query: ListCorridorsQuery = serde_json::from_str("{}").unwrap();
        assert_eq!(query.limit, 50);
        assert_eq!(query.offset, 0);
        assert!(matches!(query.sort_by, SortBy::SuccessRate));
    }

    #[test]
    fn test_corridors_response_serialization() {
        let response = CorridorsResponse {
            corridors: vec![],
            total: 0,
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("corridors"));
        assert!(json.contains("total"));
    }
}