use axum::{
    extract::{Extension, OriginalUri, Path, Query, State},
    http::HeaderMap,
    response::Response,
    Json,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{error, info, warn};
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

use crate::broadcast::broadcast_corridor_update;
use crate::cache::helpers::cached_query;
use crate::cache::keys;
use crate::cache::CacheManager;
use crate::database::Database;
use crate::error::{ApiError, ApiResult};
use crate::models::corridor::Corridor;
use crate::models::{CreateCorridorRequest, SortBy};
use crate::pagination::{Page, PaginatedResponse, PaginationParams};
use crate::request_id::RequestId;
use crate::rpc::{
    circuit_breaker::rpc_circuit_breaker,
    error::{with_retry, RetryConfig, RpcError},
    StellarRpcClient,
};
use crate::services::analytics::{compute_corridor_metrics, CorridorPayment};
use crate::services::price_feed::PriceFeedClient;
use crate::state::AppState;
use crate::validation;

/// Represents an asset pair (source -> destination) for a corridor
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct AssetPair {
    source_asset: String,
    destination_asset: String,
}

impl AssetPair {
    fn to_corridor_key(&self) -> String {
        format!("{}->{}", self.source_asset, self.destination_asset)
    }
}

/// Extract asset pair from a payment operation
/// Handles regular payments, `path_payment_strict_send`, and `path_payment_strict_receive`
fn extract_asset_pair_from_payment(payment: &crate::rpc::Payment) -> Option<AssetPair> {
    let operation_type = payment.operation_type.as_deref().unwrap_or("payment");

    match operation_type {
        "path_payment_strict_send" | "path_payment_strict_receive" => {
            // Path payments have explicit source and destination assets
            let source_asset = if let Some(src_type) = &payment.source_asset_type {
                if src_type == "native" {
                    "XLM:native".to_string()
                } else {
                    format!(
                        "{}:{}",
                        payment.source_asset_code.as_deref().unwrap_or("UNKNOWN"),
                        payment.source_asset_issuer.as_deref().unwrap_or("unknown")
                    )
                }
            } else {
                return None;
            };

            let destination_asset = if payment.asset_type == "native" {
                "XLM:native".to_string()
            } else {
                format!(
                    "{}:{}",
                    payment.get_asset_code().as_deref().unwrap_or("UNKNOWN"),
                    payment.get_asset_issuer().as_deref().unwrap_or("unknown")
                )
            };

            Some(AssetPair {
                source_asset,
                destination_asset,
            })
        }
        _ => {
            // Regular payments: same asset for source and destination
            let asset = if payment.asset_type == "native" {
                "XLM:native".to_string()
            } else {
                format!(
                    "{}:{}",
                    payment.get_asset_code().as_deref().unwrap_or("UNKNOWN"),
                    payment.get_asset_issuer().as_deref().unwrap_or("unknown")
                )
            };

            Some(AssetPair {
                source_asset: asset.clone(),
                destination_asset: asset,
            })
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CorridorResponse {
    /// Unique identifier for the corridor
    #[schema(example = "USDC:native->XLM:native")]
    pub id: String,
    /// Source asset code
    #[schema(example = "USDC")]
    pub source_asset: String,
    /// Destination asset code
    #[schema(example = "XLM")]
    pub destination_asset: String,
    /// Success rate percentage
    #[schema(example = 99.8)]
    pub success_rate: f64,
    /// Total payment attempts
    #[schema(example = 5000)]
    pub total_attempts: i64,
    /// Number of successful payments
    #[schema(example = 4990)]
    pub successful_payments: i64,
    /// Number of failed payments
    #[schema(example = 10)]
    pub failed_payments: i64,
    /// Average latency in milliseconds
    #[schema(example = 450.5)]
    pub average_latency_ms: f64,
    /// Median latency in milliseconds
    #[schema(example = 380.0)]
    pub median_latency_ms: f64,
    /// 95th percentile latency in milliseconds
    #[schema(example = 850.0)]
    pub p95_latency_ms: f64,
    /// 99th percentile latency in milliseconds
    #[schema(example = 1200.0)]
    pub p99_latency_ms: f64,
    /// Liquidity depth in USD
    #[schema(example = 1_500_000.0)]
    pub liquidity_depth_usd: f64,
    /// 24-hour trading volume in USD
    #[schema(example = 150_000.0)]
    pub liquidity_volume_24h_usd: f64,
    /// Liquidity trend (increasing, stable, decreasing)
    #[schema(example = "stable")]
    pub liquidity_trend: String,
    /// Overall health score (0-100)
    #[schema(example = 95.5)]
    pub health_score: f64,
    /// Last update timestamp
    #[schema(example = "2024-01-15T10:30:00Z")]
    pub last_updated: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SuccessRateDataPoint {
    /// Timestamp of the data point
    #[schema(example = "2024-01-15T10:00:00Z")]
    pub timestamp: String,
    /// Success rate percentage at this time
    #[schema(example = 99.5)]
    pub success_rate: f64,
    /// Number of attempts at this time
    #[schema(example = 150)]
    pub attempts: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct LatencyDataPoint {
    /// Latency bucket in milliseconds
    #[schema(example = 500)]
    pub latency_bucket_ms: i32,
    /// Number of transactions in this bucket
    #[schema(example = 250)]
    pub count: i64,
    /// Percentage of total transactions
    #[schema(example = 25.5)]
    pub percentage: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct LiquidityDataPoint {
    /// Timestamp of the data point
    #[schema(example = "2024-01-15T10:00:00Z")]
    pub timestamp: String,
    /// Liquidity in USD at this time
    #[schema(example = 1_500_000.0)]
    pub liquidity_usd: f64,
    /// 24-hour volume in USD
    #[schema(example = 150_000.0)]
    pub volume_24h_usd: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CorridorDetailResponse {
    /// Corridor summary information
    pub corridor: CorridorResponse,
    /// Historical success rate data points
    pub historical_success_rate: Vec<SuccessRateDataPoint>,
    /// Latency distribution histogram
    pub latency_distribution: Vec<LatencyDataPoint>,
    /// Liquidity trend over time
    pub liquidity_trends: Vec<LiquidityDataPoint>,
    /// Related corridors
    pub related_corridors: Option<Vec<CorridorResponse>>,
}

/// Query parameters for listing corridors with filtering and pagination.
#[derive(Debug, Default, Deserialize, IntoParams)]
#[serde(default)]
#[into_params(parameter_in = Query)]
pub struct ListCorridorsQuery {
    /// Maximum number of results to return (default: 50, max: 200)
    #[param(example = 50)]
    pub limit: Option<i64>,
    /// Opaque cursor from `pagination.next_cursor` / `prev_cursor`
    pub cursor: Option<String>,
    /// Deprecated: pagination offset. Prefer `cursor`.
    #[param(example = 0)]
    pub offset: Option<i64>,
    /// Sort by field (`success_rate`, `volume`/`liquidity`, or `health_score`)
    #[serde(default)]
    pub sort_by: SortBy,
    /// Minimum success rate filter
    #[param(example = 95.0)]
    pub success_rate_min: Option<f64>,
    /// Maximum success rate filter
    #[param(example = 100.0)]
    pub success_rate_max: Option<f64>,
    /// Minimum volume filter (USD)
    #[param(example = 100_000.0)]
    pub volume_min: Option<f64>,
    /// Maximum volume filter (USD)
    #[param(example = 10_000_000.0)]
    pub volume_max: Option<f64>,
    /// Filter by asset code
    #[param(example = "USDC")]
    pub asset_code: Option<String>,
    /// Time period for metrics (24h, 7d, 30d)
    #[param(example = "24h")]
    pub time_period: Option<String>,
}

const DEFAULT_LIMIT: i64 = 50;
const MAX_LIMIT: i64 = 200;

impl ListCorridorsQuery {
    fn page(&self) -> ApiResult<Page> {
        PaginationParams {
            limit: self.limit,
            cursor: self.cursor.clone(),
            offset: self.offset,
        }
        .resolve(DEFAULT_LIMIT, MAX_LIMIT)
    }
}

fn calculate_health_score(success_rate: f64, total_transactions: i64, volume_usd: f64) -> f64 {
    let success_weight = 0.6;
    let volume_weight = 0.2;
    let transaction_weight = 0.2;

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

    success_rate.mul_add(success_weight, volume_score * volume_weight)
        + transaction_score * transaction_weight
}

fn get_liquidity_trend(volume_usd: f64) -> String {
    if volume_usd > 10_000_000.0 {
        "increasing".to_string()
    } else if volume_usd > 1_000_000.0 {
        "stable".to_string()
    } else {
        "decreasing".to_string()
    }
}

/// Generate cache key for corridor list with filters
fn generate_corridor_list_cache_key(params: &ListCorridorsQuery, page: Page) -> String {
    let filter_str = format!(
        "sr_min:{:?}_sr_max:{:?}_vol_min:{:?}_vol_max:{:?}_asset:{:?}_period:{:?}_sort:{:?}",
        params.success_rate_min,
        params.success_rate_max,
        params.volume_min,
        params.volume_max,
        params.asset_code,
        params.time_period,
        params.sort_by
    );
    keys::corridor_list(page.limit, page.offset, &filter_str)
}

/// List all payment corridors
///
/// Returns a list of payment corridors with performance metrics.
/// Supports filtering by success rate, volume, and asset code.
///
/// **DATA SOURCE: RPC**
/// - Payment data from Horizon API
/// - Trade data from Horizon API
/// - Order book data from Horizon API
/// - Calculates corridor metrics from real-time RPC data
#[utoipa::path(
    get,
    path = "/api/corridors",
    params(ListCorridorsQuery),
    responses(
        (status = 200, description = "Paginated list of corridors (`PaginatedResponse<CorridorResponse>`)", body = PaginatedResponse<CorridorResponse>),
        (status = 400, description = "Invalid filter or pagination cursor"),
        (status = 500, description = "Internal server error")
    ),
    tag = "Corridors"
)]
#[tracing::instrument(
    skip(_db, cache, rpc_client, price_feed, params, headers, uri),
    fields(request_id = %request_id.0, query = ?params)
)]
pub async fn list_corridors(
    Extension(request_id): Extension<RequestId>,
    State((_db, cache, rpc_client, price_feed)): State<(
        Arc<Database>,
        Arc<CacheManager>,
        Arc<StellarRpcClient>,
        Arc<PriceFeedClient>,
    )>,
    OriginalUri(uri): OriginalUri,
    Query(params): Query<ListCorridorsQuery>,
    headers: HeaderMap,
) -> ApiResult<Response> {
    info!("Listing corridors");

    let page = params.page()?;

    validation::validate_corridor_filters(
        params.success_rate_min,
        params.success_rate_max,
        params.volume_min,
        params.volume_max,
    )?;

    let cache_key = generate_corridor_list_cache_key(&params, page);

    let corridors = cached_query(
        &cache,
        &cache_key,
        cache.config.get_ttl("corridor"),
        || async {
            let corridor_responses = compute_live_corridors(&rpc_client, &price_feed).await?;

            // Apply filters
            let filtered: Vec<_> = corridor_responses
                .into_iter()
                .filter(|c| {
                    if let Some(min) = params.success_rate_min {
                        if c.success_rate < min {
                            return false;
                        }
                    }
                    if let Some(max) = params.success_rate_max {
                        if c.success_rate > max {
                            return false;
                        }
                    }
                    if let Some(min) = params.volume_min {
                        if c.liquidity_depth_usd < min {
                            return false;
                        }
                    }
                    if let Some(max) = params.volume_max {
                        if c.liquidity_depth_usd > max {
                            return false;
                        }
                    }
                    if let Some(asset_code) = &params.asset_code {
                        let asset_code_lower = asset_code.to_lowercase();
                        if !c.source_asset.to_lowercase().contains(&asset_code_lower)
                            && !c
                                .destination_asset
                                .to_lowercase()
                                .contains(&asset_code_lower)
                        {
                            return false;
                        }
                    }
                    true
                })
                .collect();

            // Corridors come out of a HashMap, so impose a stable order before
            // paging or cursors would skip/repeat items between requests.
            let mut filtered = filtered;
            filtered.sort_by(|a, b| {
                let (x, y) = match params.sort_by {
                    SortBy::SuccessRate => (a.success_rate, b.success_rate),
                    SortBy::Volume => (a.liquidity_depth_usd, b.liquidity_depth_usd),
                    SortBy::HealthScore => (a.health_score, b.health_score),
                };
                y.total_cmp(&x).then_with(|| a.id.cmp(&b.id))
            });

            let total = filtered.len() as i64;
            let items: Vec<_> = filtered
                .into_iter()
                .skip(page.offset as usize)
                .take(page.limit as usize)
                .collect();

            Ok(PaginatedResponse::from_page(items, total, page))
        },
    )
    .await?;

    crate::observability::metrics::set_corridors_tracked(
        corridors.pagination.total.unwrap_or_default(),
    );
    let corridors = corridors.with_links(&uri);

    let ttl = cache.config.get_ttl("corridor");
    let response = crate::http_cache::cached_json_response(&headers, &cache_key, &corridors, ttl)?;
    Ok(response)
}

/// Build the live corridor table from recent RPC payments.
///
/// Shared by the corridor list endpoint and the pre-payment check so both
/// score corridors from the same data.
pub(crate) async fn compute_live_corridors(
    rpc_client: &StellarRpcClient,
    price_feed: &PriceFeedClient,
) -> anyhow::Result<Vec<CorridorResponse>> {
    let payments = recent_payments(rpc_client).await?;
    Ok(corridors_from_payments(&payments, price_feed).await)
}

/// Payments to score corridors from: the rolling window the background
/// refresher keeps, or the head of the feed if it has not filled yet (just
/// after startup).
async fn recent_payments(
    rpc_client: &StellarRpcClient,
) -> anyhow::Result<Vec<crate::rpc::Payment>> {
    let held = crate::services::payment_window::PaymentWindow::global()
        .snapshot()
        .await;
    if !held.is_empty() {
        return Ok(held);
    }

    let circuit_breaker = rpc_circuit_breaker();
    with_retry(
        || async {
            rpc_client
                .fetch_all_payments(Some(1000))
                .await
                .map_err(|e| RpcError::categorize(&e.to_string()))
        },
        RetryConfig::default(),
        circuit_breaker.clone(),
    )
    .await
    .map_err(|e| anyhow::anyhow!("Failed to fetch payments from RPC: {e}"))
}

/// Build the corridor table from a set of payments. Separate from the fetch
/// so it can be tested without the network or the shared circuit breaker.
pub(crate) async fn corridors_from_payments(
    payments: &[crate::rpc::Payment],
    price_feed: &PriceFeedClient,
) -> Vec<CorridorResponse> {
    // Group payments by asset pairs to identify corridors
    use std::collections::HashMap;
    let mut corridor_map: HashMap<String, Vec<&crate::rpc::Payment>> = HashMap::new();

    for payment in payments
        .iter()
        .filter(|p| !crate::services::payment_window::is_self_payment(p))
    {
        // Extract the actual asset pair from the payment
        if let Some(asset_pair) = extract_asset_pair_from_payment(payment) {
            let corridor_key = asset_pair.to_corridor_key();
            corridor_map.entry(corridor_key).or_default().push(payment);
        } else {
            warn!(
                payment_id = crate::logging::redaction::redact_hash(&payment.id),
                "Failed to extract asset pair from payment"
            );
        }
    }

    // Calculate metrics for each corridor
    let mut corridor_responses = Vec::new();

    // Batch-fetch prices for every distinct source asset up front instead of
    // awaiting price_feed.get_price() once per corridor below — with a large
    // number of distinct corridors that turned into N sequential round trips.
    let source_assets: Vec<String> = corridor_map
        .keys()
        .filter_map(|k| k.split("->").next())
        .map(String::from)
        .collect();
    let prices = price_feed.get_prices(&source_assets).await;

    for (corridor_key, corridor_payments) in &corridor_map {
        let total_attempts = corridor_payments.len() as i64;

        // Horizon is queried with include_failed=true, so failed payments are
        // in the stream and marked as such.
        let successful_payments = corridor_payments.iter().filter(|p| p.succeeded()).count() as i64;
        let failed_payments = total_attempts - successful_payments;
        let success_rate = if total_attempts > 0 {
            successful_payments as f64 / total_attempts as f64 * 100.0
        } else {
            0.0
        };

        // Parse corridor key to get assets
        let parts: Vec<&str> = corridor_key.split("->").collect();
        if parts.len() != 2 {
            continue;
        }

        let source_parts: Vec<&str> = parts[0].split(':').collect();
        let dest_parts: Vec<&str> = parts[1].split(':').collect();

        if source_parts.len() != 2 || dest_parts.len() != 2 {
            continue;
        }

        // Calculate volume from payment amounts and convert to USD
        let mut volume_usd: f64 = 0.0;
        let source_asset_key = parts[0];

        // Get price for source asset from the batch fetched above
        if let Some(&price) = prices.get(source_asset_key) {
            // Only payments that settled moved value through the corridor.
            for payment in corridor_payments.iter().filter(|p| p.succeeded()) {
                if let Ok(amount) = payment.get_amount().parse::<f64>() {
                    volume_usd += amount * price;
                }
            }
        } else {
            // No price: the asset's dollar value is unknown, so its volume is
            // reported as zero rather than counting raw token units as dollars.
            tracing::debug!(
                "Price unavailable for {}, volume left at 0",
                source_asset_key
            );
        }

        // Calculate health score
        let health_score = calculate_health_score(success_rate, total_attempts, volume_usd);
        let liquidity_trend = get_liquidity_trend(volume_usd);

        let corridor_response = CorridorResponse {
            id: corridor_key.clone(),
            source_asset: source_parts[0].to_string(),
            destination_asset: dest_parts[0].to_string(),
            success_rate,
            total_attempts,
            successful_payments,
            failed_payments,
            // Horizon's payment records carry no submission time, so
            // settlement latency is not measured; 0 means "not available".
            average_latency_ms: 0.0,
            median_latency_ms: 0.0,
            p95_latency_ms: 0.0,
            p99_latency_ms: 0.0,
            liquidity_depth_usd: volume_usd,
            // The window covers the last 24 hours.
            liquidity_volume_24h_usd: volume_usd,
            liquidity_trend,
            health_score,
            last_updated: chrono::Utc::now().to_rfc3339(),
        };

        corridor_responses.push(corridor_response);
    }

    corridor_responses
}

/// Hour bucket (`2026-01-01T13`) of a Horizon timestamp.
fn hour_bucket(created_at: &str) -> Option<&str> {
    created_at.get(..13)
}

/// Success rate per hour across the window, from each payment's outcome.
fn calculate_historical_success_rate(
    corridor_payments: &[&crate::rpc::Payment],
) -> Vec<SuccessRateDataPoint> {
    let mut hourly: std::collections::BTreeMap<&str, (i64, i64)> =
        std::collections::BTreeMap::new();
    for payment in corridor_payments {
        if let Some(hour) = hour_bucket(&payment.created_at) {
            let entry = hourly.entry(hour).or_insert((0, 0));
            entry.0 += 1;
            if payment.succeeded() {
                entry.1 += 1;
            }
        }
    }

    hourly
        .into_iter()
        .map(|(hour, (total, successful))| SuccessRateDataPoint {
            timestamp: format!("{hour}:00:00Z"),
            success_rate: successful as f64 / total as f64 * 100.0,
            attempts: total,
        })
        .collect()
}

/// Settled volume per hour, in USD when the source asset has a price.
/// Without a price the dollar value is unknown and no points are returned.
fn calculate_liquidity_trends(
    corridor_payments: &[&crate::rpc::Payment],
    price_usd: Option<f64>,
) -> Vec<LiquidityDataPoint> {
    let Some(price) = price_usd else {
        return Vec::new();
    };
    let mut hourly: std::collections::BTreeMap<&str, f64> = std::collections::BTreeMap::new();
    for payment in corridor_payments.iter().filter(|p| p.succeeded()) {
        if let (Some(hour), Ok(amount)) = (
            hour_bucket(&payment.created_at),
            payment.get_amount().parse::<f64>(),
        ) {
            *hourly.entry(hour).or_insert(0.0) += amount * price;
        }
    }

    hourly
        .into_iter()
        .map(|(hour, volume)| LiquidityDataPoint {
            timestamp: format!("{hour}:00:00Z"),
            liquidity_usd: volume,
            volume_24h_usd: volume,
        })
        .collect()
}

/// Find related corridors (same source or destination asset)
fn find_related_corridors(
    target_corridor_key: &str,
    all_corridors: &[CorridorResponse],
) -> Option<Vec<CorridorResponse>> {
    let parts: Vec<&str> = target_corridor_key.split("->").collect();
    if parts.len() != 2 {
        return None;
    }

    let target_source = parts[0];
    let target_dest = parts[1];

    let related: Vec<_> = all_corridors
        .iter()
        .filter(|c| {
            // Include corridors with same source or destination asset (excluding the target itself)
            (c.id == target_corridor_key)
                || c.id.starts_with(&format!("{target_source}->"))
                || c.id.ends_with(&format!("->{target_dest}"))
        })
        .cloned()
        .collect();

    if related.is_empty() {
        None
    } else {
        Some(related)
    }
}

/// Get detailed corridor information
///
/// Returns detailed metrics and historical data for a specific corridor.
///
/// **DATA SOURCE: RPC**
#[utoipa::path(
    get,
    path = "/api/corridors/{corridor_key}",
    params(
        ("corridor_key" = String, Path, description = "Corridor identifier (e.g., USDC:native->XLM:native)")
    ),
    responses(
        (status = 200, description = "Corridor details retrieved successfully", body = CorridorDetailResponse),
        (status = 404, description = "Corridor not found"),
        (status = 500, description = "Internal server error")
    ),
    tag = "Corridors"
)]
#[tracing::instrument(
    skip(_db, cache, rpc_client, price_feed, headers),
    fields(request_id = %request_id.0, corridor_key = %corridor_key)
)]
pub async fn get_corridor_detail(
    Extension(request_id): Extension<RequestId>,
    State((_db, cache, rpc_client, price_feed)): State<(
        Arc<Database>,
        Arc<CacheManager>,
        Arc<StellarRpcClient>,
        Arc<PriceFeedClient>,
    )>,
    Path(corridor_key): Path<String>,
    headers: HeaderMap,
) -> ApiResult<Response> {
    info!("Fetching corridor");

    // Validate corridor_key format
    let parts: Vec<&str> = corridor_key.split("->").collect();
    if parts.len() != 2 {
        return Err(ApiError::bad_request(
            "INVALID_CORRIDOR_FORMAT",
            "Corridor key must be in format 'ASSET1:ISSUER1->ASSET2:ISSUER2'",
        ));
    }

    let source_key = parts[0];
    let dest_key = parts[1];

    // Parse asset components
    let source_parts: Vec<&str> = source_key.split(':').collect();
    let dest_parts: Vec<&str> = dest_key.split(':').collect();

    if source_parts.len() != 2 || dest_parts.len() != 2 {
        return Err(ApiError::bad_request(
            "INVALID_ASSET_FORMAT",
            "Asset format must be 'CODE:ISSUER'",
        ));
    }

    let cache_key = keys::corridor_detail(&corridor_key);
    let response = cached_query(&cache, &cache_key, 300, || async {
        let payments = recent_payments(&rpc_client).await.map_err(|e| {
            error!(error = %e, "Failed to fetch payments from RPC");
            anyhow::anyhow!("Failed to fetch payment data from RPC")
        })?;

        // Score every corridor the same way the list does, so the detail page
        // and the list never disagree.
        let all_corridors = corridors_from_payments(&payments, &price_feed).await;
        let corridor = all_corridors
            .iter()
            .find(|c| c.id == corridor_key)
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("No payment data found for corridor: {corridor_key}"))?;

        let corridor_payments: Vec<&crate::rpc::Payment> = payments
            .iter()
            .filter(|p| !crate::services::payment_window::is_self_payment(p))
            .filter(|p| {
                extract_asset_pair_from_payment(p)
                    .is_some_and(|pair| pair.to_corridor_key() == corridor_key)
            })
            .collect();

        let price = price_feed
            .get_prices(&[source_key.to_string()])
            .await
            .get(source_key)
            .copied();

        Ok(CorridorDetailResponse {
            historical_success_rate: calculate_historical_success_rate(&corridor_payments),
            // Not measured: see the latency fields on CorridorResponse.
            latency_distribution: Vec::new(),
            liquidity_trends: calculate_liquidity_trends(&corridor_payments, price),
            related_corridors: find_related_corridors(&corridor_key, &all_corridors),
            corridor,
        })
    })
    .await
    .map_err(|error| {
        let message = error.to_string();
        if message.contains("No payment data found for corridor") {
            ApiError::not_found("CORRIDOR_NOT_FOUND", "Corridor not found")
        } else {
            ApiError::from(error)
        }
    })?;

    // Log successful corridor fetch
    info!(
        corridor_id = %response.corridor.id,
        success_rate = response.corridor.success_rate,
        "Corridor found"
    );

    let ttl = cache.config.get_ttl("corridor");
    let cached = crate::http_cache::cached_json_response(&headers, &cache_key, &response, ttl)?;
    Ok(cached)
}

