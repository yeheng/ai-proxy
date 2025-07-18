use ai_proxy::config::ProviderDetail;
use ai_proxy::providers::{AIProvider, anthropic::*, gemini::*};
use reqwest::Client;
use serde_json::json;
use wiremock::matchers::{method, path_regex, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[test]
fn test_gemini_request_from_anthropic() {
    let anthropic_request = AnthropicRequest {
        model: "gemini-pro".to_string(),
        messages: vec![
            Message::user("Hello".to_string()),
            Message::assistant("Hi there!".to_string()),
        ],
        max_tokens: 100,
        stream: Some(false),
        temperature: Some(0.7),
        top_p: Some(0.9),
    };

    let gemini_request = GeminiRequest::from_anthropic(&anthropic_request).unwrap();

    assert_eq!(gemini_request.contents.len(), 2);
    assert_eq!(gemini_request.contents[0].role, "user");
    assert_eq!(gemini_request.contents[0].parts[0].text, "Hello");
    assert_eq!(gemini_request.contents[1].role, "model");
    assert_eq!(gemini_request.contents[1].parts[0].text, "Hi there!");
    assert_eq!(gemini_request.generation_config.max_output_tokens, 100);
    assert_eq!(gemini_request.generation_config.temperature, Some(0.7));
    assert_eq!(gemini_request.generation_config.top_p, Some(0.9));
}

#[test]
fn test_gemini_request_invalid_role() {
    let anthropic_request = AnthropicRequest {
        model: "gemini-pro".to_string(),
        messages: vec![Message {
            role: "system".to_string(),
            content: "You are a helpful assistant".to_string(),
        }],
        max_tokens: 100,
        stream: None,
        temperature: None,
        top_p: None,
    };

    let result = GeminiRequest::from_anthropic(&anthropic_request);
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("Invalid role: system")
    );
}

#[test]
fn test_gemini_response_to_anthropic() {
    let gemini_response = GeminiResponse {
        candidates: vec![GeminiCandidate {
            content: GeminiContent {
                role: "model".to_string(),
                parts: vec![GeminiPart {
                    text: "Hello! How can I help you?".to_string(),
                }],
            },
            finish_reason: Some("STOP".to_string()),
        }],
        usage_metadata: Some(UsageMetadata {
            prompt_token_count: Some(10),
            candidates_token_count: Some(15),
            total_token_count: Some(25),
        }),
    };

    let anthropic_response = gemini_response.to_anthropic("gemini-pro").unwrap();

    assert_eq!(anthropic_response.model, "gemini-pro");
    assert_eq!(anthropic_response.content.len(), 1);
    assert_eq!(
        anthropic_response.content[0].text,
        "Hello! How can I help you?"
    );
    assert_eq!(anthropic_response.usage.input_tokens, 10);
    assert_eq!(anthropic_response.usage.output_tokens, 15);
}

#[test]
fn test_gemini_response_no_candidates() {
    let gemini_response = GeminiResponse {
        candidates: vec![],
        usage_metadata: None,
    };

    let result = gemini_response.to_anthropic("gemini-pro");
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("No candidates in Gemini response")
    );
}

#[test]
fn test_gemini_stream_response_to_events() {
    let stream_response = GeminiStreamResponse {
        candidates: Some(vec![GeminiStreamCandidate {
            content: Some(GeminiContent {
                role: "model".to_string(),
                parts: vec![GeminiPart {
                    text: "Hello".to_string(),
                }],
            }),
            finish_reason: Some("STOP".to_string()),
            index: Some(0),
        }]),
        usage_metadata: Some(UsageMetadata {
            prompt_token_count: Some(5),
            candidates_token_count: Some(10),
            total_token_count: Some(15),
        }),
    };

    let events = stream_response.to_anthropic_events("gemini-pro", "msg_123");

    assert_eq!(events.len(), 3); // ContentBlockDelta, MessageDelta, MessageStop

    // Check first event is ContentBlockDelta
    if let AnthropicStreamEvent::ContentBlockDelta { index, delta } = &events[0] {
        assert_eq!(*index, 0);
        assert_eq!(delta.text, "Hello");
    } else {
        panic!("Expected ContentBlockDelta event");
    }

    // Check second event is MessageDelta
    if let AnthropicStreamEvent::MessageDelta { delta } = &events[1] {
        assert_eq!(delta.stop_reason, Some("end_turn".to_string()));
        assert!(delta.usage.is_some());
    } else {
        panic!("Expected MessageDelta event");
    }

    // Check third event is MessageStop
    assert!(matches!(events[2], AnthropicStreamEvent::MessageStop));
}

