use ai_proxy::{
    config::{Config, ProviderDetail, ServerConfig},
    providers::ProviderRegistry,
};
use reqwest::Client;
use std::collections::HashMap;

fn create_test_config() -> Config {
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
async fn test_registry_creation() {
    let config = create_test_config();
    let client = Client::new();
    
    let registry = ProviderRegistry::new(&config, client);
    assert!(registry.is_ok());
    
    let registry = registry.unwrap();
    assert_eq!(registry.get_provider_ids().len(), 1);
    assert!(registry.get_provider_ids().contains(&"gemini".to_string()));
}

#[tokio::test]
async fn test_model_mapping() {
    let config = create_test_config();
    let client = Client::new();
    
    let registry = ProviderRegistry::new(&config, client).unwrap();
    
    // Test exact model match
    let provider = registry.get_provider_for_model("gemini-pro");
    assert!(provider.is_ok());
    
    // Test unknown model
    let provider = registry.get_provider_for_model("unknown-model");
    assert!(provider.is_err());
}

#[test]
fn test_empty_providers_config() {
    let config = Config {
        server: ServerConfig {
            host: "127.0.0.1".to_string(),
            port: 3000,
            request_timeout_seconds: 30,
            max_request_size_bytes: 1024 * 1024,
        },
        providers: HashMap::new(),
        logging: ai_proxy::config::LoggingConfig::default(),
        security: ai_proxy::config::SecurityConfig::default(),
        performance: ai_proxy::config::PerformanceConfig::default(),
    };
    let client = Client::new();
    
    let registry = ProviderRegistry::new(&config, client);
    assert!(registry.is_err());
}