/// POST /api/corridors - Create a new corridor
#[utoipa::path(
    post,
    path = "/api/corridors",
    request_body = crate::models::CreateCorridorRequest,
    responses(
        (status = 200, description = "Corridor created", body = Corridor),
        (status = 400, description = "Validation error"),
        (status = 401, description = "Missing or invalid credentials"),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Corridors"
)]
pub async fn create_corridor(
    State(app_state): State<AppState>,
    crate::validation::ValidatedJson(req): crate::validation::ValidatedJson<CreateCorridorRequest>,
) -> ApiResult<Json<Corridor>> {
    // Business logic: source and destination must differ
    crate::validation::validate_corridor_not_self_referential(
        &req.source_asset_code,
        &req.source_asset_issuer,
        &req.dest_asset_code,
        &req.dest_asset_issuer,
    )?;

    let corridor = app_state.db.create_corridor(req).await?;
    Ok(Json(corridor))
}

/// PUT /api/corridors/:id/metrics-from-transactions - Compute metrics from transactions and persist
#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateCorridorMetricsFromTxns {
    pub transactions: Vec<CorridorPaymentDto>,
}

#[derive(Debug, Deserialize, ToSchema)]
#[schema(example = json!({"successful": true, "settlement_latency_ms": 3500, "amount_usd": 250.0}))]
pub struct CorridorPaymentDto {
    pub successful: bool,
    pub settlement_latency_ms: Option<i32>,
    pub amount_usd: f64,
}

