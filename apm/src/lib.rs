// APM Module - Application Performance Monitoring for Stellar Insights
//
// This module provides comprehensive APM integration with OpenTelemetry,
// supporting multiple backends including Jaeger, New Relic, and Datadog.

pub mod apm;
pub mod middleware;
pub mod integration;

// Re-export main types for convenience
pub use apm::{ApmConfig, ApmManager, ApmMetrics, ApmPlatform};
pub use middleware::ApmMiddleware;
pub use integration::ApmIntegration;
