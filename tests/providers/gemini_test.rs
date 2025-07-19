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
            index: Some(0),
            safety_ratings: None,
            citation_metadata: None,
        }],
        usage_metadata: Some(UsageMetadata {
            prompt_token_count: Some(10),
            candidates_token_count: Some(15),
            total_token_count: Some(25),
        }),
        prompt_feedback: None,
        error: None,
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
        prompt_feedback: None,
        error: None,
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

    let events = stream_response
        .to_anthropic_events("gemini-pro", "msg_123")
        .unwrap();

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
    println!("Error: {:?}", result);

    if let Err(error) = result {
        assert!(error.to_string().contains("Invalid role 'system': must be 'user' or 'assistant'"));
    }
}

#[test]
fn test_gemini_safety_settings_creation() {
    let safety_settings = GeminiRequest::default_safety_settings();
    assert_eq!(safety_settings.len(), 4);
    assert!(matches!(
        safety_settings[0].category,
        HarmCategory::Harassment
    ));
    assert!(matches!(
        safety_settings[0].threshold,
        HarmBlockThreshold::BlockMediumAndAbove
    ));
}

#[test]
fn test_gemini_custom_safety_settings() {
    let request = GeminiRequest::new(vec![], 100)
        .with_safety_setting(HarmCategory::Harassment, HarmBlockThreshold::BlockNone)
        .with_safety_setting(
            HarmCategory::HateSpeech,
            HarmBlockThreshold::BlockLowAndAbove,
        );

    let settings = request.safety_settings.unwrap();
    assert_eq!(settings.len(), 2);
    assert!(matches!(
        settings[0].threshold,
        HarmBlockThreshold::BlockNone
    ));
    assert!(matches!(
        settings[1].threshold,
        HarmBlockThreshold::BlockLowAndAbove
    ));
}

#[test]
fn test_gemini_safety_info_logging() {
    let gemini_response = GeminiResponse {
        error: None,
        candidates: vec![GeminiCandidate {
            content: GeminiContent {
                role: "model".to_string(),
                parts: vec![GeminiPart {
                    text: "Safe content".to_string(),
                }],
            },
            finish_reason: Some("STOP".to_string()),
            safety_ratings: Some(vec![SafetyRating {
                category: HarmCategory::Harassment,
                probability: HarmProbability::Low,
                blocked: Some(false),
            }]),
            index: Some(0),
            citation_metadata: None,
        }],
        usage_metadata: Some(UsageMetadata {
            prompt_token_count: Some(10),
            candidates_token_count: Some(5),
            total_token_count: Some(15),
        }),
        prompt_feedback: None,
    };

    let safety_info = gemini_response.get_safety_info();
    assert!(safety_info.contains("Harassment"));
    assert!(safety_info.contains("Low"));
    assert!(safety_info.contains("blocked: false"));
}

#[test]
fn test_gemini_utils_create_simple_request() {
    use ai_proxy::providers::gemini::model::gemini_utils;

    let request = gemini_utils::create_simple_request("Hello Gemini".to_string(), 100);
    assert_eq!(request.contents.len(), 1);
    assert_eq!(request.contents[0].role, "user");
    assert_eq!(request.contents[0].parts[0].text, "Hello Gemini");
    assert_eq!(request.generation_config.max_output_tokens, 100);
}

#[test]
fn test_gemini_utils_create_conversation_request() {
    use ai_proxy::providers::gemini::model::gemini_utils;

    let messages = vec![
        ("user".to_string(), "Hello".to_string()),
        ("assistant".to_string(), "Hi there".to_string()),
        ("user".to_string(), "How are you?".to_string()),
    ];

    let request = gemini_utils::create_conversation_request(messages, 100).unwrap();
    assert_eq!(request.contents.len(), 3);
    assert_eq!(request.contents[0].role, "user");
    assert_eq!(request.contents[1].role, "model"); // Gemini uses "model" for assistant
    assert_eq!(request.contents[2].role, "user");
}

#[test]
fn test_gemini_utils_parse_safety_settings() {
    use ai_proxy::providers::gemini::model::gemini_utils;

    let config = vec![
        ("harassment", "block_none"),
        ("hate_speech", "block_low_and_above"),
    ];

    let settings = gemini_utils::parse_safety_settings(&config).unwrap();
    assert_eq!(settings.len(), 2);
    assert!(matches!(settings[0].category, HarmCategory::Harassment));
    assert!(matches!(
        settings[0].threshold,
        HarmBlockThreshold::BlockNone
    ));
    assert!(matches!(settings[1].category, HarmCategory::HateSpeech));
    assert!(matches!(
        settings[1].threshold,
        HarmBlockThreshold::BlockLowAndAbove
    ));
}

