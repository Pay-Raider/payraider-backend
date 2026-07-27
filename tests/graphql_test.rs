//! Integration coverage for the GraphQL API surface (issue #1856).
//!
//! The REST v1/v2 routes have dedicated integration suites; this file gives the
//! GraphQL layer an equivalent so schema/resolver drift is caught by CI instead
//! of surfacing as a runtime 500.

use stellar_insights_backend::features::graphql_api::{GraphQLAPI, GraphQLAPIConfig};
use stellar_insights_backend::models::graphql_api::GraphQLRequest;

fn request(query: &str) -> GraphQLRequest {
    GraphQLRequest {
        query: query.to_string(),
        variables: None,
        operation_name: None,
    }
}

#[tokio::test]
async fn health_query_returns_ok_status() {
    let api = GraphQLAPI::new(GraphQLAPIConfig::default(), 0);

    let response = api
        .execute(request("{ health { status version } }"))
        .await
        .expect("health query should execute");

    assert!(response.success, "expected successful response");
    let data = response.data.expect("health query should return data");
    assert_eq!(data["health"]["status"], "ok");
    assert_eq!(data["health"]["version"], "1.0.0");
}

#[tokio::test]
async fn anchor_count_query_reflects_seeded_value() {
    let api = GraphQLAPI::new(GraphQLAPIConfig::default(), 42);

    let response = api
        .execute(request("{ anchorCount { count } }"))
        .await
        .expect("anchorCount query should execute");

    assert!(response.success);
    let data = response.data.expect("anchorCount should return data");
    assert_eq!(data["anchorCount"]["count"], 42);
}

#[tokio::test]
async fn invalid_query_is_rejected_without_panicking() {
    let api = GraphQLAPI::new(GraphQLAPIConfig::default(), 0);

    let result = api.execute(request("")).await;

    assert!(result.is_err(), "empty query should be rejected");
}

#[tokio::test]
async fn disabled_api_rejects_queries() {
    let config = GraphQLAPIConfig {
        enabled: false,
        ..GraphQLAPIConfig::default()
    };
    let api = GraphQLAPI::new(config, 0);

    let result = api.execute(request("{ health { status } }")).await;

    assert!(result.is_err(), "disabled API should not execute queries");
}

#[tokio::test]
async fn health_status_is_exposed_for_health_aggregation() {
    // /health aggregates this, so a broken GraphQL layer is visible there.
    let api = GraphQLAPI::new(GraphQLAPIConfig::default(), 0);

    let status = api.health_status();

    assert!(status.enabled);
    assert_eq!(status.endpoint, "/graphql");
    assert!(!status.version.is_empty());
}
