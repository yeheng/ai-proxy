use reqwest::Client;
use serde_json::json;
use wiremock::{
    matchers::{header, method, path},
    Mock, MockServer, ResponseTemplate,
};

use ai_proxy::{
    config::ProviderDetail,
    errors::AppError,
    providers::{
        AIProvider,
        anthropic::{AnthropicRequest, Message},
        openai::{OpenAIProvider, openai_utils},
    },
};

/// Create a test provider configuration
fn create_test_config(api_base: &str) -> ProviderDetail {
    ProviderDetail {
        api_key: "test-api-key".to_string(),
        api_base: format!("{}/", api_base.trim_end_matches('/')),
        models: Some(vec![
            "gpt-4".to_string(),
            "gpt-3.5-turbo".to_string(),
        ]),
        timeout_seconds: 30,
        max_retries: 3,
        enabled: true,
        rate_limit: None,
    }
}

/// Create a test Anthropic request
fn create_test_request() -> AnthropicRequest {
    AnthropicRequest {
        model: "gpt-4".to_string(),
        messages: vec![Message {
            role: "user".to_string(),
            content: "Hello, world!".to_string(),
        }],
        max_tokens: 100,
        stream: Some(false),
        temperature: Some(0.7),
        top_p: Some(0.9),
    }
}

/// Create a mock OpenAI chat completion response
fn create_mock_chat_response() -> serde_json::Value {
    json!({
        "id": "chatcmpl-test123",
        "object": "chat.completion",
        "created": 1714560000,
        "model": "gpt-4",
        "choices": [{
            "index": 0,
            "message": {
                "role": "assistant",
                "content": "Hello! How can I help you today?"
            },
            "finish_reason": "stop"
        }],
        "usage": {
            "prompt_tokens": 10,
            "completion_tokens": 15,
            "total_tokens": 25
        }
    })
}

/// Create a mock OpenAI models list response
fn create_mock_models_response() -> serde_json::Value {
    json!({
        "object": "list",
        "data": [
            {
                "id": "gpt-4",
                "object": "model",
                "created": 1714560000,
                "owned_by": "openai"
            },
            {
                "id": "gpt-3.5-turbo",
                "object": "model",
                "created": 1714560000,
                "owned_by": "openai"
            },
            {
                "id": "text-embedding-ada-002",
                "object": "model",
                "created": 1714560000,
                "owned_by": "openai"
            }
        ]
    })
}

#[tokio::test]
async fn test_openai_chat_success() {
    // Setup mock server
    let mock_server = MockServer::start().await;
    let config = create_test_config(&mock_server.uri());
    let client = Client::new();
    let provider = OpenAIProvider::new(config, client);

    // Setup mock response
    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .and(header("authorization", "Bearer test-api-key"))
        .respond_with(ResponseTemplate::new(200).set_body_json(create_mock_chat_response()))
        .mount(&mock_server)
        .await;

    // Test the chat method
    let request = create_test_request();
    let response = provider.chat(request).await.unwrap();

    // Verify response
    assert_eq!(response.model, "gpt-4");
    assert!(!response.content.is_empty());
    assert_eq!(response.usage.input_tokens, 10);
    assert_eq!(response.usage.output_tokens, 15);
}

#[tokio::test]
async fn test_openai_chat_api_error() {
    // Setup mock server
    let mock_server = MockServer::start().await;
    let config = create_test_config(&mock_server.uri());
    let client = Client::new();
    let provider = OpenAIProvider::new(config, client);

    // Setup mock error response
    let error_response = json!({
        "error": {
            "message": "Invalid API key",
            "type": "invalid_request_error",
            "code": "invalid_api_key"
        }
    });

    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(ResponseTemplate::new(401).set_body_json(error_response))
        .mount(&mock_server)
        .await;

    // Test the chat method with error
    let request = create_test_request();
    let result = provider.chat(request).await;

    // Verify error handling
    assert!(result.is_err());
    match result.unwrap_err() {
        AppError::ProviderError { status, message } => {
            assert_eq!(status, 401);
            assert!(message.contains("authentication failed"));
        }
        _ => panic!("Expected ProviderError"),
    }
}

