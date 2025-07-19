use ai_proxy::{
    config::{Config, ProviderDetail, ServerConfig}, middleware::REQUEST_ID_HEADER, server::{create_app, AppState}
};
use axum::{
    body::Body,
    http::{Request, StatusCode, Method},
};
use std::collections::HashMap;
use tower::ServiceExt;
use uuid::Uuid;

/// Test helper to create a test configuration
fn create_test_config() -> Config {
    let mut providers = HashMap::new();
     providers.insert(
        "gemini".to_string(),
        ProviderDetail {
            api_key: "test-api-key-1234567890".to_string(),
            api_base: "https://api.example.com/v1/".to_string(),
            models: Some(vec!["model1".to_string(), "model2".to_string()]),
            timeout_seconds: 60,
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
            max_request_size_bytes: 10 * 1024 * 1024, // 10MB
        },
        providers: providers,
        logging: Default::default(),
        security: Default::default(),
        performance: Default::default(),
    }
}

/// Test helper to create test app state
async fn create_test_app_state() -> AppState {
    let config = create_test_config();
    AppState::new(config).expect("Failed to create test app state")
}

#[tokio::test]
async fn test_request_id_middleware() {
    let app_state = create_test_app_state().await;
    let app = create_app(app_state);

    // Test request without request ID - should generate one
    let request = Request::builder()
        .method(Method::GET)
        .uri("/health")
        .body(Body::empty())
        .unwrap();

    let response = app.clone().oneshot(request).await.unwrap();
    
    // Should have request ID in response headers
    assert!(response.headers().contains_key(REQUEST_ID_HEADER));
    
    let request_id = response.headers()
        .get(REQUEST_ID_HEADER)
        .unwrap()
        .to_str()
        .unwrap();
    
    // Should be a valid UUID
    assert!(Uuid::parse_str(request_id).is_ok());
}

#[tokio::test]
async fn test_request_id_preservation() {
    let app_state = create_test_app_state().await;
    let app = create_app(app_state);

    let test_request_id = "test-request-id-12345";

    // Test request with existing request ID - should preserve it
    let request = Request::builder()
        .method(Method::GET)
        .uri("/health")
        .header(REQUEST_ID_HEADER, test_request_id)
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    
    // Should preserve the original request ID
    let response_request_id = response.headers()
        .get(REQUEST_ID_HEADER)
        .unwrap()
        .to_str()
        .unwrap();
    
    assert_eq!(response_request_id, test_request_id);
}

#[tokio::test]
async fn test_validation_middleware_content_type() {
    let app_state = create_test_app_state().await;
    let app = create_app(app_state);

    // Test POST request without content-type header
    let request = Request::builder()
        .method(Method::POST)
        .uri("/v1/messages")
        .body(Body::from(r#"{"model": "test", "messages": []}"#))
        .unwrap();

    let response = app.clone().oneshot(request).await.unwrap();
    
    // Should return 400 Bad Request for missing content-type
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_validation_middleware_valid_content_type() {
    let app_state = create_test_app_state().await;
    let app = create_app(app_state);

    // Test POST request with valid content-type header
    let request = Request::builder()
        .method(Method::POST)
        .uri("/v1/messages")
        .header("content-type", "application/json")
        .body(Body::from(r#"{"model": "test", "messages": []}"#))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    
    // Should not fail due to content-type validation
    // (may fail for other reasons like missing provider, but not validation)
    assert_ne!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_validation_middleware_request_size_limit() {
    let app_state = create_test_app_state().await;
    let app = create_app(app_state);

    // Create a large request body (larger than 10MB limit)
    let large_body = "x".repeat(11 * 1024 * 1024); // 11MB

    let request = Request::builder()
        .method(Method::POST)
        .uri("/v1/messages")
        .header("content-type", "application/json")
        .header("content-length", large_body.len().to_string())
        .body(Body::from(large_body))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    
    // Should return 400 Bad Request for oversized request
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_health_endpoint_with_middleware() {
    let app_state = create_test_app_state().await;
    let app = create_app(app_state);

    let request = Request::builder()
        .method(Method::GET)
        .uri("/health")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    
    // Should return 200 OK
    assert_eq!(response.status(), StatusCode::OK);
    
    // Should have request ID in response
    assert!(response.headers().contains_key(REQUEST_ID_HEADER));
}

#[tokio::test]
async fn test_metrics_collection() {
    let app_state = create_test_app_state().await;
    let app = create_app(app_state.clone());

    // Make a request to trigger metrics collection
    let request = Request::builder()
        .method(Method::GET)
        .uri("/health")
        .body(Body::empty())
        .unwrap();

    let _response = app.oneshot(request).await.unwrap();
    
    // Check that metrics were collected
    let metrics_summary = app_state.metrics.get_metrics_summary().await;
    assert!(metrics_summary.total_requests > 0);
}

#[tokio::test]
async fn test_concurrent_request_tracking() {
    let app_state = create_test_app_state().await;
    
    // Manually test concurrent request tracking
    app_state.metrics.increment_concurrent_requests().await;
    assert_eq!(app_state.metrics.get_concurrent_requests(), 1);
    
    app_state.metrics.increment_concurrent_requests().await;
    assert_eq!(app_state.metrics.get_concurrent_requests(), 2);
    
    app_state.metrics.decrement_concurrent_requests().await;
    assert_eq!(app_state.metrics.get_concurrent_requests(), 1);
    
    app_state.metrics.decrement_concurrent_requests().await;
    assert_eq!(app_state.metrics.get_concurrent_requests(), 0);
}

#[tokio::test]
async fn test_cors_headers() {
    let app_state = create_test_app_state().await;
    let app = create_app(app_state);

    let request = Request::builder()
        .method(Method::OPTIONS)
        .uri("/health")
        .header("origin", "https://example.com")
        .header("access-control-request-method", "GET")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    
    // Should have CORS headers
    assert!(response.headers().contains_key("access-control-allow-origin"));
}