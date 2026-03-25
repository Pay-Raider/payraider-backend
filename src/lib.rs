#![allow(clippy::cast_precision_loss)]
#![allow(clippy::cast_possible_wrap)]
#![allow(clippy::cast_sign_loss)]
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::unwrap_used)]
#![allow(clippy::format_push_string)]
#![allow(clippy::unreadable_literal)]
#![allow(clippy::cognitive_complexity)]
#![allow(clippy::significant_drop_tightening)]
#![allow(clippy::unused_async)]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::too_many_lines)]
#![allow(clippy::module_name_repetitions)]

pub mod admin_audit_log;
pub mod alert_handlers;
pub mod alerts;
pub mod analytics;
pub mod api;
pub mod api_analytics_middleware;
pub mod api_v1_middleware;
pub mod monitor;

pub mod auth;
pub mod auth_middleware;
pub mod broadcast;
pub mod cache;
pub mod cache_invalidation;
// cache_middleware removed in favor of cache helper APIs
pub mod crypto;
pub mod database;

pub mod http_cache; // HTTP caching layer (ETag/conditional responses)
pub mod db;
pub mod email;
pub mod env_config;
pub mod error;
pub mod handlers;
pub mod ingestion;
pub mod ip_whitelist_middleware;
pub mod jobs;
pub mod logging;
pub mod ml;
pub mod ml_handlers;
pub mod models;
pub mod muxed;
pub mod request_signing_middleware;

pub mod network;
pub mod observability;
pub mod openapi;
pub mod rate_limit;
pub mod replay;
pub mod request_id;
pub mod services;
pub mod shutdown;
pub mod snapshot;
pub mod snapshot_handlers;
pub mod state;
pub mod validation;
pub mod vault;
pub mod webhooks;
pub mod websocket;

pub mod rpc;
pub mod rpc_handlers;
pub mod telegram;

#[cfg(test)]
mod ml_tests;
