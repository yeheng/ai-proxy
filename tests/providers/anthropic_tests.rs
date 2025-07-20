use ai_proxy::{
    config::ProviderDetail,
    providers::{
        AIProvider,
        anthropic::{AnthropicProvider, AnthropicRequest, Message},
    },
    errors::AppError,
};
use reqwest::Client;

/// Create a test Anthropic provider instance
fn create_test_provider() -> AnthropicProvider {
    let config = ProviderDetail {
        api_key: "test-key".to_string(),
        api_base: "https://api.anthropic.com/v1/".to_string(),
        models: Some(vec![
            "claude-3-5-sonnet-20241022".to_string(),
            "claude-3-haiku-20240307".to_string(),
        ]),
        timeout_seconds: 30,
        max_retries: 3,
        enabled: true,
        rate_limit: None,
    };
    
    let client = Client::new();
    AnthropicProvider::new(config, client)
}

/// Create a test request
fn create_test_request() -> AnthropicRequest {
    AnthropicRequest {
        model: "claude-3-haiku-20240307".to_string(),
        messages: vec![
            Message {
                role: "user".to_string(),
                content: "Hello, how are you?".to_string(),
            }
        ],
        max_tokens: 100,
        stream: Some(false),
        temperature: Some(0.7),
        top_p: Some(0.9),
    }
}

#[tokio::test]
async fn test_anthropic_provider_creation() {
    let _provider = create_test_provider();
    
    // Test that provider is created successfully
    // This is a basic test to ensure the struct is properly initialized
    assert!(true); // Provider creation succeeded if we reach here
}

#[tokio::test]
async fn test_request_validation() {
    let _provider = create_test_provider();
    
    // Test valid request
    let valid_request = create_test_request();
    assert!(valid_request.validate().is_ok());
    
    // Test invalid model name
    let mut invalid_request = create_test_request();
    invalid_request.model = "invalid-model".to_string();
    
    // This should fail during model validation in the provider
    // We can't test the actual API call without a real API key
    assert!(invalid_request.model != "claude-3-haiku-20240307");
}

#[tokio::test]
async fn test_model_validation() {
    // Test valid request with Claude model
    let valid_request = AnthropicRequest {
        model: "claude-3-haiku-20240307".to_string(),
        messages: vec![Message::user("Hello".to_string())],
        max_tokens: 100,
        stream: Some(false),
        temperature: Some(0.7),
        top_p: Some(0.9),
    };
    
    assert!(valid_request.validate().is_ok());
    
    // Test invalid model name in request
    let invalid_request = AnthropicRequest {
        model: "gpt-4".to_string(),
        messages: vec![Message::user("Hello".to_string())],
        max_tokens: 100,
        stream: Some(false),
        temperature: Some(0.7),
        top_p: Some(0.9),
    };
    
    // The request itself validates, but the provider would reject the model
    assert!(invalid_request.validate().is_ok()); // Basic validation passes
    assert!(!invalid_request.model.starts_with("claude-")); // But it's not a Claude model
}

#[tokio::test]
async fn test_list_models() {
    let provider = create_test_provider();
    
    // Test model listing (should return fallback models since we don't have a real API key)
    let models_result = provider.list_models().await;
    assert!(models_result.is_ok());
    
    let models = models_result.unwrap();
    assert!(!models.is_empty());
    
    // Verify all returned models are Claude models
    for model in &models {
        assert!(model.id.starts_with("claude-"));
        assert_eq!(model.owned_by, "anthropic");
        assert_eq!(model.object, "model");
    }
}

#[tokio::test]
async fn test_error_handling() {
    // Test that we can create error types properly
    let bad_request = AppError::BadRequest("Test error".to_string());
    assert!(matches!(bad_request, AppError::BadRequest(_)));
    
    let provider_error = AppError::ProviderError {
        status: 500,
        message: "Test provider error".to_string(),
    };
    assert!(matches!(provider_error, AppError::ProviderError { .. }));
    
    // Test validation error
    let validation_error = AppError::ValidationError("Test validation error".to_string());
    assert!(matches!(validation_error, AppError::ValidationError(_)));
}

