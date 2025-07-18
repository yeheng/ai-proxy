use ai_proxy::{
    config::{Config, ServerConfig, ProviderDetail},
    server::AppState,
};
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
    let _router = ai_proxy::server::create_app(app_state);
}