#[test]
fn test_gemini_utils_extract_text_content() {
    use ai_proxy::providers::gemini::model::gemini_utils;

    let response = GeminiResponse {
        candidates: vec![GeminiCandidate {
            content: GeminiContent {
                role: "model".to_string(),
                parts: vec![GeminiPart {
                    text: "Extracted content".to_string(),
                }],
            },
            finish_reason: Some("STOP".to_string()),
            index: Some(0),
            safety_ratings: None,
            citation_metadata: None,
        }],
        usage_metadata: None,
        prompt_feedback: None,
        error: None,
    };

    let text = gemini_utils::extract_text_content(&response).unwrap();
    assert_eq!(text, "Extracted content");
}

#[test]
fn test_gemini_has_high_risk_safety_rating() {
    let response = GeminiResponse {
        candidates: vec![GeminiCandidate {
            content: GeminiContent {
                role: "model".to_string(),
                parts: vec![GeminiPart {
                    text: "Content".to_string(),
                }],
            },
            finish_reason: Some("STOP".to_string()),
            safety_ratings: Some(vec![SafetyRating {
                category: HarmCategory::Harassment,
                probability: HarmProbability::High,
                blocked: Some(false),
            }]),
            index: Some(0),
            citation_metadata: None,
        }],
        usage_metadata: None,
        prompt_feedback: None,
        error: None,
    };

    assert!(response.has_high_risk_safety_rating());
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
                .contains("Gemini API error")
        );
    }
}

#[tokio::test]
async fn test_gemini_provider_list_models_success() {
    // Start a mock server
    let mock_server = MockServer::start().await;

    // Mock the Gemini models API response
    Mock::given(method("GET"))
        .and(path_regex(r".*/models"))
        .and(query_param("key", "test-api-key"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "models": [
                {
                    "name": "models/gemini-1.5-pro-latest",
                    "displayName": "Gemini 1.5 Pro Latest",
                    "description": "The latest version of Gemini 1.5 Pro"
                },
                {
                    "name": "models/gemini-1.5-flash-latest",
                    "displayName": "Gemini 1.5 Flash Latest",
                    "description": "The latest version of Gemini 1.5 Flash"
                },
                {
                    "name": "models/gemini-pro",
                    "displayName": "Gemini Pro",
                    "description": "Gemini Pro model"
                }
            ]
        })))
        .mount(&mock_server)
        .await;

    // Create provider configuration
    let config = ProviderDetail {
        api_key: "test-api-key".to_string(),
        api_base: format!("{}/v1beta/", mock_server.uri()),
        models: Some(vec![
            "gemini-1.5-pro-latest".to_string(),
            "gemini-1.5-flash-latest".to_string(),
            "gemini-pro".to_string(),
        ]),
        enabled: true,
        max_retries: 3,
        rate_limit: None,
        timeout_seconds: 60,
    };

    // Create provider instance
    let client = Client::new();
    let provider = GeminiProvider::new(config, client);

    // Test the list_models method
    let models = provider.list_models().await.unwrap();



    // Verify response
    assert_eq!(models.len(), 3);
    assert_eq!(models[0].id, "gemini-1.5-pro-latest");
    assert_eq!(models[0].object, "model");
    assert_eq!(models[0].owned_by, "google");
    assert_eq!(models[1].id, "gemini-1.5-flash-latest");
    assert_eq!(models[2].id, "gemini-pro");
}

#[tokio::test]
async fn test_gemini_provider_list_models_api_error_fallback() {
    // Start a mock server
    let mock_server = MockServer::start().await;

    // Mock an API error response
    Mock::given(method("GET"))
        .and(path_regex(r".*/models"))
        .and(query_param("key", "test-api-key"))
        .respond_with(ResponseTemplate::new(401).set_body_json(json!({
            "error": {
                "code": 401,
                "message": "Invalid API key"
            }
        })))
        .mount(&mock_server)
        .await;

    // Create provider configuration with fallback models
    let config = ProviderDetail {
        api_key: "test-api-key".to_string(),
        api_base: format!("{}/v1beta/", mock_server.uri()),
        models: Some(vec![
            "gemini-1.5-pro-latest".to_string(),
            "gemini-1.5-flash-latest".to_string(),
            "gemini-pro".to_string(),
        ]),
        enabled: true,
        max_retries: 3,
        rate_limit: None,
        timeout_seconds: 60,
    };

    // Create provider instance
    let client = Client::new();
    let provider = GeminiProvider::new(config, client);

    // Test the list_models method - should fall back to configured models
    let models = provider.list_models().await.unwrap();

    // Verify response uses fallback models
    assert_eq!(models.len(), 3);
    assert_eq!(models[0].id, "gemini-1.5-pro-latest");
    assert_eq!(models[0].object, "model");
    assert_eq!(models[0].owned_by, "google");
    assert_eq!(models[1].id, "gemini-1.5-flash-latest");
    assert_eq!(models[2].id, "gemini-pro");
}