#[utoipa::path(
    put,
    path = "/api/corridors/{id}/metrics-from-transactions",
    params(("id" = String, Path, description = "Corridor UUID")),
    request_body = UpdateCorridorMetricsFromTxns,
    responses(
        (status = 200, description = "Corridor metrics recomputed", body = Corridor),
        (status = 401, description = "Missing or invalid credentials"),
        (status = 404, description = "Corridor not found"),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Corridors"
)]
pub async fn update_corridor_metrics_from_transactions(
    State(app_state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateCorridorMetricsFromTxns>,
) -> ApiResult<Json<Corridor>> {
    if app_state.db.get_corridor_by_id(id).await?.is_none() {
        let mut details = HashMap::new();
        details.insert("corridor_id".to_string(), serde_json::json!(id.to_string()));
        return Err(ApiError::not_found_with_details(
            "CORRIDOR_NOT_FOUND",
            format!("Corridor with id {id} not found"),
            details,
        ));
    }

    let txs: Vec<CorridorPayment> = req
        .transactions
        .into_iter()
        .map(|t| CorridorPayment {
            successful: t.successful,
            settlement_latency_ms: t.settlement_latency_ms,
            amount_usd: t.amount_usd,
        })
        .collect();

    let metrics = compute_corridor_metrics(&txs, None, 1.0);
    let corridor = app_state
        .db
        .update_corridor_metrics(id, metrics, &app_state.cache)
        .await?;

    // Broadcast the corridor update to WebSocket clients
    broadcast_corridor_update(&app_state.ws_state, &corridor);

    Ok(Json(corridor))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn live_corridors_count_failed_payments() {
        use crate::services::price_feed::{default_asset_mapping, PriceFeedConfig};

        // Mock data marks every seventh payment as failed.
        let payments = crate::rpc::mock_stellar::mock_payments(200);
        let price_feed = PriceFeedClient::new(PriceFeedConfig::default(), default_asset_mapping());

        let corridors = corridors_from_payments(&payments, &price_feed).await;

        assert!(!corridors.is_empty());
        for corridor in &corridors {
            assert_eq!(
                corridor.successful_payments + corridor.failed_payments,
                corridor.total_attempts
            );
            let expected =
                corridor.successful_payments as f64 / corridor.total_attempts as f64 * 100.0;
            assert!((corridor.success_rate - expected).abs() < 1e-9);
        }
        assert!(
            corridors
                .iter()
                .any(|c| c.failed_payments > 0 && c.success_rate < 100.0),
            "failed payments must lower a corridor's success rate"
        );
    }

    #[test]
    fn test_health_score_calculation() {
        let score = calculate_health_score(95.0, 1000, 1_000_000.0);
        assert!(score > 0.0 && score <= 100.0);
    }

    #[test]
    fn test_liquidity_trend() {
        assert_eq!(get_liquidity_trend(15_000_000.0), "increasing");
        assert_eq!(get_liquidity_trend(5_000_000.0), "stable");
        assert_eq!(get_liquidity_trend(500_000.0), "decreasing");
    }

    #[test]
    fn test_extract_asset_pair_regular_payment_native() {
        let payment = crate::rpc::Payment {
            id: "test_1".to_string(),
            paging_token: "token_1".to_string(),
            transaction_hash: "hash_1".to_string(),
            source_account: "GTEST".to_string(),
            destination: "GDEST".to_string(),
            asset_type: "native".to_string(),
            asset_code: None,
            asset_issuer: None,
            amount: "100.0".to_string(),
            created_at: "2026-01-01T00:00:00Z".to_string(),
            operation_type: Some("payment".to_string()),
            source_asset_type: None,
            source_asset_code: None,
            source_asset_issuer: None,
            source_amount: None,
            from: Some("GTEST".to_string()),
            to: Some("GDEST".to_string()),
            asset_balance_changes: None,
            transaction_successful: None,
        };

        let pair = extract_asset_pair_from_payment(&payment)
            .expect("extract_asset_pair_from_payment should succeed for this fixture");
        assert_eq!(pair.source_asset, "XLM:native");
        assert_eq!(pair.destination_asset, "XLM:native");
        assert_eq!(pair.to_corridor_key(), "XLM:native->XLM:native");
    }

    #[test]
    fn test_extract_asset_pair_regular_payment_issued_asset() {
        let payment = crate::rpc::Payment {
            id: "test_2".to_string(),
            paging_token: "token_2".to_string(),
            transaction_hash: "hash_2".to_string(),
            source_account: "GTEST".to_string(),
            destination: "GDEST".to_string(),
            asset_type: "credit_alphanum4".to_string(),
            asset_code: Some("USDC".to_string()),
            asset_issuer: Some("GISSUER".to_string()),
            amount: "100.0".to_string(),
            created_at: "2026-01-01T00:00:00Z".to_string(),
            operation_type: Some("payment".to_string()),
            source_asset_type: None,
            source_asset_code: None,
            source_asset_issuer: None,
            source_amount: None,
            from: Some("GTEST".to_string()),
            to: Some("GDEST".to_string()),
            asset_balance_changes: None,
            transaction_successful: None,
        };

        let pair = extract_asset_pair_from_payment(&payment)
            .expect("extract_asset_pair_from_payment should succeed for this fixture");
        assert_eq!(pair.source_asset, "USDC:GISSUER");
        assert_eq!(pair.destination_asset, "USDC:GISSUER");
        assert_eq!(pair.to_corridor_key(), "USDC:GISSUER->USDC:GISSUER");
    }

    #[test]
    fn test_extract_asset_pair_path_payment_cross_asset() {
        let payment = crate::rpc::Payment {
            id: "test_3".to_string(),
            paging_token: "token_3".to_string(),
            transaction_hash: "hash_3".to_string(),
            source_account: "GTEST".to_string(),
            destination: "GDEST".to_string(),
            asset_type: "credit_alphanum4".to_string(),
            asset_code: Some("EUR".to_string()),
            asset_issuer: Some("GEURISSUER".to_string()),
            amount: "100.0".to_string(),
            created_at: "2026-01-01T00:00:00Z".to_string(),
            operation_type: Some("path_payment_strict_send".to_string()),
            source_asset_type: Some("credit_alphanum4".to_string()),
            source_asset_code: Some("USD".to_string()),
            source_asset_issuer: Some("GUSDISSUER".to_string()),
            source_amount: Some("105.0".to_string()),
            from: Some("GTEST".to_string()),
            to: Some("GDEST".to_string()),
            asset_balance_changes: None,
            transaction_successful: None,
        };

        let pair = extract_asset_pair_from_payment(&payment)
            .expect("extract_asset_pair_from_payment should succeed for this fixture");
        assert_eq!(pair.source_asset, "USD:GUSDISSUER");
        assert_eq!(pair.destination_asset, "EUR:GEURISSUER");
        assert_eq!(pair.to_corridor_key(), "USD:GUSDISSUER->EUR:GEURISSUER");
    }

    #[test]
    fn test_extract_asset_pair_path_payment_native_to_issued() {
        let payment = crate::rpc::Payment {
            id: "test_4".to_string(),
            paging_token: "token_4".to_string(),
            transaction_hash: "hash_4".to_string(),
            source_account: "GTEST".to_string(),
            destination: "GDEST".to_string(),
            asset_type: "credit_alphanum4".to_string(),
            asset_code: Some("USDC".to_string()),
            asset_issuer: Some("GISSUER".to_string()),
            amount: "100.0".to_string(),
            created_at: "2026-01-01T00:00:00Z".to_string(),
            operation_type: Some("path_payment_strict_receive".to_string()),
            source_asset_type: Some("native".to_string()),
            source_asset_code: None,
            source_asset_issuer: None,
            source_amount: Some("150.0".to_string()),
            from: Some("GTEST".to_string()),
            to: Some("GDEST".to_string()),
            asset_balance_changes: None,
            transaction_successful: None,
        };

        let pair = extract_asset_pair_from_payment(&payment)
            .expect("extract_asset_pair_from_payment should succeed for this fixture");
        assert_eq!(pair.source_asset, "XLM:native");
        assert_eq!(pair.destination_asset, "USDC:GISSUER");
        assert_eq!(pair.to_corridor_key(), "XLM:native->USDC:GISSUER");
    }

    #[test]
    fn test_extract_asset_pair_path_payment_issued_to_native() {
        let payment = crate::rpc::Payment {
            id: "test_5".to_string(),
            paging_token: "token_5".to_string(),
            transaction_hash: "hash_5".to_string(),
            source_account: "GTEST".to_string(),
            destination: "GDEST".to_string(),
            asset_type: "native".to_string(),
            asset_code: None,
            asset_issuer: None,
            amount: "100.0".to_string(),
            created_at: "2026-01-01T00:00:00Z".to_string(),
            operation_type: Some("path_payment_strict_send".to_string()),
            source_asset_type: Some("credit_alphanum4".to_string()),
            source_asset_code: Some("BRL".to_string()),
            source_asset_issuer: Some("GBRLISSUER".to_string()),
            source_amount: Some("500.0".to_string()),
            from: Some("GTEST".to_string()),
            to: Some("GDEST".to_string()),
            asset_balance_changes: None,
            transaction_successful: None,
        };

        let pair = extract_asset_pair_from_payment(&payment)
            .expect("extract_asset_pair_from_payment should succeed for this fixture");
        assert_eq!(pair.source_asset, "BRL:GBRLISSUER");
        assert_eq!(pair.destination_asset, "XLM:native");
        assert_eq!(pair.to_corridor_key(), "BRL:GBRLISSUER->XLM:native");
    }

    #[test]
    fn test_extract_asset_pair_missing_operation_type() {
        // Should default to regular payment behavior
        let payment = crate::rpc::Payment {
            id: "test_6".to_string(),
            paging_token: "token_6".to_string(),
            transaction_hash: "hash_6".to_string(),
            source_account: "GTEST".to_string(),
            destination: "GDEST".to_string(),
            asset_type: "credit_alphanum4".to_string(),
            asset_code: Some("NGNT".to_string()),
            asset_issuer: Some("GNGNTISSUER".to_string()),
            amount: "100.0".to_string(),
            created_at: "2026-01-01T00:00:00Z".to_string(),
            operation_type: None,
            source_asset_type: None,
            source_asset_code: None,
            source_asset_issuer: None,
            source_amount: None,
            from: Some("GTEST".to_string()),
            to: Some("GDEST".to_string()),
            asset_balance_changes: None,
            transaction_successful: None,
        };

        let pair = extract_asset_pair_from_payment(&payment)
            .expect("extract_asset_pair_from_payment should succeed for this fixture");
        assert_eq!(pair.source_asset, "NGNT:GNGNTISSUER");
        assert_eq!(pair.destination_asset, "NGNT:GNGNTISSUER");
    }

    #[test]
    fn test_calculate_historical_success_rate_empty() {
        let payments = vec![];
        let result = calculate_historical_success_rate(&payments);
        assert_eq!(result.len(), 0);
    }

    #[test]
    fn test_calculate_historical_success_rate_single_day() {
        let payment = crate::rpc::Payment {
            id: "test_1".to_string(),
            paging_token: "token_1".to_string(),
            transaction_hash: "hash_1".to_string(),
            source_account: "GTEST".to_string(),
            destination: "GDEST".to_string(),
            asset_type: "native".to_string(),
            asset_code: None,
            asset_issuer: None,
            amount: "100.0".to_string(),
            created_at: "2026-01-15T10:00:00Z".to_string(),
            operation_type: Some("payment".to_string()),
            source_asset_type: None,
            source_asset_code: None,
            source_asset_issuer: None,
            source_amount: None,
            from: Some("GTEST".to_string()),
            to: Some("GDEST".to_string()),
            asset_balance_changes: None,
            transaction_successful: None,
        };

        let payments = vec![&payment];
        let result = calculate_historical_success_rate(&payments);

        assert!(!result.is_empty());
        assert!(result[0].success_rate == 100.0);
        assert_eq!(result[0].attempts, 1);
        assert!(result[0].timestamp.contains("2026-01-15"));
    }

    #[test]
    fn historical_success_rate_is_hourly_and_counts_failures() {
        let payment = crate::rpc::Payment {
            id: "test_1".to_string(),
            paging_token: "token_1".to_string(),
            transaction_hash: "hash_1".to_string(),
            source_account: "GTEST".to_string(),
            destination: "GDEST".to_string(),
            asset_type: "native".to_string(),
            asset_code: None,
            asset_issuer: None,
            amount: "100.0".to_string(),
            created_at: "2026-01-15T10:00:00Z".to_string(),
            operation_type: Some("payment".to_string()),
            source_asset_type: None,
            source_asset_code: None,
            source_asset_issuer: None,
            source_amount: None,
            from: Some("GTEST".to_string()),
            to: Some("GDEST".to_string()),
            asset_balance_changes: None,
            transaction_successful: None,
        };

        let failed = crate::rpc::Payment {
            transaction_successful: Some(false),
            ..payment.clone()
        };
        let later = crate::rpc::Payment {
            created_at: "2026-01-15T11:30:00Z".to_string(),
            ..payment.clone()
        };
        let payments = vec![&payment, &failed, &later];
        let result = calculate_historical_success_rate(&payments);

        // One point per hour, failures counted.
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].attempts, 2);
        assert!((result[0].success_rate - 50.0).abs() < f64::EPSILON);
        assert_eq!(result[1].timestamp, "2026-01-15T11:00:00Z");

        let trends = calculate_liquidity_trends(&payments, Some(2.0));
        assert!((trends[0].volume_24h_usd - 200.0).abs() < f64::EPSILON);
        assert!(calculate_liquidity_trends(&payments, None).is_empty());
    }

    #[test]
    fn test_calculate_liquidity_trends_empty() {
        let payments = vec![];
        let result = calculate_liquidity_trends(&payments, Some(1.0));
        assert_eq!(result.len(), 0);
    }

    #[test]
    fn test_find_related_corridors_same_source() {
        let target = "USDC:GISSUER->XLM:native";
        let corridors = vec![
            CorridorResponse {
                id: "USDC:GISSUER->XLM:native".to_string(),
                source_asset: "USDC".to_string(),
                destination_asset: "XLM".to_string(),
                success_rate: 100.0,
                total_attempts: 100,
                successful_payments: 100,
                failed_payments: 0,
                average_latency_ms: 400.0,
                median_latency_ms: 300.0,
                p95_latency_ms: 1000.0,
                p99_latency_ms: 1200.0,
                liquidity_depth_usd: 1_000_000.0,
                liquidity_volume_24h_usd: 100_000.0,
                liquidity_trend: "stable".to_string(),
                health_score: 95.0,
                last_updated: "2026-01-15T10:00:00Z".to_string(),
            },
            CorridorResponse {
                id: "USDC:GISSUER->EUR:GEURISSUER".to_string(),
                source_asset: "USDC".to_string(),
                destination_asset: "EUR".to_string(),
                success_rate: 99.0,
                total_attempts: 90,
                successful_payments: 89,
                failed_payments: 1,
                average_latency_ms: 420.0,
                median_latency_ms: 310.0,
                p95_latency_ms: 1050.0,
                p99_latency_ms: 1250.0,
                liquidity_depth_usd: 900_000.0,
                liquidity_volume_24h_usd: 90000.0,
                liquidity_trend: "stable".to_string(),
                health_score: 94.0,
                last_updated: "2026-01-15T10:00:00Z".to_string(),
            },
        ];

        let related = find_related_corridors(target, &corridors);
        assert!(related.is_some());
        let related_corridors =
            related.expect("related corridors should be Some after asserting is_some");
        assert!(related_corridors.len() >= 2); // At least target and one related
    }
}
