use std::time::Instant;
use axum::{
    extract::{Request, State},
    http::{HeaderMap, HeaderValue, Method, Uri},
    middleware::Next,
    response::Response,
};
use uuid::Uuid;
use tracing::{info, warn, error};

use crate::{
    errors::AppError,
    server::AppState,
};

/// Request ID header name
pub const REQUEST_ID_HEADER: &str = "x-request-id";

/// Request context information for logging and tracing
#[derive(Debug, Clone)]
pub struct RequestContext {
    pub request_id: String,
    pub method: String,
    pub uri: String,
    pub user_agent: Option<String>,
    pub start_time: Instant,
}

impl RequestContext {
    /// Create new request context with generated request ID
    pub fn new(method: Method, uri: Uri, headers: &HeaderMap) -> Self {
        let request_id = Uuid::new_v4().to_string();
        let user_agent = headers
            .get("user-agent")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());

        Self {
            request_id,
            method: method.to_string(),
            uri: uri.to_string(),
            user_agent,
            start_time: Instant::now(),
        }
    }

    /// Create request context from existing request ID
    pub fn from_request_id(
        request_id: String,
        method: Method,
        uri: Uri,
        headers: &HeaderMap,
    ) -> Self {
        let user_agent = headers
            .get("user-agent")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());

        Self {
            request_id,
            method: method.to_string(),
            uri: uri.to_string(),
            user_agent,
            start_time: Instant::now(),
        }
    }

    /// Get elapsed time since request start
    pub fn elapsed(&self) -> std::time::Duration {
        self.start_time.elapsed()
    }
}

/// Logging middleware that adds request ID and structured logging
pub async fn logging_middleware(
    State(state): State<AppState>,
    mut request: Request,
    next: Next,
) -> Result<Response, AppError> {
    // Extract or generate request ID
    let request_id = request
        .headers()
        .get(REQUEST_ID_HEADER)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
        .unwrap_or_else(|| Uuid::new_v4().to_string());

    // Create request context
    let context = RequestContext::from_request_id(
        request_id.clone(),
        request.method().clone(),
        request.uri().clone(),
        request.headers(),
    );

    // Add request ID to headers for downstream processing
    request.headers_mut().insert(
        REQUEST_ID_HEADER,
        HeaderValue::from_str(&request_id)
            .map_err(|e| AppError::internal(format!("Invalid request ID: {}", e)))?,
    );

    // Create tracing span with request context
    let span = tracing::info_span!(
        "http_request",
        request_id = %context.request_id,
        method = %context.method,
        uri = %context.uri,
        user_agent = context.user_agent.as_deref().unwrap_or("unknown")
    );

    // Enter the span for this request
    let _enter = span.enter();

    // Log request start
    info!(
        request_id = %context.request_id,
        method = %context.method,
        uri = %context.uri,
        user_agent = context.user_agent.as_deref().unwrap_or("unknown"),
        "Request started"
    );

    // Record request start for metrics
    let metrics_start = state.metrics.record_request_start();

    // Process the request
    let mut response = next.run(request).await;

    // Calculate request duration
    let duration = context.elapsed();
    let duration_ms = duration.as_millis() as u64;

    // Add request ID to response headers
    response.headers_mut().insert(
        REQUEST_ID_HEADER,
        HeaderValue::from_str(&request_id)
            .map_err(|e| AppError::internal(format!("Invalid request ID: {}", e)))?,
    );

    let status = response.status();

    // Log request completion
    if status.is_success() {
        info!(
            request_id = %context.request_id,
            method = %context.method,
            uri = %context.uri,
            status = %status.as_u16(),
            duration_ms = duration_ms,
            "Request completed successfully"
        );

        // Record successful request for metrics
        let provider_name = extract_provider_from_uri(&context.uri);
        let model_name = "unknown"; // Will be extracted from request body in actual handler
        state.metrics.record_request_end(metrics_start, true, provider_name, model_name).await;
    } else {
        warn!(
            request_id = %context.request_id,
            method = %context.method,
            uri = %context.uri,
            status = %status.as_u16(),
            duration_ms = duration_ms,
            "Request completed with error status"
        );

        // Record failed request for metrics
        let provider_name = extract_provider_from_uri(&context.uri);
        let model_name = "unknown";
        state.metrics.record_request_end(metrics_start, false, provider_name, model_name).await;
    }

    Ok(response)
}