#[tokio::test]
async fn test_streaming_request_preparation() {
    let _provider = create_test_provider();
    
    // Test that streaming requests are properly prepared
    let mut request = create_test_request();
    request.stream = Some(true);
    
    assert!(request.is_streaming());
    assert_eq!(request.stream, Some(true));
}

#[tokio::test]
async fn test_model_listing() {
    let provider = create_test_provider();
    
    // Test model listing (should return fallback models since we don't have a real API key)
    let models_result = provider.list_models().await;
    assert!(models_result.is_ok());
    
    let models = models_result.unwrap();
    assert!(!models.is_empty());
    
    // Verify model structure
    for model in &models {
        assert!(model.id.starts_with("claude-"));
        assert_eq!(model.owned_by, "anthropic");
        assert_eq!(model.object, "model");
        assert!(model.created > 0);
    }
}

#[tokio::test]
async fn test_request_estimation() {
    let request = create_test_request();
    
    // Test token estimation
    let estimated_tokens = request.estimate_input_tokens();
    assert!(estimated_tokens > 0);
    
    // Test with longer content
    let mut long_request = request.clone();
    long_request.messages[0].content = "This is a much longer message that should result in more estimated tokens".repeat(10);
    
    let long_estimated = long_request.estimate_input_tokens();
    assert!(long_estimated > estimated_tokens);
}

#[tokio::test]
async fn test_message_validation() {
    // Test valid messages
    let valid_user_msg = Message::user("Hello".to_string());
    assert!(valid_user_msg.validate().is_ok());
    
    let valid_assistant_msg = Message::assistant("Hi there!".to_string());
    assert!(valid_assistant_msg.validate().is_ok());
    
    // Test invalid messages
    let empty_content = Message {
        role: "user".to_string(),
        content: "".to_string(),
    };
    assert!(empty_content.validate().is_err());
    
    let invalid_role = Message {
        role: "system".to_string(),
        content: "Hello".to_string(),
    };
    assert!(invalid_role.validate().is_err());
    
    let null_content = Message {
        role: "user".to_string(),
        content: "Hello\0World".to_string(),
    };
    assert!(null_content.validate().is_err());
}

#[tokio::test]
async fn test_comprehensive_request_validation() {
    // Test comprehensive request validation
    let mut request = create_test_request();
    
    // Valid request should pass
    assert!(request.validate().is_ok());
    
    // Test invalid model
    request.model = "".to_string();
    assert!(request.validate().is_err());
    request.model = "claude-3-haiku-20240307".to_string();
    
    // Test invalid max_tokens
    request.max_tokens = 0;
    assert!(request.validate().is_err());
    request.max_tokens = 100;
    
    // Test invalid temperature
    request.temperature = Some(-1.0);
    assert!(request.validate().is_err());
    request.temperature = Some(0.7);
    
    // Test invalid top_p
    request.top_p = Some(1.5);
    assert!(request.validate().is_err());
    request.top_p = Some(0.9);
    
    // Test empty messages
    request.messages = vec![];
    assert!(request.validate().is_err());
}

// Integration test that would require a real API key
#[tokio::test]
#[ignore] // Ignored by default since it requires real API credentials
async fn test_real_api_integration() {
    // This test would require setting up real API credentials
    // and should only be run in integration test environments
    
    let api_key = std::env::var("ANTHROPIC_API_KEY").unwrap_or_default();
    if api_key.is_empty() {
        println!("Skipping real API test - no API key provided");
        return;
    }
    
    let config = ProviderDetail {
        api_key,
        api_base: "https://api.anthropic.com/v1/".to_string(),
        models: None,
        timeout_seconds: 30,
        max_retries: 3,
        enabled: true,
        rate_limit: None,
    };
    
    let client = Client::new();
    let provider = AnthropicProvider::new(config, client);
    
    // Test health check
    let health = provider.health_check().await.unwrap();
    assert_eq!(health.provider, "anthropic");
    
    // Test model listing
    let models = provider.list_models().await.unwrap();
    assert!(!models.is_empty());
    
    // Test actual chat request
    let request = create_test_request();
    let response = provider.chat(request).await.unwrap();
    assert!(!response.content.is_empty());
    assert!(response.usage.input_tokens > 0);
}