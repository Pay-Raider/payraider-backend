use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use serde_json::json;
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::sync::LazyLock;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tokio::sync::Mutex;

use crate::api::auth::{
    check_rate_limit_for_account, clear_failed_login_state, preflight_login_guards,
    record_failed_login, AuthApiError,
};
use crate::auth::sep10_simple::{ChallengeRequest, Sep10Service, VerificationRequest};
use crate::observability::metrics::record_auth_security_event;

const SEP10_CHALLENGE_LIMIT_PER_MINUTE: usize = 10;
static SEP10_CHALLENGE_WINDOWS: LazyLock<Mutex<HashMap<String, VecDeque<Instant>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

fn extract_client_ip(headers: &HeaderMap) -> String {
    headers
        .get("x-forwarded-for")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.split(',').next())
        .map(str::trim)
        .filter(|ip| !ip.is_empty())
        .map(str::to_string)
        .or_else(|| {
            headers
                .get("x-real-ip")
                .and_then(|value| value.to_str().ok())
                .map(str::trim)
                .filter(|ip| !ip.is_empty())
                .map(str::to_string)
        })
        .unwrap_or_else(|| "unknown".to_string())
}

async fn check_challenge_rate_limit(ip: &str) -> Option<u64> {
    let now = Instant::now();
    let window = Duration::from_secs(60);
    let mut buckets = SEP10_CHALLENGE_WINDOWS.lock().await;
    let entries = buckets.entry(ip.to_string()).or_insert_with(VecDeque::new);

    while let Some(oldest) = entries.front().copied() {
        if now.duration_since(oldest) >= window {
            entries.pop_front();
        } else {
            break;
        }
    }

    if entries.len() >= SEP10_CHALLENGE_LIMIT_PER_MINUTE {
        let retry_after_seconds = entries
            .front()
            .copied()
            .map(|first| {
                window
                    .saturating_sub(now.duration_since(first))
                    .as_secs()
                    .max(1)
            })
            .unwrap_or(60);
        return Some(retry_after_seconds);
    }

    entries.push_back(now);
    None
}

fn now_unix() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

/// Decode the base64 challenge transaction and return the `max_time`
/// (stored as `expires_at` during challenge generation).
fn extract_max_time(transaction: &str) -> Option<i64> {
    let bytes = BASE64.decode(transaction).ok()?;
    let json: serde_json::Value = serde_json::from_slice(&bytes).ok()?;
    json["expires_at"].as_i64()
}

/// Decode the base64 challenge transaction and return the client account it was issued to.
fn extract_client_account(transaction: &str) -> Option<String> {
    let bytes = BASE64.decode(transaction).ok()?;
    let json: serde_json::Value = serde_json::from_slice(&bytes).ok()?;
    json["client"].as_str().map(str::to_string)
}

/// GET /api/sep10/info - Get SEP-10 server information
#[utoipa::path(
    get,
    path = "/api/sep10/info",
    responses(
        (status = 200, description = "SEP-10 server information")
    ),
    tag = "SEP-10"
)]
pub async fn get_info(
    State(sep10_service): State<Arc<Sep10Service>>,
) -> Result<Response, Sep10ApiError> {
    let info = json!({
        "authentication_endpoint": "/api/sep10/auth",
        "network_passphrase": sep10_service.network_passphrase,
        "signing_key": sep10_service.server_public_key,
        "version": "1.0.0"
    });

    Ok((StatusCode::OK, Json(info)).into_response())
}

/// POST /api/sep10/auth - Request SEP-10 challenge transaction
#[utoipa::path(
    post,
    path = "/api/sep10/auth",
    request_body = ChallengeRequest,
    responses(
        (status = 200, description = "Challenge transaction generated"),
        (status = 400, description = "Challenge generation failed")
    ),
    tag = "SEP-10"
)]
pub async fn request_challenge(
    State(sep10_service): State<Arc<Sep10Service>>,
    headers: HeaderMap,
    Json(request): Json<ChallengeRequest>,
) -> Result<Response, Sep10ApiError> {
    let client_ip = extract_client_ip(&headers);
    if let Some(retry_after_seconds) = check_challenge_rate_limit(&client_ip).await {
        tracing::warn!(
            client_ip = %client_ip,
            retry_after_seconds,
            "SEP-10 challenge rate limit exceeded"
        );
        record_auth_security_event("sep10_challenge", "rate_limited");
        return Err(Sep10ApiError::RateLimited {
            retry_after_seconds,
        });
    }

    let response = sep10_service
        .generate_challenge(request)
        .await
        .map_err(|e| Sep10ApiError::ChallengeGenerationFailed(e.to_string()))?;

    Ok((StatusCode::OK, Json(response)).into_response())
}

