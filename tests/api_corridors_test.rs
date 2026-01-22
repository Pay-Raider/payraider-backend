use axum::{
    body::Body,
    http::{Request, StatusCode},
    Router,
};
use serde_json::Value;
use sqlx::PgPool;
use std::sync::Arc;
use tower::ServiceExt;
use uuid::Uuid;

use backend::api::corridors::{get_corridors, get_corridor_by_asset_pair};
use backend::database::Database;
use backend::models::corridor::CorridorMetrics;

async fn setup_test_db() -> PgPool {
    let database_url = std::env::var("TEST_DATABASE_URL")
        .unwrap_or_else(|_| "postgresql://postgres:password@localhost:5432/stellar_insights_test".to_string());
    
    let pool = sqlx::PgPool::connect(&database_url).await.unwrap();
    
    // Run migrations
    sqlx::migrate!("./migrations").run(&pool).await.unwrap();
    
    // Clean up existing test data
    sqlx::query("DELETE FROM corridor_metrics").execute(&pool).await.unwrap();
    
    pool
}

async fn create_test_corridor_metrics(db: &Database) -> Vec<CorridorMetrics> {
    let mut corridors = Vec::new();
    
    // Create test corridor 1: USDC->EURC with high success rate
    let corridor1 = db.create_or_update_corridor_metrics(
        "EURC:issuer2->USDC:issuer1",
        "EURC",
        "issuer2", 
        "USDC",
        "issuer1",
        1000,
        950,
        50,
        1000000.0,
    ).await.unwrap();
    corridors.push(corridor1);
    
    // Create test corridor 2: BTC->ETH with medium success rate
    let corridor2 = db.create_or_update_corridor_metrics(
        "BTC:issuer3->ETH:issuer4",
        "BTC",
        "issuer3",
        "ETH", 
        "issuer4",
        500,
        400,
        100,
        2000000.0,
    ).await.unwrap();
    corridors.push(corridor2);
    
    // Create test corridor 3: XLM->USD with low success rate but high volume
    let corridor3 = db.create_or_update_corridor_metrics(
        "USD:issuer6->XLM:issuer5",
        "USD",
        "issuer6",
        "XLM",
        "issuer5", 
        2000,
        1400,
        600,
        5000000.0,
    ).await.unwrap();
    corridors.push(corridor3);
    
    corridors
}

fn create_test_router(db: Arc<Database>) -> Router {
    Router::new()
        .route("/api/corridors", axum::routing::get(get_corridors))
        .route("/api/corridors/:asset_pair", axum::routing::get(get_corridor_by_asset_pair))
        .with_state(db)
}

#[tokio::test]
async fn test_get_corridors_success() {
    let pool = setup_test_db().await;
    let db = Arc::new(Database::new(pool));
    let _test_corridors = create_test_corridor_metrics(&db).await;
    
    let app = create_test_router(db);
    
    let request = Request::builder()
        .uri("/api/corridors")
        .body(Body::empty())
        .unwrap();
    
    let response = app.oneshot(request).await.unwrap();
    
    assert_eq!(response.status(), StatusCode::OK);
    
    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    
    assert!(json["corridors"].is_array());
    assert_eq!(json["corridors"].as_array().unwrap().len(), 3);
    assert_eq!(json["total"].as_u64().unwrap(), 3);
    
    // Verify response structure
    let first_corridor = &json["corridors"][0];
    assert!(first_corridor["asset_pair"].is_string());
    assert!(first_corridor["asset_a_code"].is_string());
    assert!(first_corridor["asset_a_issuer"].is_string());
    assert!(first_corridor["asset_b_code"].is_string());
    assert!(first_corridor["asset_b_issuer"].is_string());
    assert!(first_corridor["success_rate"].is_number());
    assert!(first_corridor["total_transactions"].is_number());
    assert!(first_corridor["successful_transactions"].is_number());
    assert!(first_corridor["failed_transactions"].is_number());
    assert!(first_corridor["volume_usd"].is_number());
    assert!(first_corridor["last_updated"].is_string());
}

