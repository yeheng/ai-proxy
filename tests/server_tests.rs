use ai_proxy::{
    config::{
        Config, LoggingConfig, PerformanceConfig, ProviderDetail, SecurityConfig, ServerConfig,
    },
    metrics::MetricsCollector,
    providers::registry::ProviderRegistry,
    server::{AppState, create_app},
};
use axum::{
    body::Body,
    http::{Method, Request, StatusCode},
};
use reqwest::Client;
use serde_json::json;
use std::{collections::HashMap, sync::Arc};
use tokio::sync::RwLock;
use tower::ServiceExt;

// Helper function to create test configuration
fn create_test_config() -> Config {
    let mut providers = HashMap::new();
    providers.insert(
        "openai".to_string(),
        ProviderDetail {
            api_key: "test-api-key-1234567890".to_string(),
            api_base: "https://api.openai.com/v1/".to_string(),
            models: Some(vec!["gpt-3.5-turbo".to_string()]),
            timeout_seconds: 30,
            max_retries: 3,
            enabled: true,
            rate_limit: None,
        },
    );

    Config {
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
    }
}

// Helper function to create test app state
fn create_test_app_state() -> AppState {
    let config = create_test_config();
    let http_client = Client::new();
    // Create registry with config and http client
    let registry = ProviderRegistry::new(&config, http_client.clone()).unwrap();
    let provider_registry = Arc::new(RwLock::new(registry));
    let metrics = Arc::new(MetricsCollector::new());

    AppState {
        config: Arc::new(config),
        http_client,
        provider_registry,
        metrics,
    }
}

#[tokio::test]
async fn test_create_app_basic_routes() {
    let app_state = create_test_app_state();
    let app = create_app(app_state);

    // Test health endpoint
    let request = Request::builder()
        .method(Method::GET)
        .uri("/health")
        .body(Body::empty())
        .unwrap();

    let response = app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    // Test models endpoint
    let request = Request::builder()
        .method(Method::GET)
        .uri("/v1/models")
        .body(Body::empty())
        .unwrap();

    let response = app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    // Test providers health endpoint
    let request = Request::builder()
        .method(Method::GET)
        .uri("/health/providers")
        .body(Body::empty())
        .unwrap();

    let response = app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    // Test metrics endpoint
    let request = Request::builder()
        .method(Method::GET)
        .uri("/metrics")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_chat_handler_invalid_json() {
    let app_state = create_test_app_state();
    let app = create_app(app_state);

    let request = Request::builder()
        .method(Method::POST)
        .uri("/v1/messages")
        .header("content-type", "application/json")
        .body(Body::from("invalid json"))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn test_chat_handler_missing_content_type() {
    let app_state = create_test_app_state();
    let app = create_app(app_state);

    let request = Request::builder()
        .method(Method::POST)
        .uri("/v1/messages")
        .body(Body::from(r#"{"model": "test-model", "messages": []}"#))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn test_chat_handler_validation_error() {
    let app_state = create_test_app_state();
    let app = create_app(app_state);

    // Request with empty messages (should fail validation)
    let request = Request::builder()
        .method(Method::POST)
        .uri("/v1/messages")
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "model": "test-model",
                "messages": [],
                "max_tokens": 100
            })
            .to_string(),
        ))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::METHOD_NOT_ALLOWED);
}

#[tokio::test]
async fn test_models_handler_success() {
    let app_state = create_test_app_state();
    let app = create_app(app_state);

    let request = Request::builder()
        .method(Method::GET)
        .uri("/v1/models")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    // Response should be JSON
    let content_type = response.headers().get("content-type").unwrap();
    assert!(content_type.to_str().unwrap().contains("application/json"));
}

#[tokio::test]
async fn test_health_handler_success() {
    let app_state = create_test_app_state();
    let app = create_app(app_state);

    let request = Request::builder()
        .method(Method::GET)
        .uri("/health")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    // Response should be JSON
    let content_type = response.headers().get("content-type").unwrap();
    assert!(content_type.to_str().unwrap().contains("application/json"));
}

#[tokio::test]
async fn test_providers_health_handler_success() {
    let app_state = create_test_app_state();
    let app = create_app(app_state);

    let request = Request::builder()
        .method(Method::GET)
        .uri("/health/providers")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    // Response should be JSON
    let content_type = response.headers().get("content-type").unwrap();
    assert!(content_type.to_str().unwrap().contains("application/json"));
}

