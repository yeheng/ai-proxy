use ai_proxy::{
    config::{Config, ServerConfig, ProviderDetail, LoggingConfig, SecurityConfig, PerformanceConfig},
    providers::ProviderRegistry,
};
use std::collections::HashMap;

#[tokio::main]
async fn main() {
    // Setup mock server
    let mock_server = wiremock::MockServer::start().await;
    
    println\!("Mock server URL: {}", mock_server.uri());
    
    // Create test configuration
    let mut providers = HashMap::new();
    providers.insert(
        "openai".to_string(),
        ProviderDetail {
            api_key: "test-openai-key-1234567890".to_string(),
            api_base: format\!("{}/v1/", mock_server.uri()),
            models: Some(vec\!["gpt-4".to_string(), "gpt-3.5-turbo".to_string()]),
            timeout_seconds: 30,
            max_retries: 3,
            enabled: true,
            rate_limit: None,
        },
    );

    let config = Config {
        server: ServerConfig {
            host: "127.0.0.1".to_string(),
            port: 0,
            request_timeout_seconds: 30,
            max_request_size_bytes: 1024 * 1024,
        },
        providers,
        logging: LoggingConfig {
            level: "debug".to_string(),
            format: "json".to_string(),
            log_requests: true,
            log_responses: true,
        },
        security: SecurityConfig::default(),
        performance: PerformanceConfig::default(),
    };
    
    let http_client = reqwest::Client::new();
    let registry = ProviderRegistry::new(&config, http_client).await.unwrap();
    
    // Check what URLs are configured
    let provider = registry.get_provider("gpt-4").unwrap();
    
    println\!("Provider API base: {}", provider.get_api_base());
}
EOF < /dev/null