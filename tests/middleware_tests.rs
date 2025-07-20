use ai_proxy::{
    config::{
        Config, LoggingConfig, PerformanceConfig, ProviderDetail, SecurityConfig, ServerConfig,
    },
    metrics::MetricsCollector,
    middleware::{
        error_handling_middleware, logging_middleware, performance_middleware,
        request_id_middleware, validation_middleware,
    },
    providers::registry::ProviderRegistry,
    server::AppState,
};
use axum::{
    Router,
    body::Body,
    http::{Request, StatusCode},
    middleware,
    response::Response,
    routing::{get, post},
};
use reqwest::Client;
use std::{collections::HashMap, sync::Arc};
use tokio::sync::RwLock;
use tower::ServiceExt;

// Helper function to create test app state
fn create_test_app_state() -> AppState {
    let mut providers = HashMap::new();
    providers.insert(
        "anthropic-test".to_string(),
        ProviderDetail {
            api_key: "test-api-key-1234567890".to_string(),
            api_base: "https://api.anthropic.com/v1/".to_string(),
            models: Some(vec!["claude-3-sonnet".to_string()]),
            timeout_seconds: 30,
            max_retries: 3,
            enabled: true,
            rate_limit: None,
        },
    );

    let config = Config {
        server: ServerConfig {
            host: "127.0.0.1".to_string(),
            port: 3000,
            request_timeout_seconds: 30,
            max_request_size_bytes: 1024 * 1024,
        },
        providers,
        logging: LoggingConfig::default(),
        security: SecurityConfig::default(),
        performance: PerformanceConfig::default(),
    };

    let http_client = Client::new();
    let provider_registry = Arc::new(RwLock::new(
        ProviderRegistry::new(&config, http_client.clone()).unwrap(),
    ));
    let metrics = Arc::new(MetricsCollector::new());

    AppState {
        config: Arc::new(config),
        http_client,
        provider_registry,
        metrics,
    }
}

// Mock handlers for testing middleware
async fn mock_handler_success() -> Response<Body> {
    Response::builder()
        .status(StatusCode::OK)
        .body(Body::from("Success"))
        .unwrap()
}

async fn mock_handler_error() -> Response<Body> {
    Response::builder()
        .status(StatusCode::BAD_REQUEST)
        .body(Body::from("Error"))
        .unwrap()
}

#[tokio::test]
async fn test_request_id_middleware_adds_header() {
    let app = Router::new()
        .route("/test", get(mock_handler_success))
        .layer(middleware::from_fn(request_id_middleware));

    let request = Request::builder().uri("/test").body(Body::empty()).unwrap();

    let response = app.oneshot(request).await.unwrap();

    // Check that request ID header is added
    assert!(response.headers().contains_key("x-request-id"));
    let request_id = response.headers().get("x-request-id").unwrap();
    assert!(!request_id.is_empty());

    // Request ID should be a valid UUID format
    let request_id_str = request_id.to_str().unwrap();
    assert!(request_id_str.len() >= 32); // UUID without hyphens is 32 chars
}

#[tokio::test]
async fn test_request_id_middleware_preserves_existing_id() {
    let existing_id = "existing-request-id-123";
    let app = Router::new()
        .route("/test", get(mock_handler_success))
        .layer(middleware::from_fn(request_id_middleware));

    let request = Request::builder()
        .uri("/test")
        .header("x-request-id", existing_id)
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    // Should preserve existing request ID (middleware doesn't overwrite existing IDs)
    let response_id = response
        .headers()
        .get("x-request-id")
        .unwrap()
        .to_str()
        .unwrap();
    assert_eq!(response_id, existing_id);
}

#[tokio::test]
async fn test_logging_middleware_logs_request_response() {
    let app_state = create_test_app_state();
    let app = Router::new()
        .route("/v1/messages", get(mock_handler_success))
        .layer(middleware::from_fn_with_state(
            app_state.clone(),
            logging_middleware,
        ))
        .with_state(app_state);

    let request = Request::builder()
        .method("GET")
        .uri("/v1/messages")
        .header("content-type", "application/json")
        .header("x-request-id", "test-request-123")
        .body(Body::empty())
        .unwrap();

    // This test mainly ensures the middleware doesn't panic
    // Actual log verification would require capturing log output
    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_logging_middleware_logs_errors() {
    let app_state = create_test_app_state();
    let app = Router::new()
        .route("/v1/messages", get(mock_handler_error))
        .layer(middleware::from_fn_with_state(
            app_state.clone(),
            logging_middleware,
        ))
        .with_state(app_state);

    let request = Request::builder()
        .method("GET")
        .uri("/v1/messages")
        .header("x-request-id", "test-request-error")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    // Should handle the response (logging middleware doesn't change status)
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_error_handling_middleware_passes_through() {
    let app = Router::new()
        .route("/test", get(mock_handler_error))
        .layer(middleware::from_fn(error_handling_middleware));

    let request = Request::builder().uri("/test").body(Body::empty()).unwrap();

    let response = app.oneshot(request).await.unwrap();

    // Error handling middleware just logs, doesn't change the response
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_error_handling_middleware_passes_success() {
    let app = Router::new()
        .route("/test", get(mock_handler_success))
        .layer(middleware::from_fn(error_handling_middleware));

    let request = Request::builder().uri("/test").body(Body::empty()).unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_validation_middleware_valid_post_request() {
    let app = Router::new()
        .route("/v1/messages", post(mock_handler_success))
        .layer(middleware::from_fn(validation_middleware));

    let request = Request::builder()
        .method("POST")
        .uri("/v1/messages")
        .header("content-type", "application/json")
        .body(Body::from(r#"{"model": "test-model", "messages": []}"#))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_validation_middleware_missing_content_type() {
    let app = Router::new()
        .route("/v1/messages", post(mock_handler_success))
        .layer(middleware::from_fn(validation_middleware));

    let request = Request::builder()
        .method("POST")
        .uri("/v1/messages")
        .body(Body::from(r#"{"model": "test-model"}"#))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    // Should return error status due to missing content-type
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_validation_middleware_invalid_content_type() {
    let app = Router::new()
        .route("/v1/messages", post(mock_handler_success))
        .layer(middleware::from_fn(validation_middleware));

    let request = Request::builder()
        .method("POST")
        .uri("/v1/messages")
        .header("content-type", "text/plain")
        .body(Body::from("plain text"))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    // Should return error status due to invalid content-type
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_validation_middleware_get_request_passes() {
    let app = Router::new()
        .route("/health", get(mock_handler_success))
        .layer(middleware::from_fn(validation_middleware));

    let request = Request::builder()
        .method("GET")
        .uri("/health")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_performance_middleware() {
    let app_state = create_test_app_state();
    let app = Router::new()
        .route("/v1/messages", get(mock_handler_success))
        .layer(middleware::from_fn_with_state(
            app_state.clone(),
            performance_middleware,
        ))
        .with_state(app_state);

    let request = Request::builder()
        .uri("/v1/messages")
        .header("x-request-id", "perf-test-123")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}