#[tokio::test]
async fn test_gemini_provider_list_models_no_config_fallback() {
    // Start a mock server
    let mock_server = MockServer::start().await;

    // Mock an API error response
    Mock::given(method("GET"))
        .and(path_regex(r".*/models"))
        .and(query_param("key", "test-api-key"))
        .respond_with(ResponseTemplate::new(500).set_body_json(json!({
            "error": {
                "code": 500,
                "message": "Internal server error"
            }
        })))
        .mount(&mock_server)
        .await;

    // Create provider configuration without models (should use default fallback)
    let config = ProviderDetail {
        api_key: "test-api-key".to_string(),
        api_base: format!("{}/v1beta/", mock_server.uri()),
        models: None, // No configured models
        enabled: true,
        max_retries: 3,
        rate_limit: None,
        timeout_seconds: 60,
    };

    // Create provider instance
    let client = Client::new();
    let provider = GeminiProvider::new(config, client);

    // Test the list_models method - should use default fallback models
    let models = provider.list_models().await.unwrap();

    // Verify response uses default fallback models
    assert_eq!(models.len(), 3);
    assert_eq!(models[0].id, "gemini-1.5-pro-latest");
    assert_eq!(models[1].id, "gemini-1.5-flash-latest");
    assert_eq!(models[2].id, "gemini-pro");
}

#[tokio::test]
async fn test_gemini_provider_health_check_success() {
    // Start a mock server
    let mock_server = MockServer::start().await;

    // Mock the Gemini models API response for health check
    Mock::given(method("GET"))
        .and(path_regex(r".*/models"))
        .and(query_param("key", "test-api-key"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "models": [
                {
                    "name": "models/gemini-pro",
                    "displayName": "Gemini Pro",
                    "description": "Gemini Pro model"
                }
            ]
        })))
        .mount(&mock_server)
        .await;

    // Create provider configuration
    let config = ProviderDetail {
        api_key: "test-api-key".to_string(),
        api_base: format!("{}/v1beta/", mock_server.uri()),
        models: Some(vec!["gemini-pro".to_string()]),
        enabled: true,
        max_retries: 3,
        rate_limit: None,
        timeout_seconds: 60,
    };

    // Create provider instance
    let client = Client::new();
    let provider = GeminiProvider::new(config, client);

    // Test the health_check method
    let health = provider.health_check().await.unwrap();

    // Verify response
    assert_eq!(health.status, "healthy");
    assert_eq!(health.provider, "gemini");
    assert!(health.latency_ms.is_some());
    assert!(health.latency_ms.unwrap() > 0);
    assert!(health.error.is_none());
}

#[tokio::test]
async fn test_gemini_provider_health_check_api_error() {
    // Start a mock server
    let mock_server = MockServer::start().await;

    // Mock an API error response
    Mock::given(method("GET"))
        .and(path_regex(r".*/models"))
        .and(query_param("key", "test-api-key"))
        .respond_with(ResponseTemplate::new(401).set_body_json(json!({
            "error": {
                "code": 401,
                "message": "Invalid API key"
            }
        })))
        .mount(&mock_server)
        .await;

    // Create provider configuration
    let config = ProviderDetail {
        api_key: "test-api-key".to_string(),
        api_base: format!("{}/v1beta/", mock_server.uri()),
        models: Some(vec!["gemini-pro".to_string()]),
        enabled: true,
        max_retries: 3,
        rate_limit: None,
        timeout_seconds: 60,
    };

    // Create provider instance
    let client = Client::new();
    let provider = GeminiProvider::new(config, client);

    // Test the health_check method
    let health = provider.health_check().await.unwrap();

    // Verify response indicates unhealthy status
    assert_eq!(health.status, "unhealthy");
    assert_eq!(health.provider, "gemini");
    assert!(health.latency_ms.is_some());
    assert!(health.error.is_some());
    assert_eq!(health.error.unwrap(), "HTTP 401 Unauthorized");
}

#[tokio::test]
async fn test_gemini_provider_health_check_network_error() {
    // Create provider configuration with invalid URL
    let config = ProviderDetail {
        api_key: "test-api-key".to_string(),
        api_base: "http://invalid-url-that-does-not-exist:9999/v1beta/".to_string(),
        models: Some(vec!["gemini-pro".to_string()]),
        enabled: true,
        max_retries: 3,
        rate_limit: None,
        timeout_seconds: 60,
    };

    // Create provider instance
    let client = Client::new();
    let provider = GeminiProvider::new(config, client);

    // Test the health_check method
    let health = provider.health_check().await.unwrap();

    // Verify response indicates unhealthy status due to network error
    assert_eq!(health.status, "unhealthy");
    assert_eq!(health.provider, "gemini");
    assert!(health.latency_ms.is_some());
    assert!(health.error.is_some());
    // Network error should contain connection-related message
    let error_msg = health.error.unwrap();
    assert!(error_msg.contains("error") || error_msg.contains("connection") || error_msg.contains("resolve") || error_msg.contains("HTTP"));
}