#[test]
fn test_create_message_start_event() {
    let event = GeminiStreamResponse::create_message_start_event("gemini-pro", "msg_123");

    if let AnthropicStreamEvent::MessageStart { message } = event {
        assert_eq!(message.id, "msg_123");
        assert_eq!(message.model, "gemini-pro");
        assert_eq!(message.role, "assistant");
    } else {
        panic!("Expected MessageStart event");
    }
}

#[test]
fn test_create_content_block_start_event() {
    let event = GeminiStreamResponse::create_content_block_start_event();

    if let AnthropicStreamEvent::ContentBlockStart {
        index,
        content_block,
    } = event
    {
        assert_eq!(index, 0);
        assert_eq!(content_block.type_field, "text");
        assert_eq!(content_block.text, "");
    } else {
        panic!("Expected ContentBlockStart event");
    }
}

#[tokio::test]
async fn test_gemini_provider_chat_success() {
    // Start a mock server
    let mock_server = MockServer::start().await;

    // Mock the Gemini API response
    Mock::given(method("POST"))
        .and(path_regex(r"/gemini-pro:generateContent"))
        .and(query_param("key", "test-api-key"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "candidates": [{
                "content": {
                    "role": "model",
                    "parts": [{
                        "text": "Hello! How can I help you today?"
                    }]
                },
                "finishReason": "STOP"
            }],
            "usageMetadata": {
                "promptTokenCount": 10,
                "candidatesTokenCount": 15,
                "totalTokenCount": 25
            }
        })))
        .mount(&mock_server)
        .await;

    // Create provider configuration
    let config = ProviderDetail {
        api_key: "test-api-key".to_string(),
        api_base: mock_server.uri(),
        models: Some(vec!["gemini-pro".to_string()]),
        enabled: true,
        max_retries: 3,
        rate_limit: None,
        timeout_seconds: 60,
    };

    // Create provider instance
    let client = Client::new();
    let provider = GeminiProvider::new(config, client);

    // Create test request
    let request = AnthropicRequest {
        model: "gemini-pro".to_string(),
        messages: vec![Message::user("Hello".to_string())],
        max_tokens: 100,
        stream: Some(false),
        temperature: Some(0.7),
        top_p: Some(0.9),
    };

    // Test the chat method
    let response = provider.chat(request).await.unwrap();

    // Verify response
    assert_eq!(response.model, "gemini-pro");
    assert_eq!(response.content.len(), 1);
    assert_eq!(response.content[0].text, "Hello! How can I help you today?");
    assert_eq!(response.usage.input_tokens, 10);
    assert_eq!(response.usage.output_tokens, 15);
}

