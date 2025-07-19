use ai_proxy::{
    config::{Config, ServerConfig, ProviderDetail},
    server::{AppState, create_app},
};
use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use tower::ServiceExt;
use std::collections::HashMap;

fn create_test_config() -> Config {
    let mut providers = HashMap::new();
    providers.insert("test".to_string(), ProviderDetail {
        api_key: "test-key".to_string(),
        api_base: "https://api.test.com/".to_string(),
        models: Some(vec!["test-model".to_string()]),
        timeout_seconds: 60,
        max_retries: 3,
        enabled: true,
        rate_limit: None,
    });

    Config {
        server: ServerConfig {
            host: "127.0.0.1".to_string(),
            port: 3000,
            request_timeout_seconds: 30,
            max_request_size_bytes: 1024 * 1024,
        },
        providers,
        logging: ai_proxy::config::LoggingConfig::default(),
        security: ai_proxy::config::SecurityConfig::default(),
        performance: ai_proxy::config::PerformanceConfig::default(),
    }
}

fn create_valid_test_config() -> Config {
    let mut providers = HashMap::new();
    providers.insert("gemini".to_string(), ProviderDetail {
        api_key: "test-key".to_string(),
        api_base: "https://api.gemini.com/".to_string(),
        models: Some(vec!["gemini-pro".to_string()]),
        timeout_seconds: 60,
        max_retries: 3,
        enabled: true,
        rate_limit: None,
    });

    Config {
        server: ServerConfig {
            host: "127.0.0.1".to_string(),
            port: 3000,
            request_timeout_seconds: 30,
            max_request_size_bytes: 1024 * 1024,
        },
        providers,
        logging: ai_proxy::config::LoggingConfig::default(),
        security: ai_proxy::config::SecurityConfig::default(),
        performance: ai_proxy::config::PerformanceConfig::default(),
    }
}

#[tokio::test]
async fn test_app_state_creation() {
    let config = create_test_config();
    let app_state = AppState::new(config);
    
    // Should fail because "test" is not a recognized provider type
    assert!(app_state.is_err());
}

#[tokio::test]
async fn test_app_state_creation_valid() {
    let config = create_valid_test_config();
    let app_state = AppState::new(config);
    
    // Should succeed with valid provider
    assert!(app_state.is_ok());
}

#[tokio::test]
async fn test_router_creation() {
    let config = create_valid_test_config();
    let app_state = AppState::new(config).unwrap();
    
    // This should not panic - router creation should work
    let _router = create_app(app_state);
}

#[tokio::test]
async fn test_models_endpoint() {
    let config = create_valid_test_config();
    let app_state = AppState::new(config).unwrap();
    let app = create_app(app_state);

    // Create a request to the models endpoint
    let request = Request::builder()
        .uri("/v1/models")
        .method("GET")
        .body(Body::empty())
        .unwrap();

    // Send the request
    let response = app.oneshot(request).await.unwrap();

    // Check that we get a successful response
    assert_eq!(response.status(), StatusCode::OK);

    // Check content type
    let content_type = response.headers().get("content-type").unwrap();
    assert!(content_type.to_str().unwrap().contains("application/json"));

    // Check response body structure
    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    
    // Verify the response has the expected structure
    assert_eq!(json["object"], "list");
    assert!(json["data"].is_array());
    
    // Should have at least one model from our test config
    let models = json["data"].as_array().unwrap();
    assert!(!models.is_empty(), "Should have at least one model");
    
    // Check that each model has the required fields
    for model in models {
        assert!(model["id"].is_string());
        assert!(model["object"].is_string());
        assert!(model["created"].is_number());
        assert!(model["owned_by"].is_string());
    }
}

#[tokio::test]
async fn test_health_endpoint() {
    let config = create_valid_test_config();
    let app_state = AppState::new(config).unwrap();
    let app = create_app(app_state);

    // Create a request to the health endpoint
    let request = Request::builder()
        .uri("/health")
        .method("GET")
        .body(Body::empty())
        .unwrap();

    // Send the request
    let response = app.oneshot(request).await.unwrap();

    // Check that we get a successful response
    assert_eq!(response.status(), StatusCode::OK);

    // Check content type
    let content_type = response.headers().get("content-type").unwrap();
    assert!(content_type.to_str().unwrap().contains("application/json"));

    // Check response body structure
    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    
    // Verify the response has the expected structure
    assert_eq!(json["status"], "healthy");
    assert_eq!(json["service"], "ai-proxy");
    assert!(json["version"].is_string());
    assert!(json["providers_configured"].is_number());
    assert!(json["timestamp"].is_string());
    
    // Should have at least one provider configured
    let provider_count = json["providers_configured"].as_u64().unwrap();
    assert!(provider_count > 0, "Should have at least one provider configured");
}

