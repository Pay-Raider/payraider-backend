use axum::{
    http::{HeaderValue, Request, Response},
    middleware::Next,
};

pub async fn version_middleware<B>(request: Request<B>, next: Next<B>) -> Response {
    let mut response = next.run(request).await;
    
    let headers = response.headers_mut();
    headers.insert("X-API-Version", HeaderValue::from_static("v1"));
    headers.insert("X-API-Status", HeaderValue::from_static("supported"));
    
    response
}