/// Error handling middleware that provides consistent error responses
pub async fn error_handling_middleware(
    request: Request,
    next: Next,
) -> Response {
    let request_id = request
        .headers()
        .get(REQUEST_ID_HEADER)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("unknown")
        .to_string();

    let response = next.run(request).await;
    
    // Check if response indicates an error
    if !response.status().is_success() {
        error!(
            request_id = request_id,
            status = %response.status().as_u16(),
            "Request completed with error status"
        );
    }

    response
}

/// Request validation middleware
pub async fn validation_middleware(
    request: Request,
    next: Next,
) -> Result<Response, AppError> {
    let request_id = request
        .headers()
        .get(REQUEST_ID_HEADER)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("unknown");

    // Validate content type for POST requests
    if request.method() == Method::POST {
        let content_type = request
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok());

        match content_type {
            Some(ct) if ct.starts_with("application/json") => {
                // Valid JSON content type
            }
            Some(ct) => {
                warn!(
                    request_id = request_id,
                    content_type = ct,
                    "Invalid content type for POST request"
                );
                return Err(AppError::ValidationError(
                    "Content-Type must be application/json for POST requests".to_string(),
                ));
            }
            None => {
                warn!(
                    request_id = request_id,
                    "Missing content-type header for POST request"
                );
                return Err(AppError::ValidationError(
                    "Content-Type header is required for POST requests".to_string(),
                ));
            }
        }
    }

    // Validate request size (prevent extremely large requests)
    if let Some(content_length) = request.headers().get("content-length") {
        if let Ok(length_str) = content_length.to_str() {
            if let Ok(length) = length_str.parse::<usize>() {
                const MAX_REQUEST_SIZE: usize = 10 * 1024 * 1024; // 10MB
                if length > MAX_REQUEST_SIZE {
                    warn!(
                        request_id = request_id,
                        content_length = length,
                        max_allowed = MAX_REQUEST_SIZE,
                        "Request size exceeds maximum allowed"
                    );
                    return Err(AppError::ValidationError(
                        "Request size exceeds maximum allowed limit".to_string(),
                    ));
                }
            }
        }
    }

    info!(
        request_id = request_id,
        "Request validation passed"
    );

    Ok(next.run(request).await)
}

/// Performance monitoring middleware
pub async fn performance_middleware(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> Result<Response, AppError> {
    let request_id = request
        .headers()
        .get(REQUEST_ID_HEADER)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("unknown")
        .to_string();

    let start_time = Instant::now();
    let uri = request.uri().to_string();

    // Record concurrent request
    state.metrics.increment_concurrent_requests().await;

    let response = next.run(request).await;

    // Record request completion
    state.metrics.decrement_concurrent_requests().await;

    let duration = start_time.elapsed();
    let duration_ms = duration.as_millis() as u64;

    // Log performance metrics
    info!(
        request_id = request_id,
        uri = uri,
        duration_ms = duration_ms,
        "Performance metrics recorded"
    );

    // Warn about slow requests
    if duration_ms > 5000 {
        warn!(
            request_id = request_id,
            uri = uri,
            duration_ms = duration_ms,
            "Slow request detected"
        );
    }

    Ok(response)
}

/// Extract provider name from URI for metrics
fn extract_provider_from_uri(uri: &str) -> &str {
    if uri.contains("openai") || uri.contains("gpt") {
        "openai"
    } else if uri.contains("gemini") {
        "gemini"
    } else if uri.contains("anthropic") || uri.contains("claude") {
        "anthropic"
    } else {
        "unknown"
    }
}

/// Simple request ID middleware
pub async fn request_id_middleware(
    mut request: Request,
    next: Next,
) -> Response {
    // Generate request ID if not present
    let request_id = if !request.headers().contains_key(REQUEST_ID_HEADER) {
        let request_id = Uuid::new_v4().to_string();
        if let Ok(header_value) = HeaderValue::from_str(&request_id) {
            request.headers_mut().insert(REQUEST_ID_HEADER, header_value);
        }
        request_id
    } else {
        // Get existing request ID
        request.headers()
            .get(REQUEST_ID_HEADER)
            .and_then(|v| v.to_str().ok())
            .unwrap_or_else(|| "unknown")
            .to_string()
    };

    let mut response = next.run(request).await;

    // Add request ID to response headers
    if let Ok(header_value) = HeaderValue::from_str(&request_id) {
        response.headers_mut().insert(REQUEST_ID_HEADER, header_value);
    }

    response
}