#[tokio::test]
async fn test_health_providers_endpoint() {
    let config = create_valid_test_config();
    let app_state = AppState::new(config).unwrap();
    let app = create_app(app_state);

    // Create a request to the provider health endpoint
    let request = Request::builder()
        .uri("/health/providers")
        .method("GET")
        .body(Body::empty())
        .unwrap();

    // Send the request
    let response = app.oneshot(request).await.unwrap();

    // Check that we get a successful response
    assert_eq!(response.status(), StatusCode::OK);

    // Check content type
    let content_type = response.headers().get("content-type").unwrap();
    assert!(content_type.to_str().unwrap().contains("application/json"));

    // Check response body structure
    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    
    // Verify the response has the expected structure
    assert!(json["status"].is_string());
    assert!(json["providers"].is_object());
    assert!(json["timestamp"].is_string());
    
    // Should have provider health information
    let providers = json["providers"].as_object().unwrap();
    assert!(!providers.is_empty(), "Should have at least one provider health status");
    
    // Check each provider health status structure
    for (provider_id, health) in providers {
        assert!(!provider_id.is_empty());
        assert!(health["status"].is_string());
        assert!(health["provider"].is_string());
        // latency_ms and error are optional fields
    }
}

#[tokio::test]
async fn test_metrics_endpoint() {
    let config = create_valid_test_config();
    let app_state = AppState::new(config).unwrap();
    let app = create_app(app_state);

    // Create a request to the metrics endpoint
    let request = Request::builder()
        .uri("/metrics")
        .method("GET")
        .body(Body::empty())
        .unwrap();

    // Send the request
    let response = app.oneshot(request).await.unwrap();

    // Check that we get a successful response
    assert_eq!(response.status(), StatusCode::OK);

    // Check content type
    let content_type = response.headers().get("content-type").unwrap();
    assert!(content_type.to_str().unwrap().contains("application/json"));

    // Check response body structure
    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    
    // Verify the response has the expected structure
    assert!(json["metrics"].is_object());
    let metrics = &json["metrics"];
    
    // Check basic metrics fields
    assert!(metrics["uptime_seconds"].is_number());
    assert!(metrics["total_requests"].is_number());
    assert!(metrics["successful_requests"].is_number());
    assert!(metrics["failed_requests"].is_number());
    assert!(metrics["success_rate_percent"].is_number());
    assert!(metrics["error_rate_percent"].is_number());
    assert!(metrics["avg_latency_ms"].is_number());
    assert!(metrics["timestamp"].is_string());
    
    // Check nested structures
    assert!(metrics["latency_stats"].is_object());
    assert!(metrics["provider_metrics"].is_object());
    assert!(metrics["model_metrics"].is_object());
}

#[tokio::test]
async fn test_metrics_collection() {
    use ai_proxy::metrics::MetricsCollector;
    use std::sync::Arc;
    
    let metrics = Arc::new(MetricsCollector::new());
    
    // Simulate some requests
    let start1 = metrics.record_request_start();
    tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
    metrics.record_request_end(start1, true, "openai", "gpt-4").await;
    
    let start2 = metrics.record_request_start();
    tokio::time::sleep(tokio::time::Duration::from_millis(20)).await;
    metrics.record_request_end(start2, false, "gemini", "gemini-pro").await;
    
    let start3 = metrics.record_request_start();
    tokio::time::sleep(tokio::time::Duration::from_millis(15)).await;
    metrics.record_request_end(start3, true, "openai", "gpt-3.5-turbo").await;
    
    // Get metrics summary
    let summary = metrics.get_metrics_summary().await;
    
    // Verify basic stats
    assert_eq!(summary.total_requests, 3);
    assert_eq!(summary.successful_requests, 2);
    assert_eq!(summary.failed_requests, 1);
    assert!((summary.success_rate_percent - 66.67).abs() < 0.1);
    assert!((summary.error_rate_percent - 33.33).abs() < 0.1);
    
    // Verify provider metrics
    assert!(summary.provider_metrics.contains_key("openai"));
    assert!(summary.provider_metrics.contains_key("gemini"));
    
    let openai_metrics = &summary.provider_metrics["openai"];
    assert_eq!(openai_metrics.total_requests, 2);
    assert_eq!(openai_metrics.successful_requests, 2);
    assert_eq!(openai_metrics.failed_requests, 0);
    
    let gemini_metrics = &summary.provider_metrics["gemini"];
    assert_eq!(gemini_metrics.total_requests, 1);
    assert_eq!(gemini_metrics.successful_requests, 0);
    assert_eq!(gemini_metrics.failed_requests, 1);
    
    // Verify model metrics
    assert!(summary.model_metrics.contains_key("gpt-4"));
    assert!(summary.model_metrics.contains_key("gemini-pro"));
    assert!(summary.model_metrics.contains_key("gpt-3.5-turbo"));
}
