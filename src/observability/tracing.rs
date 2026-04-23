use anyhow::Result;
use axum::{body::Body, extract::Request, middleware::Next, response::Response};
use opentelemetry::global;
use opentelemetry::propagation::TextMapPropagator;
use opentelemetry::trace::TracerProvider;
use opentelemetry::KeyValue;
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::propagation::TraceContextPropagator;
use opentelemetry_sdk::resource::Resource;
use opentelemetry_sdk::runtime;
use opentelemetry_sdk::trace::Config;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_opentelemetry::OpenTelemetrySpanExt;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, Layer};

const MAX_LOG_FILES: usize = 30;

fn init_otel_tracer(service_name: &str) -> Result<opentelemetry_sdk::trace::Tracer> {
    // HTTP/protobuf OTLP on 4318; OTLP 0.17+ avoids pulling `tonic`'s legacy `axum` into this crate graph.
    let endpoint = std::env::var("OTEL_EXPORTER_OTLP_ENDPOINT").unwrap_or_else(|_| {
        "http://localhost:4318/v1/traces".to_string()
    });

    let resource = Resource::new([KeyValue::new(
        "service.name",
        service_name.to_string(),
    )]);

    let provider = opentelemetry_otlp::new_pipeline()
        .tracing()
        .with_exporter(
            opentelemetry_otlp::new_exporter()
                .http()
                .with_endpoint(endpoint),
        )
        .with_trace_config(Config::default().with_resource(resource))
        .install_batch(runtime::Tokio)?;

    global::set_tracer_provider(provider.clone());
    Ok(provider.tracer("stellar-insights-backend"))
}

/// Initialize tracing. When `LOG_DIR` is set, logs are also written to a rotating file
/// (daily rotation, up to 30 files retained). The returned guard must be held for the
/// process lifetime so that file logs are flushed; drop it only at shutdown.
pub fn init_tracing(service_name: &str) -> Result<Option<WorkerGuard>> {
    // Register W3C TraceContext as the global propagator so that
    // `traceparent` / `tracestate` headers are used for context propagation.
    global::set_text_map_propagator(TraceContextPropagator::new());

    let _ = tracing_log::LogTracer::init();

    let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| "backend=info,tower_http=info".into());

    let log_format = std::env::var("LOG_FORMAT").unwrap_or_else(|_| "json".to_string());
    let use_json = log_format.eq_ignore_ascii_case("json");

    let otel_enabled = std::env::var("OTEL_ENABLED")
        .map(|v| v.eq_ignore_ascii_case("true"))
        .unwrap_or(true);

    // Optional rotating file appender
    let log_dir = std::env::var("LOG_DIR").ok();
    let (file_writer, file_guard) = if let Some(ref dir) = log_dir {
        std::fs::create_dir_all(dir)?;
        let appender = RollingFileAppender::builder()
            .rotation(Rotation::DAILY)
            .filename_prefix("stellar-insights")
            .filename_suffix("log")
            .max_log_files(MAX_LOG_FILES)
            .build(dir)?;
        let (nb, guard) = tracing_appender::non_blocking(appender);
        (Some(nb), Some(guard))
    } else {
        (None, None)
    };

    // OTel layer must be registered on `registry()` first so `LookupSpan` bounds are satisfied.
    let stdout = std::io::stdout;

    if otel_enabled {
        let otel_tracer = init_otel_tracer(service_name)?;
        let otel_layer = tracing_opentelemetry::layer().with_tracer(otel_tracer);
        let base = tracing_subscriber::registry()
            .with(otel_layer)
            .with(env_filter);

        match (use_json, file_writer) {
            (true, Some(w)) => {
                let stdout_layer = tracing_subscriber::fmt::layer()
                    .json()
                    .with_writer(stdout)
                    .with_target(true)
                    .with_level(true)
                    .boxed();
                let file_layer = tracing_subscriber::fmt::layer()
                    .json()
                    .with_writer(w)
                    .with_target(true)
                    .with_level(true)
                    .boxed();
                base.with(stdout_layer).with(file_layer).init();
            }
            (true, None) => {
                let stdout_layer = tracing_subscriber::fmt::layer()
                    .json()
                    .with_writer(stdout)
                    .with_target(true)
                    .with_level(true)
                    .boxed();
                base.with(stdout_layer).init();
            }
            (false, Some(w)) => {
                let stdout_layer = tracing_subscriber::fmt::layer()
                    .with_writer(stdout)
                    .with_target(true)
                    .with_level(true)
                    .boxed();
                let file_layer = tracing_subscriber::fmt::layer()
                    .with_writer(w)
                    .with_target(true)
                    .with_level(true)
                    .boxed();
                base.with(stdout_layer).with(file_layer).init();
            }
            (false, None) => {
                let stdout_layer = tracing_subscriber::fmt::layer()
                    .with_writer(stdout)
                    .with_target(true)
                    .with_level(true)
                    .boxed();
                base.with(stdout_layer).init();
            }
        }
        tracing::info!("OpenTelemetry tracing enabled");
    } else {
        let base = tracing_subscriber::registry().with(env_filter);
        match (use_json, file_writer) {
            (true, Some(w)) => {
                let stdout_layer = tracing_subscriber::fmt::layer()
                    .json()
                    .with_writer(stdout)
                    .with_target(true)
                    .with_level(true)
                    .boxed();
                let file_layer = tracing_subscriber::fmt::layer()
                    .json()
                    .with_writer(w)
                    .with_target(true)
                    .with_level(true)
                    .boxed();
                base.with(stdout_layer).with(file_layer).init();
            }
            (true, None) => {
                let stdout_layer = tracing_subscriber::fmt::layer()
                    .json()
                    .with_writer(stdout)
                    .with_target(true)
                    .with_level(true)
                    .boxed();
                base.with(stdout_layer).init();
            }
            (false, Some(w)) => {
                let stdout_layer = tracing_subscriber::fmt::layer()
                    .with_writer(stdout)
                    .with_target(true)
                    .with_level(true)
                    .boxed();
                let file_layer = tracing_subscriber::fmt::layer()
                    .with_writer(w)
                    .with_target(true)
                    .with_level(true)
                    .boxed();
                base.with(stdout_layer).with(file_layer).init();
            }
            (false, None) => {
                let stdout_layer = tracing_subscriber::fmt::layer()
                    .with_writer(stdout)
                    .with_target(true)
                    .with_level(true)
                    .boxed();
                base.with(stdout_layer).init();
            }
        }
    }

    Ok(file_guard)
}