#[tokio::test]
async fn test_openai_list_models_success() {
    // Setup mock server
    let mock_server = MockServer::start().await;
    let config = create_test_config(&mock_server.uri());
    let client = Client::new();
    let provider = OpenAIProvider::new(config, client);

    // Setup mock response
    Mock::given(method("GET"))
        .and(path("/models"))
        .and(header("authorization", "Bearer test-api-key"))
        .respond_with(ResponseTemplate::new(200).set_body_json(create_mock_models_response()))
        .mount(&mock_server)
        .await;

    // Test the list_models method
    let models = provider.list_models().await.unwrap();

    // Verify response - should filter out embedding models
    assert_eq!(models.len(), 2); // Only chat models, not embedding
    assert!(models.iter().any(|m| m.id == "gpt-4"));
    assert!(models.iter().any(|m| m.id == "gpt-3.5-turbo"));
    assert!(!models.iter().any(|m| m.id == "text-embedding-ada-002"));
}

#[tokio::test]
async fn test_openai_list_models_fallback() {
    // Setup mock server that returns error
    let mock_server = MockServer::start().await;
    let config = create_test_config(&mock_server.uri());
    let client = Client::new();
    let provider = OpenAIProvider::new(config, client);

    // Setup mock error response
    Mock::given(method("GET"))
        .and(path("/models"))
        .respond_with(ResponseTemplate::new(500))
        .mount(&mock_server)
        .await;

    // Test the list_models method with fallback
    let models = provider.list_models().await.unwrap();

    // Verify fallback models are returned
    assert!(!models.is_empty());
    assert!(models.iter().any(|m| m.id == "gpt-4"));
    assert!(models.iter().any(|m| m.id == "gpt-3.5-turbo"));
}

#[tokio::test]
async fn test_openai_health_check_success() {
    // Setup mock server
    let mock_server = MockServer::start().await;
    let config = create_test_config(&mock_server.uri());
    let client = Client::new();
    let provider = OpenAIProvider::new(config, client);

    // Setup mock response
    Mock::given(method("GET"))
        .and(path("/models"))
        .and(header("authorization", "Bearer test-api-key"))
        .respond_with(ResponseTemplate::new(200).set_body_json(create_mock_models_response()))
        .mount(&mock_server)
        .await;

    // Test the health_check method
    let health = provider.health_check().await.unwrap();

    // Verify health status
    assert_eq!(health.status, "healthy");
    assert_eq!(health.provider, "openai");
    assert!(health.latency_ms.is_some());
    assert!(health.error.is_none());
}

#[tokio::test]
async fn test_openai_health_check_failure() {
    // Setup mock server that returns error
    let mock_server = MockServer::start().await;
    let config = create_test_config(&mock_server.uri());
    let client = Client::new();
    let provider = OpenAIProvider::new(config, client);

    // Setup mock error response
    Mock::given(method("GET"))
        .and(path("/models"))
        .respond_with(ResponseTemplate::new(401))
        .mount(&mock_server)
        .await;

    // Test the health_check method with failure
    let health = provider.health_check().await.unwrap();

    // Verify error health status
    assert_eq!(health.status, "unhealthy");
    assert_eq!(health.provider, "openai");
    assert!(health.latency_ms.is_some());
    assert!(health.error.is_some());
}

#[tokio::test]
async fn test_openai_streaming_response_parsing() {
    // Test streaming response conversion
    let streaming_data = r#"{"id":"chatcmpl-test","object":"chat.completion.chunk","created":1714560000,"model":"gpt-4","choices":[{"index":0,"delta":{"content":"Hello"},"finish_reason":null}]}"#;
    
    let stream_response: ai_proxy::providers::openai::OpenAIStreamResponse = 
        serde_json::from_str(streaming_data).unwrap();
    
    // Test conversion to Anthropic events
    let events = stream_response.to_anthropic_events("msg_123").unwrap();
    
    // Verify events
    assert!(!events.is_empty());
    // Should contain content delta event
    assert!(events.iter().any(|e| matches!(e, ai_proxy::providers::anthropic::AnthropicStreamEvent::ContentBlockDelta { .. })));
}

#[test]
fn test_openai_request_validation() {
    // Test valid request
    let valid_request = openai_utils::create_simple_request(
        "Hello".to_string(),
        "gpt-4".to_string(),
        100
    );
    assert!(valid_request.validate().is_ok());
    
    // Test invalid model name
    let mut invalid_request = openai_utils::create_simple_request(
        "Hello".to_string(),
        "".to_string(),
        100
    );
    assert!(invalid_request.validate().is_err());
    
    // Test invalid max_tokens
    invalid_request = openai_utils::create_simple_request(
        "Hello".to_string(),
        "gpt-4".to_string(),
        0
    );
    assert!(invalid_request.validate().is_err());
    
    // Test invalid temperature
    let invalid_request = valid_request.clone().with_temperature(-1.0);
    assert!(invalid_request.validate().is_err());
    
    // Test invalid top_p
    let invalid_request = valid_request.clone().with_top_p(2.0);
    assert!(invalid_request.validate().is_err());
}

