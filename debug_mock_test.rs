use ai_proxy::{
    config::{Config, ServerConfig, ProviderDetail, LoggingConfig, SecurityConfig, PerformanceConfig},
    server::create_app,
    providers::{ProviderRegistry},
};
use std::collections::HashMap;

#[tokio::main]
async fn main() {
    // Setup mock server
    let mock_server = wiremock::MockServer::start().await;
    
    println!("Mock server URL: {}", mock_server.uri());
    
    // Create mock response
    let streaming_response = vec![
        "data: {\"id\":\"chatcmpl-stream123\",\"object\":\"chat.completion.chunk\",\"created\":1234567890,\"model\":\"gpt-4\",\"choices\":[{\"index\":0,\"delta\":{\"role\":\"assistant\"},\"finish_reason\":null}]}",
        "data: {\"id\":\"chatcmpl-stream123\",\"object\":\"chat.completion.chunk\",\"created\":1234567890,\"model\":\"gpt-4\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\"Hello\"},\"finish_reason\":null}]}",
        "data: {\"id\":\"chatcmpl-stream123\",\"object\":\"chat.completion.chunk\",\"created\":1234567890,\"model\":\"gpt-4\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\" there!\"},\"finish_reason\":null}]}",
        "data: {\"id\":\"chatcmpl-stream123\",\"object\":\"chat.completion.chunk\",\"created\":1234567890,\"model\":\"gpt-4\",\"choices\":[{\"index\":0,\"delta\":{},\"finish_reason\":\"stop\"}]}",
        "data: [DONE]",
        ""
    ].join("\n\n");
    
    wiremock::Mock::given(wiremock::matchers::method("POST"))
        .and(wiremock::matchers::path("/v1/chat/completions"))
        .respond_with(
            wiremock::ResponseTemplate::new(200)
                .set_body_string(streaming_response)
                .insert_header("content-type", "text/event-stream")
                .insert_header("cache-control", "no-cache")
                .insert_header("connection", "keep-alive")
        )
        .mount(&mock_server)
        .await;
    
    // Create test configuration
    let mut providers = HashMap::new();
    providers.insert(
        "openai".to_string(),
        ProviderDetail {
            api_key: "test-openai-key-1234567890".to_string(),
            api_base: format!("{}/v1/", mock_server.uri()),
            models: Some(vec!["gpt-4".to_string(), "gpt-3.5-turbo".to_string()]),
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
    
    let app_state = ai_proxy::server::AppState::new(config).await.unwrap();
    
    // Check what URLs are configured
    let registry = app_state.provider_registry.lock().await;
    let provider = registry.get_provider("gpt-4").unwrap();
    
    println!("Provider API base: {}", provider.get_api_base());
}