#[tokio::test]
async fn test_get_corridors_sorted_by_success_rate() {
    let pool = setup_test_db().await;
    let db = Arc::new(Database::new(pool));
    let _test_corridors = create_test_corridor_metrics(&db).await;
    
    let app = create_test_router(db);
    
    let request = Request::builder()
        .uri("/api/corridors?sort_by=success_rate")
        .body(Body::empty())
        .unwrap();
    
    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    
    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    
    let corridors = json["corridors"].as_array().unwrap();
    assert_eq!(corridors.len(), 3);
    
    // Verify sorting by success rate (descending)
    let success_rates: Vec<f64> = corridors
        .iter()
        .map(|c| c["success_rate"].as_f64().unwrap())
        .collect();
    
    for i in 1..success_rates.len() {
        assert!(success_rates[i-1] >= success_rates[i], 
            "Success rates should be in descending order: {} >= {}", 
            success_rates[i-1], success_rates[i]);
    }
}

#[tokio::test]
async fn test_get_corridors_sorted_by_volume() {
    let pool = setup_test_db().await;
    let db = Arc::new(Database::new(pool));
    let _test_corridors = create_test_corridor_metrics(&db).await;
    
    let app = create_test_router(db);
    
    let request = Request::builder()
        .uri("/api/corridors?sort_by=volume")
        .body(Body::empty())
        .unwrap();
    
    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    
    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    
    let corridors = json["corridors"].as_array().unwrap();
    assert_eq!(corridors.len(), 3);
    
    // Verify sorting by volume (descending)
    let volumes: Vec<f64> = corridors
        .iter()
        .map(|c| c["volume_usd"].as_f64().unwrap())
        .collect();
    
    for i in 1..volumes.len() {
        assert!(volumes[i-1] >= volumes[i], 
            "Volumes should be in descending order: {} >= {}", 
            volumes[i-1], volumes[i]);
    }
}

#[tokio::test]
async fn test_get_corridors_with_pagination() {
    let pool = setup_test_db().await;
    let db = Arc::new(Database::new(pool));
    let _test_corridors = create_test_corridor_metrics(&db).await;
    
    let app = create_test_router(db);
    
    let request = Request::builder()
        .uri("/api/corridors?limit=2&offset=1")
        .body(Body::empty())
        .unwrap();
    
    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    
    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    
    let corridors = json["corridors"].as_array().unwrap();
    assert_eq!(corridors.len(), 2); // Limited to 2 results
    assert_eq!(json["total"].as_u64().unwrap(), 2);
}

#[tokio::test]
async fn test_get_corridor_by_asset_pair_success() {
    let pool = setup_test_db().await;
    let db = Arc::new(Database::new(pool));
    let test_corridors = create_test_corridor_metrics(&db).await;
    
    let app = create_test_router(db);
    
    // Test with normalized asset pair format
    let asset_pair = "USDC:issuer1->EURC:issuer2";
    let request = Request::builder()
        .uri(&format!("/api/corridors/{}", asset_pair))
        .body(Body::empty())
        .unwrap();
    
    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    
    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    
    // Verify response structure
    assert!(json["asset_pair"].is_string());
    assert!(json["success_rate"].is_number());
    assert!(json["total_transactions"].is_number());
    assert!(json["volume_usd"].is_number());
    
    // Verify the correct corridor was returned
    let expected_corridor = &test_corridors[0]; // EURC->USDC corridor
    assert_eq!(json["success_rate"].as_f64().unwrap(), expected_corridor.success_rate);
    assert_eq!(json["total_transactions"].as_i64().unwrap(), expected_corridor.total_transactions);
}