#[test]
fn test_openai_utils_functions() {
    // Test model name validation
    assert!(openai_utils::validate_model_name("gpt-4").is_ok());
    assert!(openai_utils::validate_model_name("gpt-3.5-turbo").is_ok());
    assert!(openai_utils::validate_model_name("invalid-model").is_err());
    assert!(openai_utils::validate_model_name("").is_err());
    
    // Test streaming support check
    assert!(openai_utils::supports_streaming("gpt-4"));
    assert!(openai_utils::supports_streaming("gpt-3.5-turbo"));
    assert!(!openai_utils::supports_streaming("text-embedding-ada-002"));
    assert!(!openai_utils::supports_streaming("whisper-1"));
    
    // Test recommended max tokens
    assert_eq!(openai_utils::get_recommended_max_tokens("gpt-4"), 4096);
    assert_eq!(openai_utils::get_recommended_max_tokens("gpt-3.5-turbo-16k"), 16384);
    assert_eq!(openai_utils::get_recommended_max_tokens("unknown-model"), 2048);
}

#[test]
fn test_openai_conversation_request() {
    // Test conversation request creation
    let messages = vec![
        ("system".to_string(), "You are a helpful assistant.".to_string()),
        ("user".to_string(), "Hello!".to_string()),
        ("assistant".to_string(), "Hi there!".to_string()),
        ("user".to_string(), "How are you?".to_string()),
    ];
    
    let request = openai_utils::create_conversation_request(
        messages,
        "gpt-4".to_string(),
        100
    ).unwrap();
    
    assert_eq!(request.messages.len(), 4);
    assert_eq!(request.messages[0].role, "system");
    assert_eq!(request.messages[1].role, "user");
    assert_eq!(request.messages[2].role, "assistant");
    assert_eq!(request.messages[3].role, "user");
    
    // Test invalid role
    let invalid_messages = vec![
        ("invalid_role".to_string(), "Test".to_string()),
    ];
    
    let result = openai_utils::create_conversation_request(
        invalid_messages,
        "gpt-4".to_string(),
        100
    );
    
    assert!(result.is_err());
}

#[test]
fn test_openai_error_parsing() {
    // Test error response parsing
    let error_json = r#"{"error":{"message":"Invalid API key","type":"invalid_request_error","code":"invalid_api_key"}}"#;
    let parsed_error = openai_utils::parse_error_response(error_json);
    assert_eq!(parsed_error, "Invalid API key");
    
    // Test malformed error response
    let malformed_json = "not valid json";
    let parsed_error = openai_utils::parse_error_response(malformed_json);
    assert_eq!(parsed_error, "not valid json");
}

#[test]
fn test_openai_response_conversion() {
    use ai_proxy::providers::openai::{OpenAIResponse, OpenAIChoice, OpenAIMessage, OpenAIUsage};
    
    // Create a test OpenAI response
    let openai_response = OpenAIResponse {
        id: "test-id".to_string(),
        object: "chat.completion".to_string(),
        created: 1714560000,
        model: "gpt-4".to_string(),
        choices: vec![OpenAIChoice {
            index: 0,
            message: OpenAIMessage {
                role: "assistant".to_string(),
                content: "Hello, world!".to_string(),
                name: None,
            },
            finish_reason: Some("stop".to_string()),
            logprobs: None,
        }],
        usage: OpenAIUsage {
            prompt_tokens: 10,
            completion_tokens: 15,
            total_tokens: 25,
        },
        system_fingerprint: None,
    };
    
    // Test conversion to Anthropic format
    let anthropic_response = openai_response.to_anthropic().unwrap();
    
    assert_eq!(anthropic_response.id, "test-id");
    assert_eq!(anthropic_response.model, "gpt-4");
    assert!(!anthropic_response.content.is_empty());
    assert_eq!(anthropic_response.usage.input_tokens, 10);
    assert_eq!(anthropic_response.usage.output_tokens, 15);
    
    // Test finish reason parsing
    let finish_reason = openai_response.get_finish_reason().unwrap();
    assert!(finish_reason.contains("completed naturally"));
    
    // Test usage info
    let usage_info = openai_response.get_usage_info();
    assert!(usage_info.contains("prompt_tokens: 10"));
    assert!(usage_info.contains("completion_tokens: 15"));
    assert!(usage_info.contains("total_tokens: 25"));
    
    // Test response validation
    assert!(!openai_response.has_issues());
}