#[tokio::test]
async fn test_gemini_provider_chat_api_error() {
    // Start a mock server
    let mock_server = MockServer::start().await;

    // Mock an API error response
    Mock::given(method("POST"))
        .and(path_regex(r"/gemini-pro:generateContent"))
        .and(query_param("key", "test-api-key"))
        .respond_with(ResponseTemplate::new(400).set_body_json(json!({
            "error": {
                "code": 400,
                "message": "Invalid request format"
            }
        })))
        .mount(&mock_server)
        .await;

    // Create provider configuration
    let config = ProviderDetail {
        api_key: "test-api-key".to_string(),
        api_base: mock_server.uri(),
        models: Some(vec!["gemini-pro".to_string()]),
        enabled: true,
        max_retries: 3,
        rate_limit: None,
        timeout_seconds: 60,
    };

    // Create provider instance
    let client = Client::new();
    let provider = GeminiProvider::new(config, client);

    // Create test request
    let request = AnthropicRequest {
        model: "gemini-pro".to_string(),
        messages: vec![Message::user("Hello".to_string())],
        max_tokens: 100,
        stream: None,
        temperature: None,
        top_p: None,
    };

    // Test the chat method - should return error
    let result = provider.chat(request).await;
    assert!(result.is_err());

    if let Err(error) = result {
        assert!(error.to_string().contains("Gemini API error"));
    }
}

#[tokio::test]
async fn test_gemini_provider_chat_validation_error() {
    // Create provider configuration (no need for mock server since validation happens first)
    let config = ProviderDetail {
        api_key: "test-api-key".to_string(),
        api_base: "http://localhost:8080".to_string(),
        models: Some(vec!["gemini-pro".to_string()]),
        enabled: true,
        max_retries: 3,
        rate_limit: None,
        timeout_seconds: 60,
    };

    // Create provider instance
    let client = Client::new();
    let provider = GeminiProvider::new(config, client);

    // Create invalid request (empty messages)
    let request = AnthropicRequest {
        model: "gemini-pro".to_string(),
        messages: vec![], // Invalid: empty messages
        max_tokens: 100,
        stream: None,
        temperature: None,
        top_p: None,
    };

    // Test the chat method - should return validation error
    let result = provider.chat(request).await;
    assert!(result.is_err());

    if let Err(error) = result {
        assert!(error.to_string().contains("Messages cannot be empty"));
    }
}

#[tokio::test]
async fn test_gemini_provider_chat_conversion_error() {
    // Create provider configuration (no need for mock server since conversion happens first)
    let config = ProviderDetail {
        api_key: "test-api-key".to_string(),
        api_base: "http://localhost:8080".to_string(),
        models: Some(vec!["gemini-pro".to_string()]),
        enabled: true,
        max_retries: 3,
        rate_limit: None,
        timeout_seconds: 60,
    };

    // Create provider instance
    let client = Client::new();
    let provider = GeminiProvider::new(config, client);

    // Create request with invalid role for Gemini
    let request = AnthropicRequest {
        model: "gemini-pro".to_string(),
        messages: vec![Message {
            role: "system".to_string(), // Invalid for Gemini
            content: "You are a helpful assistant".to_string(),
        }],
        max_tokens: 100,
        stream: None,
        temperature: None,
        top_p: None,
    };

    // Test the chat method - should return conversion error
    let result = provider.chat(request).await;
    assert!(result.is_err());

    if let Err(error) = result {
        assert!(error.to_string().contains("Invalid role: system"));
    }
}

#[tokio::test]
async fn test_gemini_provider_chat_network_error() {
    // Create provider configuration with invalid URL
    let config = ProviderDetail {
        api_key: "test-api-key".to_string(),
        api_base: "http://invalid-url-that-does-not-exist:9999".to_string(),
        models: Some(vec!["gemini-pro".to_string()]),
        enabled: true,
        max_retries: 3,
        rate_limit: None,
        timeout_seconds: 60,
    };

    // Create provider instance
    let client = Client::new();
    let provider = GeminiProvider::new(config, client);

    // Create valid request
    let request = AnthropicRequest {
        model: "gemini-pro".to_string(),
        messages: vec![Message::user("Hello".to_string())],
        max_tokens: 100,
        stream: None,
        temperature: None,
        top_p: None,
    };

    // Test the chat method - should return network error
    let result = provider.chat(request).await;
    assert!(result.is_err());

    if let Err(error) = result {
        assert!(
            error
                .to_string()
                .contains("Failed to send request to Gemini")
        );
    }
}