#[tokio::test]
async fn test_get_corridor_by_asset_pair_with_spaces() {
    let pool = setup_test_db().await;
    let db = Arc::new(Database::new(pool));
    let _test_corridors = create_test_corridor_metrics(&db).await;
    
    let app = create_test_router(db);
    
    // Test with spaces in asset pair format
    let asset_pair = "USDC:issuer1 -> EURC:issuer2";
    let encoded_asset_pair = urlencoding::encode(asset_pair);
    let request = Request::builder()
        .uri(&format!("/api/corridors/{}", encoded_asset_pair))
        .body(Body::empty())
        .unwrap();
    
    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_get_corridor_by_asset_pair_not_found() {
    let pool = setup_test_db().await;
    let db = Arc::new(Database::new(pool));
    let _test_corridors = create_test_corridor_metrics(&db).await;
    
    let app = create_test_router(db);
    
    let asset_pair = "NONEXISTENT:issuer->FAKE:issuer";
    let request = Request::builder()
        .uri(&format!("/api/corridors/{}", asset_pair))
        .body(Body::empty())
        .unwrap();
    
    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    
    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    
    assert!(json["error"].is_string());
    assert!(json["error"].as_str().unwrap().contains("not found"));
}

#[tokio::test]
async fn test_get_corridor_by_asset_pair_invalid_format() {
    let pool = setup_test_db().await;
    let db = Arc::new(Database::new(pool));
    
    let app = create_test_router(db);
    
    let invalid_asset_pair = "USDC-EURC"; // Missing issuer and wrong separator
    let request = Request::builder()
        .uri(&format!("/api/corridors/{}", invalid_asset_pair))
        .body(Body::empty())
        .unwrap();
    
    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    
    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    
    assert!(json["error"].is_string());
    assert!(json["error"].as_str().unwrap().contains("Invalid asset pair format"));
}

#[tokio::test]
async fn test_get_corridors_empty_dataset() {
    let pool = setup_test_db().await;
    let db = Arc::new(Database::new(pool));
    // Don't create any test data
    
    let app = create_test_router(db);
    
    let request = Request::builder()
        .uri("/api/corridors")
        .body(Body::empty())
        .unwrap();
    
    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    
    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    
    assert!(json["corridors"].is_array());
    assert_eq!(json["corridors"].as_array().unwrap().len(), 0);
    assert_eq!(json["total"].as_u64().unwrap(), 0);
}

#[tokio::test]
async fn test_corridor_response_json_structure() {
    let pool = setup_test_db().await;
    let db = Arc::new(Database::new(pool));
    let _test_corridors = create_test_corridor_metrics(&db).await;
    
    let app = create_test_router(db);
    
    let request = Request::builder()
        .uri("/api/corridors?limit=1")
        .body(Body::empty())
        .unwrap();
    
    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    
    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    
    // Verify top-level structure
    assert!(json.is_object());
    assert!(json["corridors"].is_array());
    assert!(json["total"].is_number());
    
    // Verify corridor object structure
    let corridor = &json["corridors"][0];
    let required_fields = [
        "asset_pair", "asset_a_code", "asset_a_issuer", "asset_b_code", "asset_b_issuer",
        "success_rate", "total_transactions", "successful_transactions", "failed_transactions",
        "volume_usd", "last_updated"
    ];
    
    for field in &required_fields {
        assert!(corridor[field].is_string() || corridor[field].is_number(), 
            "Field '{}' should be present and have correct type", field);
    }
    
    // Verify asset_pair format
    let asset_pair = corridor["asset_pair"].as_str().unwrap();
    assert!(asset_pair.contains(" -> "), "Asset pair should contain ' -> ' separator");
    assert!(asset_pair.contains(":"), "Asset pair should contain ':' for issuer separation");
}

#[tokio::test]
async fn test_corridor_metrics_calculation() {
    let pool = setup_test_db().await;
    let db = Arc::new(Database::new(pool));
    
    // Create corridor with known metrics
    let corridor = db.create_or_update_corridor_metrics(
        "TEST:issuer1->DEMO:issuer2",
        "TEST",
        "issuer1",
        "DEMO",
        "issuer2",
        100, // total
        80,  // successful
        20,  // failed
        50000.0, // volume
    ).await.unwrap();
    
    let app = create_test_router(db);
    
    let request = Request::builder()
        .uri("/api/corridors/TEST:issuer1->DEMO:issuer2")
        .body(Body::empty())
        .unwrap();
    
    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    
    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    
    // Verify calculated metrics
    assert_eq!(json["success_rate"].as_f64().unwrap(), 80.0);
    assert_eq!(json["total_transactions"].as_i64().unwrap(), 100);
    assert_eq!(json["successful_transactions"].as_i64().unwrap(), 80);
    assert_eq!(json["failed_transactions"].as_i64().unwrap(), 20);
    assert_eq!(json["volume_usd"].as_f64().unwrap(), 50000.0);
}