/// POST /api/sep10/verify - Verify signed challenge transaction
#[utoipa::path(
    post,
    path = "/api/sep10/verify",
    request_body = VerificationRequest,
    responses(
        (status = 200, description = "Verification successful"),
        (status = 401, description = "Verification failed")
    ),
    tag = "SEP-10"
)]
pub async fn verify_challenge(
    State(sep10_service): State<Arc<Sep10Service>>,
    headers: HeaderMap,
    Json(request): Json<VerificationRequest>,
) -> Result<Response, Sep10ApiError> {
    // Same per-account rate limit, backoff, lockout and CAPTCHA escalation as the
    // login endpoint. Unreadable challenges carry no account, so they are keyed by IP.
    let client_ip = extract_client_ip(&headers);
    let account_key = format!(
        "sep10:{}",
        extract_client_account(&request.transaction).unwrap_or_else(|| format!("ip:{client_ip}"))
    );
    if let Some(retry_after_seconds) = check_rate_limit_for_account(&account_key).await {
        tracing::warn!(
            client_ip = %client_ip,
            retry_after_seconds,
            "SEP-10 token endpoint rate limit exceeded for account"
        );
        record_auth_security_event("sep10_verify", "rate_limited");
        return Err(Sep10ApiError::RateLimited {
            retry_after_seconds,
        });
    }
    preflight_login_guards(&account_key, &headers, &client_ip)
        .await
        .map_err(Sep10ApiError::Auth)?;

    // Enforce time bounds before any signature work: reject expired challenges
    // immediately with HTTP 401 rather than letting them reach service logic.
    let max_time = extract_max_time(&request.transaction).ok_or_else(|| {
        Sep10ApiError::VerificationFailed("Missing or unreadable time bounds".to_string())
    })?;

    if now_unix() >= max_time {
        tracing::warn!(max_time, "SEP-10 challenge submitted after expiry");
        return Err(Sep10ApiError::ChallengeExpired);
    }

    match sep10_service.verify_challenge(request).await {
        Ok(response) => {
            clear_failed_login_state(&account_key).await;
            Ok((StatusCode::OK, Json(response)).into_response())
        }
        Err(e) => {
            record_failed_login("sep10_verify", &account_key, &client_ip).await;
            Err(Sep10ApiError::VerificationFailed(e.to_string()))
        }
    }
}

/// POST /api/sep10/logout - Invalidate SEP-10 session
#[utoipa::path(
    post,
    path = "/api/sep10/logout",
    responses(
        (status = 200, description = "Logged out successfully"),
        (status = 500, description = "Logout failed")
    ),
    tag = "SEP-10"
)]
pub async fn logout(
    State(sep10_service): State<Arc<Sep10Service>>,
    axum::extract::Extension(token): axum::extract::Extension<String>,
) -> Result<Response, Sep10ApiError> {
    sep10_service
        .invalidate_session(&token)
        .await
        .map_err(|e| Sep10ApiError::LogoutFailed(e.to_string()))?;

    let body = json!({
        "message": "Logged out successfully"
    });

    Ok((StatusCode::OK, Json(body)).into_response())
}

/// SEP-10 API errors
#[derive(Debug)]
pub enum Sep10ApiError {
    ChallengeGenerationFailed(String),
    VerificationFailed(String),
    ChallengeExpired,
    LogoutFailed(String),
    RateLimited {
        retry_after_seconds: u64,
    },
    /// Lockout / backoff / CAPTCHA rejection from the shared auth guards.
    Auth(AuthApiError),
}

impl IntoResponse for Sep10ApiError {
    fn into_response(self) -> Response {
        let (status, message, retry_after) = match self {
            Self::Auth(err) => return err.into_response(),
            Self::ChallengeGenerationFailed(msg) => (
                StatusCode::BAD_REQUEST,
                format!("Challenge generation failed: {msg}"),
                None,
            ),
            Self::VerificationFailed(msg) => (
                StatusCode::UNAUTHORIZED,
                format!("Verification failed: {msg}"),
                None,
            ),
            Self::ChallengeExpired => (
                StatusCode::UNAUTHORIZED,
                "Challenge transaction has expired".to_string(),
                None,
            ),
            Self::LogoutFailed(msg) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Logout failed: {msg}"),
                None,
            ),
            Self::RateLimited {
                retry_after_seconds,
            } => (
                StatusCode::TOO_MANY_REQUESTS,
                format!(
                    "Too many SEP-10 authentication requests. Retry after {retry_after_seconds} seconds"
                ),
                Some(retry_after_seconds),
            ),
        };