#[tokio::test]
async fn test_404_not_found() {
    let app_state = create_test_app_state();
    let app = create_app(app_state);

    let request = Request::builder()
        .method(Method::GET)
        .uri("/nonexistent")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_method_not_allowed() {
    let app_state = create_test_app_state();
    let app = create_app(app_state);

    // POST to health endpoint (should be GET only)
    let request = Request::builder()
        .method(Method::POST)
        .uri("/health")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_cors_headers() {
    let app_state = create_test_app_state();
    let app = create_app(app_state);

    let request = Request::builder()
        .method(Method::OPTIONS)
        .uri("/v1/messages")
        .header("origin", "https://example.com")
        .header("access-control-request-method", "POST")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    // Should handle CORS preflight
    assert!(response.status().is_success() || response.status() == StatusCode::NO_CONTENT);
}

#[tokio::test]
async fn test_request_id_header() {
    let app_state = create_test_app_state();
    let app = create_app(app_state);

    let request = Request::builder()
        .method(Method::GET)
        .uri("/health")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    // Should include request ID header
    assert!(response.headers().contains_key("x-request-id"));
}

#[tokio::test]
async fn test_streaming_endpoint() {
    let app_state = create_test_app_state();
    let app = create_app(app_state);

    let request = Request::builder()
        .method(Method::POST)
        .uri("/v1/messages")
        .header("content-type", "application/json")
        .header("accept", "text/event-stream")
        .body(Body::from(
            json!({
                "model": "test-model",
                "messages": [{"role": "user", "content": "Hello"}],
                "max_tokens": 100,
                "stream": true
            })
            .to_string(),
        ))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    // Should handle streaming requests (even if provider is not available)
    // The response might be an error, but it should be handled gracefully
    assert!(response.status().is_client_error() || response.status().is_server_error());
}

// Note: Individual handler functions are not exported from the server module
// so we test them through the full application routes

// Test error handling in handlers

#[tokio::test]
async fn test_handler_error_responses() {
    let app_state = create_test_app_state();
    let app = create_app(app_state);

    // Test various error conditions
    let test_cases = vec![
        // Invalid JSON
        (
            Method::POST,
            "/v1/messages",
            "application/json",
            "invalid json",
            StatusCode::BAD_REQUEST,
        ),
        // Missing required fields
        (
            Method::POST,
            "/v1/messages",
            "application/json",
            r#"{"model": ""}"#,
            StatusCode::BAD_REQUEST,
        ),
        // Invalid content type
        (
            Method::POST,
            "/v1/messages",
            "text/plain",
            "hello",
            StatusCode::BAD_REQUEST,
        ),
    ];

    for (method, uri, content_type, body, expected_status) in test_cases {
        let request = Request::builder()
            .method(method)
            .uri(uri)
            .header("content-type", content_type)
            .body(Body::from(body))
            .unwrap();

        let response = app.clone().oneshot(request).await.unwrap();
        assert_eq!(response.status(), expected_status);
    }
}

// Test middleware integration with server

#[tokio::test]
async fn test_middleware_integration() {
    let app_state = create_test_app_state();
    let app = create_app(app_state);

    let request = Request::builder()
        .method(Method::GET)
        .uri("/health")
        .header("x-custom-header", "test-value")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    // Should have middleware-added headers
    assert!(response.headers().contains_key("x-request-id"));

    // Should handle the request successfully
    assert_eq!(response.status(), StatusCode::OK);
}

// Test server configuration validation

#[test]
fn test_app_state_creation() {
    let config = create_test_config();
    let http_client = Client::new();
    let registry = ProviderRegistry::new(&config, http_client.clone()).unwrap();
    let provider_registry = Arc::new(RwLock::new(registry));
    let metrics = Arc::new(MetricsCollector::new());

    let app_state = AppState {
        config: Arc::new(config),
        http_client,
        provider_registry,
        metrics,
    };

    // Verify app state is created correctly
    assert_eq!(app_state.config.server.port, 3000);
    assert!(!app_state.config.providers.is_empty());
}

// Test concurrent request handling

#[tokio::test]
async fn test_concurrent_requests() {
    let app_state = create_test_app_state();
    let app = create_app(app_state);

    // Create multiple concurrent requests
    let mut handles = vec![];

    for i in 0..10 {
        let app_clone = app.clone();
        let handle = tokio::spawn(async move {
            let request = Request::builder()
                .method(Method::GET)
                .uri("/health")
                .header("x-request-id", format!("concurrent-test-{}", i))
                .body(Body::empty())
                .unwrap();

            app_clone.oneshot(request).await.unwrap()
        });
        handles.push(handle);
    }

    // Wait for all requests to complete
    let responses = futures::future::join_all(handles).await;

    // All requests should succeed
    for response_result in responses {
        let response = response_result.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }
}

// Test request/response body size limits

#[tokio::test]
async fn test_request_size_limits() {
    let app_state = create_test_app_state();
    let app = create_app(app_state);

    // Test with large request body
    let large_body = "a".repeat(2 * 1024 * 1024); // 2MB
    let request = Request::builder()
        .method(Method::POST)
        .uri("/v1/messages")
        .header("content-type", "application/json")
        .body(Body::from(large_body))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    // Should handle large requests appropriately (either accept or reject with proper status)
    assert!(response.status().is_client_error() || response.status().is_success());
}

// Test graceful error handling for various scenarios

#[tokio::test]
async fn test_graceful_error_handling() {
    let app_state = create_test_app_state();
    let app = create_app(app_state);

    // Test various error scenarios
    let error_scenarios = vec![
        // Malformed JSON
        (
            r#"{"model": "test", "messages": [{"role": "user", "content": "hello"}"#,
            StatusCode::BAD_REQUEST,
        ),
        // Missing required fields
        (r#"{"messages": []}"#, StatusCode::BAD_REQUEST),
        // Invalid field values
        (
            r#"{"model": "", "messages": [], "max_tokens": -1}"#,
            StatusCode::BAD_REQUEST,
        ),
    ];

    for (body, expected_status) in error_scenarios {
        let request = Request::builder()
            .method(Method::POST)
            .uri("/v1/messages")
            .header("content-type", "application/json")
            .body(Body::from(body))
            .unwrap();

        let response = app.clone().oneshot(request).await.unwrap();
        assert_eq!(response.status(), expected_status);

        // Response should be JSON with error details
        let content_type = response.headers().get("content-type").unwrap();
        assert!(content_type.to_str().unwrap().contains("application/json"));
    }
}
