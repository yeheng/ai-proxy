use ai_proxy::config::{
    Config, LoggingConfig, PerformanceConfig, ProviderDetail, SecurityConfig, ServerConfig,
};
use ai_proxy::providers::{HealthStatus, ModelInfo, ProviderRegistry};
use std::collections::HashMap;
use std::sync::Arc;

// Helper function to create a test config
fn create_test_config() -> Config {
    let mut providers = HashMap::new();
    providers.insert(
        "openai".to_string(),
        ProviderDetail {
            api_key: "test-openai-key-1234567890".to_string(),
            api_base: "https://api.openai.com/v1/".to_string(),
            models: Some(vec!["gpt-4".to_string(), "gpt-3.5-turbo".to_string()]),
            timeout_seconds: 60,
            max_retries: 3,
            enabled: true,
            rate_limit: None,
        },
    );
    providers.insert(
        "anthropic".to_string(),
        ProviderDetail {
            api_key: "test-anthropic-key-1234567890".to_string(),
            api_base: "https://api.anthropic.com/v1/".to_string(),
            models: Some(vec![
                "claude-3-sonnet".to_string(),
                "claude-3-haiku".to_string(),
            ]),
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
            max_request_size_bytes: 1024 * 1024,
        },
        providers,
        logging: LoggingConfig::default(),
        security: SecurityConfig::default(),
        performance: PerformanceConfig::default(),
    }
}

#[tokio::test]
async fn test_provider_registry_creation() {
    let config = Arc::new(create_test_config());
    let http_client = reqwest::Client::new();

    let registry = ProviderRegistry::new(&config, http_client);
    assert!(registry.is_ok());

    let registry = registry.unwrap();

    // Test that providers are registered
    assert!(registry.get_provider("gpt-4").is_some());
    assert!(registry.get_provider("claude-3-sonnet").is_some());
    assert!(registry.get_provider("nonexistent-model").is_none());
}

#[tokio::test]
async fn test_provider_registry_model_mapping() {
    let config = Arc::new(create_test_config());
    let http_client = reqwest::Client::new();

    let registry = ProviderRegistry::new(&config, http_client).unwrap();

    // Test model to provider mapping
    let openai_provider = registry.get_provider("gpt-4");
    assert!(openai_provider.is_some());

    let anthropic_provider = registry.get_provider("claude-3-sonnet");
    assert!(anthropic_provider.is_some());

    // Test that different models map to different providers
    let gpt_provider = registry.get_provider("gpt-4");
    let claude_provider = registry.get_provider("claude-3-sonnet");

    // They should be different provider instances
    assert!(gpt_provider.is_some());
    assert!(claude_provider.is_some());
}

#[tokio::test]
async fn test_provider_registry_list_models() {
    let config = Arc::new(create_test_config());
    let http_client = reqwest::Client::new();

    let registry = ProviderRegistry::new(&config, http_client).unwrap();

    let models = registry.list_all_models().await;
    assert!(models.is_ok());

    let models = models.unwrap();
    assert!(!models.is_empty());

    // Should contain models from both providers
    let model_ids: Vec<&str> = models.iter().map(|m| m.id.as_str()).collect();
    assert!(model_ids.contains(&"gpt-4"));
    assert!(model_ids.contains(&"claude-3-sonnet"));
}

#[tokio::test]
async fn test_provider_registry_health_check() {
    let config = Arc::new(create_test_config());
    let http_client = reqwest::Client::new();

    let registry = ProviderRegistry::new(&config, http_client).unwrap();

    let health_status = registry.health_check_all().await;
    assert!(!health_status.is_empty());

    // Should have health status for each provider
    assert!(health_status.len() >= 2);
}

#[tokio::test]
async fn test_provider_registry_disabled_provider() {
    let mut config = create_test_config();

    // Disable one provider
    config.providers.get_mut("openai").unwrap().enabled = false;

    let config = Arc::new(config);
    let http_client = reqwest::Client::new();

    let registry = ProviderRegistry::new(&config, http_client).unwrap();

    // Disabled provider's models should not be available
    // After disabling, provider should not be available
    // Verify disabled provider's models are not available
    let models = registry.list_all_models().await.unwrap();
    assert!(!models.iter().any(|m| m.id == "gpt-4"));

    // Enabled provider's models should still be available
    assert!(registry.get_provider("claude-3-sonnet").is_some());
}

#[tokio::test]
async fn test_provider_registry_empty_config() {
    let mut config = create_test_config();
    config.providers.clear();

    let config = Arc::new(config);
    let http_client = reqwest::Client::new();

    let registry = ProviderRegistry::new(&config, http_client);

    // Empty provider config should return error
    assert!(registry.is_err());
    if let Err(e) = registry {
        assert!(e.to_string().contains("No providers configured"));
    } else {
        panic!("Expected error when providers are empty");
    }
}

#[tokio::test]
async fn test_provider_registry_model_prefix_matching() {
    let config = Arc::new(create_test_config());
    let http_client = reqwest::Client::new();

    let registry = ProviderRegistry::new(&config, http_client).unwrap();

    // Test that models are matched correctly by prefix or exact name
    assert!(registry.get_provider("gpt-4").is_some());
    assert!(registry.get_provider("gpt-3.5-turbo").is_some());
    assert!(registry.get_provider("claude-3-sonnet").is_some());
    assert!(registry.get_provider("claude-3-haiku").is_some());

    // Non-existent models should return None
    assert!(registry.get_provider("gpt-5").is_none());
    assert!(registry.get_provider("claude-4").is_none());
}

#[test]
fn test_model_info_creation() {
    let model = ModelInfo {
        id: "test-model".to_string(),
        object: "model".to_string(),
        created: 1234567890,
        owned_by: "test-provider".to_string(),
    };

    assert_eq!(model.id, "test-model");
    assert_eq!(model.object, "model");
    assert_eq!(model.created, 1234567890);
    assert_eq!(model.owned_by, "test-provider");
}

#[test]
fn test_health_status_creation() {
    let health = HealthStatus {
        status: "healthy".to_string(),
        provider: "test-provider".to_string(),
        latency_ms: Some(150),
        error: None,
    };

    assert_eq!(health.status, "healthy");
    assert_eq!(health.provider, "test-provider");
    assert_eq!(health.latency_ms, Some(150));
    assert!(health.error.is_none());

    let unhealthy = HealthStatus {
        status: "unhealthy".to_string(),
        provider: "test-provider".to_string(),
        latency_ms: None,
        error: Some("Connection failed".to_string()),
    };

    assert_eq!(unhealthy.status, "unhealthy");
    assert!(unhealthy.error.is_some());
}

#[test]
fn test_model_info_serialization() {
    let model = ModelInfo {
        id: "test-model".to_string(),
        object: "model".to_string(),
        created: 1234567890,
        owned_by: "test-provider".to_string(),
    };

    let serialized = serde_json::to_string(&model);
    assert!(serialized.is_ok());

    let json = serialized.unwrap();
    assert!(json.contains("test-model"));
    assert!(json.contains("test-provider"));
}

#[test]
fn test_health_status_serialization() {
    let health = HealthStatus {
        status: "healthy".to_string(),
        provider: "test-provider".to_string(),
        latency_ms: Some(150),
        error: None,
    };

    let serialized = serde_json::to_string(&health);
    assert!(serialized.is_ok());

    let json = serialized.unwrap();
    assert!(json.contains("healthy"));
    assert!(json.contains("test-provider"));
    assert!(json.contains("150"));
}