        let body = json!({
            "error": message,
        });
        let mut response = (status, Json(body)).into_response();
        if let Some(retry_after_seconds) = retry_after {
            if let Ok(value) = axum::http::HeaderValue::from_str(&retry_after_seconds.to_string()) {
                response
                    .headers_mut()
                    .insert(axum::http::header::RETRY_AFTER, value);
            }
        }
        response
    }
}

/// The account a SEP-10 session belongs to. Lets clients check that a
/// stored token is still valid (sessions end on expiry, logout, and, without
/// Redis, on a backend restart).
pub async fn get_session(
    axum::extract::Extension(user): axum::extract::Extension<crate::auth::Sep10User>,
) -> Response {
    Json(json!({ "account": user.account })).into_response()
}

/// Create SEP-10 routes
pub fn routes(sep10_service: Arc<Sep10Service>) -> Router {
    // logout and session read the token the auth middleware attaches; without
    // it logout failed with "Missing request extension" (500).
    let authenticated = Router::new()
        .route("/api/sep10/logout", post(logout))
        .route("/api/sep10/session", get(get_session))
        .route_layer(axum::middleware::from_fn_with_state(
            sep10_service.clone(),
            crate::auth::sep10_auth_middleware,
        ));

    Router::new()
        .route("/api/sep10/info", get(get_info))
        .route("/api/sep10/auth", post(request_challenge))
        .route("/api/sep10/verify", post(verify_challenge))
        .merge(authenticated)
        .with_state(sep10_service)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::Request;
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    async fn call(app: &Router, req: Request<Body>) -> (StatusCode, serde_json::Value) {
        let res = app.clone().oneshot(req).await.unwrap();
        let status = res.status();
        let bytes = res.into_body().collect().await.unwrap().to_bytes();
        (
            status,
            serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null),
        )
    }

    fn post(uri: &str, body: serde_json::Value, token: Option<&str>) -> Request<Body> {
        let mut req = Request::post(uri).header("content-type", "application/json");
        if let Some(t) = token {
            req = req.header("authorization", format!("Bearer {t}"));
        }
        req.body(Body::from(body.to_string())).unwrap()
    }

    #[tokio::test]
    async fn sign_in_session_and_logout_round_trip() {
        use ed25519_dalek::Signer;
        let service = Arc::new(
            Sep10Service::new(
                "GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN".to_string(),
                "Test SDF Network ; September 2015".to_string(),
                "example.com".to_string(),
                Arc::new(tokio::sync::RwLock::new(None)),
            )
            .unwrap(),
        );
        let app = routes(service);
        let key = ed25519_dalek::SigningKey::from_bytes(&[3; 32]);
        let account: String = format!(
            "{}",
            stellar_strkey::ed25519::PublicKey(key.verifying_key().to_bytes())
        );

        let (status, challenge) = call(
            &app,
            post(
                "/api/sep10/auth",
                json!({ "account": account, "home_domain": "example.com" }),
                None,
            ),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{challenge}");
        let transaction = challenge["transaction"].as_str().unwrap().to_string();
        let signature = BASE64.encode(key.sign(transaction.as_bytes()).to_bytes());

        let (status, verified) = call(
            &app,
            post(
                "/api/sep10/verify",
                json!({ "transaction": transaction, "signature": signature }),
                None,
            ),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{verified}");
        let token = verified["token"].as_str().unwrap().to_string();

        let session = |t: &str| {
            Request::get("/api/sep10/session")
                .header("authorization", format!("Bearer {t}"))
                .body(Body::empty())
                .unwrap()
        };
        let (status, body) = call(&app, session(&token)).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["account"], account.as_str());

        let (status, _) = call(&app, post("/api/sep10/logout", json!({}), Some(&token))).await;
        assert_eq!(status, StatusCode::OK);

        let (status, _) = call(&app, session(&token)).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);
    }
}