pub fn shutdown_tracing() {
    global::shutdown_tracer_provider();
}

/// Axum middleware that extracts W3C TraceContext headers (`traceparent`, `tracestate`)
/// from incoming requests and sets them as the parent context on the current span.
///
/// This must be placed *after* `TraceLayer` in the middleware stack so that a span
/// already exists when this middleware runs.
pub async fn trace_propagation_middleware(req: Request<Body>, next: Next) -> Response {
    // Build a simple header-map view that the OTel propagator can read from.
    let headers = req.headers();
    let carrier: std::collections::HashMap<String, String> = headers
        .iter()
        .filter_map(|(name, value)| {
            value
                .to_str()
                .ok()
                .map(|v| (name.as_str().to_owned(), v.to_owned()))
        })
        .collect();

    // Extract the remote context using the globally registered propagator.
    let propagator = TraceContextPropagator::new();
    let parent_cx = propagator.extract(&carrier);

    // Attach the remote context to the current tracing span so that child spans
    // created during this request are correctly parented.
    let span = tracing::Span::current();
    span.set_parent(parent_cx);

    next.run(req).await
}

/// Inject the current trace context into an outbound `reqwest::RequestBuilder`.
///
/// Call this on every outbound HTTP request to propagate `traceparent` /
/// `tracestate` headers to downstream services.
///
/// # Example
/// ```rust
/// let response = inject_trace_context(client.get(&url)).send().await?;
/// ```
pub fn inject_trace_context(builder: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
    let mut carrier = std::collections::HashMap::new();
    let propagator = TraceContextPropagator::new();
    let cx = opentelemetry::Context::current();
    propagator.inject_context(&cx, &mut carrier);

    let mut builder = builder;
    for (key, value) in carrier {
        builder = builder.header(key, value);
    }
    builder
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::Body,
        http::{Request, StatusCode},
        middleware,
        routing::get,
        Router,
    };
    use tower::ServiceExt;

    #[tokio::test]
    async fn propagation_middleware_does_not_break_requests() {
        let app = Router::new()
            .route("/ping", get(|| async { StatusCode::OK }))
            .layer(middleware::from_fn(trace_propagation_middleware));

        let response = app
            .oneshot(Request::builder().uri("/ping").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn propagation_middleware_accepts_traceparent_header() {
        let app = Router::new()
            .route("/ping", get(|| async { StatusCode::OK }))
            .layer(middleware::from_fn(trace_propagation_middleware));

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/ping")
                    // Valid W3C traceparent header
                    .header(
                        "traceparent",
                        "00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01",
                    )
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }
}

/// Re-export redaction utilities for use throughout the application
pub use crate::logging::redaction::{
    redact_account, redact_amount, redact_email, redact_hash, redact_ip, redact_token,
    redact_user_id, Redacted,
};
