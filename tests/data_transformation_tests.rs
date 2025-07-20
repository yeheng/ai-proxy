use ai_proxy::
    providers::{
        anthropic::{AnthropicRequest, AnthropicResponse, Message, SSEEvent, AnthropicStreamEvent},
        openai::{OpenAIRequest, OpenAIResponse, OpenAIMessage, OpenAIChoice, OpenAIUsage},
        gemini::{GeminiRequest, GeminiResponse, GeminiContent, GeminiPart, GeminiCandidate, UsageMetadata, GeminiStreamResponse, GeminiStreamCandidate},
    }
;

// Test data transformation functions for all providers

#[test]
fn test_anthropic_request_validation_valid() {
    let request = AnthropicRequest {
        model: "claude-3-sonnet".to_string(),
        messages: vec![Message::user("Hello".to_string())],
        max_tokens: 100,
        stream: Some(false),
        temperature: Some(0.7),
        top_p: Some(0.9),
    };
    
    assert!(request.validate().is_ok());
}

#[test]
fn test_anthropic_request_validation_empty_model() {
    let request = AnthropicRequest {
        model: "".to_string(),
        messages: vec![Message::user("Hello".to_string())],
        max_tokens: 100,
        stream: None,
        temperature: None,
        top_p: None,
    };
    
    let result = request.validate();
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Model name cannot be empty"));
}

#[test]
fn test_anthropic_request_validation_invalid_model_chars() {
    let request = AnthropicRequest {
        model: "claude@3#sonnet".to_string(),
        messages: vec![Message::user("Hello".to_string())],
        max_tokens: 100,
        stream: None,
        temperature: None,
        top_p: None,
    };
    
    let result = request.validate();
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Model name contains invalid characters"));
}

#[test]
fn test_anthropic_request_validation_empty_messages() {
    let request = AnthropicRequest {
        model: "claude-3-sonnet".to_string(),
        messages: vec![],
        max_tokens: 100,
        stream: None,
        temperature: None,
        top_p: None,
    };
    
    let result = request.validate();
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Messages cannot be empty"));
}

#[test]
fn test_anthropic_request_validation_too_many_messages() {
    let messages = (0..101).map(|i| Message::user(format!("Message {}", i))).collect();
    let request = AnthropicRequest {
        model: "claude-3-sonnet".to_string(),
        messages,
        max_tokens: 100,
        stream: None,
        temperature: None,
        top_p: None,
    };
    
    let result = request.validate();
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Too many messages"));
}

#[test]
fn test_anthropic_request_validation_zero_max_tokens() {
    let request = AnthropicRequest {
        model: "claude-3-sonnet".to_string(),
        messages: vec![Message::user("Hello".to_string())],
        max_tokens: 0,
        stream: None,
        temperature: None,
        top_p: None,
    };
    
    let result = request.validate();
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("max_tokens must be greater than 0"));
}

#[test]
fn test_anthropic_request_validation_excessive_max_tokens() {
    let request = AnthropicRequest {
        model: "claude-3-sonnet".to_string(),
        messages: vec![Message::user("Hello".to_string())],
        max_tokens: 10000,
        stream: None,
        temperature: None,
        top_p: None,
    };
    
    let result = request.validate();
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("max_tokens cannot exceed 8192"));
}

#[test]
fn test_anthropic_request_validation_invalid_temperature() {
    let request = AnthropicRequest {
        model: "claude-3-sonnet".to_string(),
        messages: vec![Message::user("Hello".to_string())],
        max_tokens: 100,
        stream: None,
        temperature: Some(-1.0),
        top_p: None,
    };
    
    let result = request.validate();
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("temperature must be between 0.0 and 2.0"));
    
    let request = AnthropicRequest {
        model: "claude-3-sonnet".to_string(),
        messages: vec![Message::user("Hello".to_string())],
        max_tokens: 100,
        stream: None,
        temperature: Some(3.0),
        top_p: None,
    };
    
    let result = request.validate();
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("temperature must be between 0.0 and 2.0"));
}

#[test]
fn test_anthropic_request_validation_invalid_top_p() {
    let request = AnthropicRequest {
        model: "claude-3-sonnet".to_string(),
        messages: vec![Message::user("Hello".to_string())],
        max_tokens: 100,
        stream: None,
        temperature: None,
        top_p: Some(-0.1),
    };
    
    let result = request.validate();
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("top_p must be between 0.0 and 1.0"));
    
    let request = AnthropicRequest {
        model: "claude-3-sonnet".to_string(),
        messages: vec![Message::user("Hello".to_string())],
        max_tokens: 100,
        stream: None,
        temperature: None,
        top_p: Some(1.5),
    };
    
    let result = request.validate();
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("top_p must be between 0.0 and 1.0"));
}

#[test]
fn test_anthropic_request_validation_excessive_content_length() {
    let long_content = "a".repeat(50_001);
    let request = AnthropicRequest {
        model: "claude-3-sonnet".to_string(),
        messages: vec![
            Message::user(long_content.clone()),
            Message::assistant(long_content),
        ],
        max_tokens: 100,
        stream: None,
        temperature: None,
        top_p: None,
    };
    
    let result = request.validate();
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Total content length exceeds maximum"));
}

#[test]
fn test_anthropic_request_is_streaming() {
    let mut request = AnthropicRequest {
        model: "claude-3-sonnet".to_string(),
        messages: vec![Message::user("Hello".to_string())],
        max_tokens: 100,
        stream: None,
        temperature: None,
        top_p: None,
    };
    
    assert!(!request.is_streaming());
    
    request.stream = Some(false);
    assert!(!request.is_streaming());
    
    request.stream = Some(true);
    assert!(request.is_streaming());
}

#[test]
fn test_anthropic_request_estimate_input_tokens() {
    let request = AnthropicRequest {
        model: "claude-3-sonnet".to_string(),
        messages: vec![
            Message::user("Hello world".to_string()), // ~11 chars + 4 (role) = 15 chars
            Message::assistant("Hi there".to_string()), // ~8 chars + 9 (role) = 17 chars
        ],
        max_tokens: 100,
        stream: None,
        temperature: None,
        top_p: None,
    };
    
    let estimated = request.estimate_input_tokens();
    // Total ~32 chars / 4 = 8 tokens
    assert!(estimated >= 8);
    assert!(estimated <= 10); // Allow some variance
}

#[test]
fn test_message_validation_valid() {
    let user_msg = Message::user("Hello".to_string());
    assert!(user_msg.validate().is_ok());
    
    let assistant_msg = Message::assistant("Hi there".to_string());
    assert!(assistant_msg.validate().is_ok());
}

#[test]
fn test_message_validation_invalid_role() {
    let msg = Message {
        role: "system".to_string(),
        content: "Hello".to_string(),
    };
    
    let result = msg.validate();
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Invalid role 'system'"));
}

#[test]
fn test_message_validation_empty_content() {
    let msg = Message {
        role: "user".to_string(),
        content: "".to_string(),
    };
    
    let result = msg.validate();
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Message content cannot be empty"));
}

#[test]
fn test_message_validation_content_too_long() {
    let long_content = "a".repeat(100_001);
    let msg = Message {
        role: "user".to_string(),
        content: long_content,
    };
    
    let result = msg.validate();
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Message content too long"));
}

#[test]
fn test_message_validation_null_bytes() {
    let msg = Message {
        role: "user".to_string(),
        content: "Hello\0World".to_string(),
    };
    
    let result = msg.validate();
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Message content cannot contain null bytes"));
}

#[test]
fn test_anthropic_response_creation() {
    let response = AnthropicResponse::new(
        "msg_123".to_string(),
        "claude-3-sonnet".to_string(),
        "Hello! How can I help you?".to_string(),
        10,
        25,
    );
    
    assert_eq!(response.id, "msg_123");
    assert_eq!(response.model, "claude-3-sonnet");
    assert_eq!(response.content.len(), 1);
    assert_eq!(response.content[0].type_field, "text");
    assert_eq!(response.content[0].text, "Hello! How can I help you?");
    assert_eq!(response.usage.input_tokens, 10);
    assert_eq!(response.usage.output_tokens, 25);
}

#[test]
fn test_sse_event_formatting() {
    let event = SSEEvent::new("Hello World".to_string());
    let formatted = event.to_sse_string();
    assert_eq!(formatted, "data: Hello World\n\n");
    
    let event_with_type = SSEEvent::with_event("message".to_string(), "Hello World".to_string());
    let formatted = event_with_type.to_sse_string();
    assert_eq!(formatted, "event: message\ndata: Hello World\n\n");
}

#[test]
fn test_sse_event_multiline_data() {
    let multiline_data = "Line 1\nLine 2\nLine 3".to_string();
    let event = SSEEvent::new(multiline_data);
    let formatted = event.to_sse_string();
    assert_eq!(formatted, "data: Line 1\ndata: Line 2\ndata: Line 3\n\n");
}

// OpenAI transformation tests

#[test]
fn test_openai_request_from_anthropic() {
    let anthropic_request = AnthropicRequest {
        model: "gpt-4".to_string(),
        messages: vec![
            Message::user("Hello".to_string()),
            Message::assistant("Hi there".to_string()),
        ],
        max_tokens: 100,
        stream: Some(true),
        temperature: Some(0.7),
        top_p: Some(0.9),
    };
    
    let openai_request = OpenAIRequest::from_anthropic(&anthropic_request).unwrap();
    
    assert_eq!(openai_request.model, "gpt-4");
    assert_eq!(openai_request.messages.len(), 2);
    assert_eq!(openai_request.messages[0].role, "user");
    assert_eq!(openai_request.messages[0].content, "Hello");
    assert_eq!(openai_request.messages[1].role, "assistant");
    assert_eq!(openai_request.messages[1].content, "Hi there");
    assert_eq!(openai_request.max_tokens, 100);
    assert_eq!(openai_request.stream, Some(true));
    assert_eq!(openai_request.temperature, Some(0.7));
    assert_eq!(openai_request.top_p, Some(0.9));
}

#[test]
fn test_openai_request_builder_methods() {
    let request = OpenAIRequest::new(
        "gpt-4".to_string(),
        vec![OpenAIMessage {
            role: "user".to_string(),
            content: "Hello".to_string(),
            name: None,
        }],
        100,
    )
    .with_stream(true)
    .with_temperature(0.8)
    .with_top_p(0.95)
    .with_frequency_penalty(0.1)
    .with_presence_penalty(0.2)
    .with_stop(vec!["STOP".to_string()])
    .with_user("user123".to_string());
    
    assert_eq!(request.stream, Some(true));
    assert_eq!(request.temperature, Some(0.8));
    assert_eq!(request.top_p, Some(0.95));
    assert_eq!(request.frequency_penalty, Some(0.1));
    assert_eq!(request.presence_penalty, Some(0.2));
    assert_eq!(request.stop, Some(vec!["STOP".to_string()]));
    assert_eq!(request.user, Some("user123".to_string()));
}

#[test]
fn test_openai_request_validation() {
    let valid_request = OpenAIRequest::new(
        "gpt-4".to_string(),
        vec![OpenAIMessage {
            role: "user".to_string(),
            content: "Hello".to_string(),
            name: None,
        }],
        100,
    );
    
    assert!(valid_request.validate().is_ok());
    
    // Test empty model
    let mut invalid_request = valid_request.clone();
    invalid_request.model = "".to_string();
    assert!(invalid_request.validate().is_err());
    
    // Test empty messages
    let mut invalid_request = valid_request.clone();
    invalid_request.messages = vec![];
    assert!(invalid_request.validate().is_err());
    
    // Test zero max_tokens
    let mut invalid_request = valid_request.clone();
    invalid_request.max_tokens = 0;
    assert!(invalid_request.validate().is_err());
    
    // Test excessive max_tokens
    let mut invalid_request = valid_request.clone();
    invalid_request.max_tokens = 5000;
    assert!(invalid_request.validate().is_err());
    
    // Test invalid temperature
    let mut invalid_request = valid_request.clone();
    invalid_request.temperature = Some(-1.0);
    assert!(invalid_request.validate().is_err());
    
    // Test invalid top_p
    let mut invalid_request = valid_request.clone();
    invalid_request.top_p = Some(2.0);
    assert!(invalid_request.validate().is_err());
    
    // Test invalid frequency_penalty
    let mut invalid_request = valid_request.clone();
    invalid_request.frequency_penalty = Some(-3.0);
    assert!(invalid_request.validate().is_err());
    
    // Test invalid presence_penalty
    let mut invalid_request = valid_request.clone();
    invalid_request.presence_penalty = Some(3.0);
    assert!(invalid_request.validate().is_err());
    
    // Test too many stop sequences
    let mut invalid_request = valid_request.clone();
    invalid_request.stop = Some(vec!["1".to_string(), "2".to_string(), "3".to_string(), "4".to_string(), "5".to_string()]);
    assert!(invalid_request.validate().is_err());
}

#[test]
fn test_openai_response_to_anthropic() {
    let openai_response = OpenAIResponse {
        id: "chatcmpl-123".to_string(),
        object: "chat.completion".to_string(),
        created: 1234567890,
        model: "gpt-4".to_string(),
        choices: vec![OpenAIChoice {
            index: 0,
            message: OpenAIMessage {
                role: "assistant".to_string(),
                content: "Hello! How can I help you?".to_string(),
                name: None,
            },
            finish_reason: Some("stop".to_string()),
            logprobs: None,
        }],
        usage: OpenAIUsage {
            prompt_tokens: 10,
            completion_tokens: 25,
            total_tokens: 35,
        },
        system_fingerprint: None,
    };
    
    let anthropic_response = openai_response.to_anthropic().unwrap();
    
    assert_eq!(anthropic_response.id, "chatcmpl-123");
    assert_eq!(anthropic_response.model, "gpt-4");
    assert_eq!(anthropic_response.content.len(), 1);
    assert_eq!(anthropic_response.content[0].text, "Hello! How can I help you?");
    assert_eq!(anthropic_response.usage.input_tokens, 10);
    assert_eq!(anthropic_response.usage.output_tokens, 25);
}

#[test]
fn test_openai_response_to_anthropic_no_choices() {
    let openai_response = OpenAIResponse {
        id: "chatcmpl-123".to_string(),
        object: "chat.completion".to_string(),
        created: 1234567890,
        model: "gpt-4".to_string(),
        choices: vec![],
        usage: OpenAIUsage {
            prompt_tokens: 10,
            completion_tokens: 0,
            total_tokens: 10,
        },
        system_fingerprint: None,
    };
    
    let result = openai_response.to_anthropic();
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("No choices in OpenAI response"));
}

#[test]
fn test_openai_response_to_anthropic_empty_content() {
    let openai_response = OpenAIResponse {
        id: "chatcmpl-123".to_string(),
        object: "chat.completion".to_string(),
        created: 1234567890,
        model: "gpt-4".to_string(),
        choices: vec![OpenAIChoice {
            index: 0,
            message: OpenAIMessage {
                role: "assistant".to_string(),
                content: "".to_string(),
                name: None,
            },
            finish_reason: Some("stop".to_string()),
            logprobs: None,
        }],
        usage: OpenAIUsage {
            prompt_tokens: 10,
            completion_tokens: 0,
            total_tokens: 10,
        },
        system_fingerprint: None,
    };
    
    let result = openai_response.to_anthropic();
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Empty response content from OpenAI"));
}

#[test]
fn test_openai_response_get_finish_reason() {
    let openai_response = OpenAIResponse {
        id: "chatcmpl-123".to_string(),
        object: "chat.completion".to_string(),
        created: 1234567890,
        model: "gpt-4".to_string(),
        choices: vec![OpenAIChoice {
            index: 0,
            message: OpenAIMessage {
                role: "assistant".to_string(),
                content: "Hello".to_string(),
                name: None,
            },
            finish_reason: Some("stop".to_string()),
            logprobs: None,
        }],
        usage: OpenAIUsage {
            prompt_tokens: 10,
            completion_tokens: 5,
            total_tokens: 15,
        },
        system_fingerprint: None,
    };
    
    let finish_reason = openai_response.get_finish_reason().unwrap();
    assert!(finish_reason.contains("completed naturally"));
}

#[test]
fn test_openai_response_get_usage_info() {
    let openai_response = OpenAIResponse {
        id: "chatcmpl-123".to_string(),
        object: "chat.completion".to_string(),
        created: 1234567890,
        model: "gpt-4".to_string(),
        choices: vec![OpenAIChoice {
            index: 0,
            message: OpenAIMessage {
                role: "assistant".to_string(),
                content: "Hello".to_string(),
                name: None,
            },
            finish_reason: Some("stop".to_string()),
            logprobs: None,
        }],
        usage: OpenAIUsage {
            prompt_tokens: 10,
            completion_tokens: 5,
            total_tokens: 15,
        },
        system_fingerprint: None,
    };
    
    let usage_info = openai_response.get_usage_info();
    assert!(usage_info.contains("prompt_tokens: 10"));
    assert!(usage_info.contains("completion_tokens: 5"));
    assert!(usage_info.contains("total_tokens: 15"));
}

#[test]
fn test_openai_response_has_issues() {
    // Response with no issues
    let good_response = OpenAIResponse {
        id: "chatcmpl-123".to_string(),
        object: "chat.completion".to_string(),
        created: 1234567890,
        model: "gpt-4".to_string(),
        choices: vec![OpenAIChoice {
            index: 0,
            message: OpenAIMessage {
                role: "assistant".to_string(),
                content: "Hello".to_string(),
                name: None,
            },
            finish_reason: Some("stop".to_string()),
            logprobs: None,
        }],
        usage: OpenAIUsage {
            prompt_tokens: 10,
            completion_tokens: 5,
            total_tokens: 15,
        },
        system_fingerprint: None,
    };
    
    assert!(!good_response.has_issues());
    
    // Response with no choices
    let bad_response = OpenAIResponse {
        id: "chatcmpl-123".to_string(),
        object: "chat.completion".to_string(),
        created: 1234567890,
        model: "gpt-4".to_string(),
        choices: vec![],
        usage: OpenAIUsage {
            prompt_tokens: 10,
            completion_tokens: 0,
            total_tokens: 10,
        },
        system_fingerprint: None,
    };
    
    assert!(bad_response.has_issues());
    
    // Response with empty content
    let empty_response = OpenAIResponse {
        id: "chatcmpl-123".to_string(),
        object: "chat.completion".to_string(),
        created: 1234567890,
        model: "gpt-4".to_string(),
        choices: vec![OpenAIChoice {
            index: 0,
            message: OpenAIMessage {
                role: "assistant".to_string(),
                content: "".to_string(),
                name: None,
            },
            finish_reason: Some("stop".to_string()),
            logprobs: None,
        }],
        usage: OpenAIUsage {
            prompt_tokens: 10,
            completion_tokens: 0,
            total_tokens: 10,
        },
        system_fingerprint: None,
    };
    
    assert!(empty_response.has_issues());
}

// Gemini transformation tests

#[test]
fn test_gemini_request_from_anthropic() {
    let anthropic_request = AnthropicRequest {
        model: "gemini-pro".to_string(),
        messages: vec![
            Message::user("Hello".to_string()),
            Message::assistant("Hi there".to_string()),
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
    assert_eq!(gemini_request.contents[1].role, "model"); // Gemini uses "model" for assistant
    assert_eq!(gemini_request.contents[1].parts[0].text, "Hi there");
    assert_eq!(gemini_request.generation_config.max_output_tokens, 100);
    assert_eq!(gemini_request.generation_config.temperature, Some(0.7));
    assert_eq!(gemini_request.generation_config.top_p, Some(0.9));
}

#[test]
fn test_gemini_request_from_anthropic_invalid_role() {
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
    assert!(result.unwrap_err().to_string().contains("Invalid role: system"));
}

#[test]
fn test_gemini_request_validation() {
    let valid_request = GeminiRequest::new(
        vec![GeminiContent {
            role: "user".to_string(),
            parts: vec![GeminiPart {
                text: "Hello".to_string(),
            }],
        }],
        100,
    );
    
    assert!(valid_request.validate().is_ok());
    
    // Test empty contents
    let mut invalid_request = valid_request.clone();
    invalid_request.contents = vec![];
    assert!(invalid_request.validate().is_err());
    
    // Test zero max_output_tokens
    let mut invalid_request = valid_request.clone();
    invalid_request.generation_config.max_output_tokens = 0;
    assert!(invalid_request.validate().is_err());
    
    // Test excessive max_output_tokens
    let mut invalid_request = valid_request.clone();
    invalid_request.generation_config.max_output_tokens = 10000;
    assert!(invalid_request.validate().is_err());
    
    // Test invalid temperature
    let mut invalid_request = valid_request.clone();
    invalid_request.generation_config.temperature = Some(-1.0);
    assert!(invalid_request.validate().is_err());
    
    // Test invalid top_p
    let mut invalid_request = valid_request.clone();
    invalid_request.generation_config.top_p = Some(2.0);
    assert!(invalid_request.validate().is_err());
    
    // Test invalid top_k
    let mut invalid_request = valid_request.clone();
    invalid_request.generation_config.top_k = Some(0);
    assert!(invalid_request.validate().is_err());
    
    // Test invalid candidate_count
    let mut invalid_request = valid_request.clone();
    invalid_request.generation_config.candidate_count = Some(0);
    assert!(invalid_request.validate().is_err());
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
            candidates_token_count: Some(25),
            total_token_count: Some(35),
        }),
        prompt_feedback: None,
        error: None,
    };
    
    let anthropic_response = gemini_response.to_anthropic("gemini-pro").unwrap();
    
    assert_eq!(anthropic_response.model, "gemini-pro");
    assert_eq!(anthropic_response.content.len(), 1);
    assert_eq!(anthropic_response.content[0].text, "Hello! How can I help you?");
    assert_eq!(anthropic_response.usage.input_tokens, 10);
    assert_eq!(anthropic_response.usage.output_tokens, 25);
}

#[test]
fn test_gemini_response_to_anthropic_no_candidates() {
    let gemini_response = GeminiResponse {
        candidates: vec![],
        usage_metadata: None,
        prompt_feedback: None,
        error: None,
    };
    
    let result = gemini_response.to_anthropic("gemini-pro");
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("No candidates in Gemini response"));
}

#[test]
fn test_gemini_response_get_finish_reason() {
    let gemini_response = GeminiResponse {
        candidates: vec![GeminiCandidate {
            content: GeminiContent {
                role: "model".to_string(),
                parts: vec![GeminiPart {
                    text: "Hello".to_string(),
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
    
    let finish_reason = gemini_response.get_finish_reason().unwrap();
    assert!(finish_reason.contains("completed naturally"));
}

#[test]
fn test_gemini_response_get_usage_info() {
    let gemini_response = GeminiResponse {
        candidates: vec![GeminiCandidate {
            content: GeminiContent {
                role: "model".to_string(),
                parts: vec![GeminiPart {
                    text: "Hello".to_string(),
                }],
            },
            finish_reason: Some("STOP".to_string()),
            index: Some(0),
            safety_ratings: None,
            citation_metadata: None,
        }],
        usage_metadata: Some(UsageMetadata {
            prompt_token_count: Some(10),
            candidates_token_count: Some(5),
            total_token_count: Some(15),
        }),
        prompt_feedback: None,
        error: None,
    };
    
    let usage_info = gemini_response.get_usage_info();
    assert!(usage_info.contains("prompt_tokens: 10"));
    assert!(usage_info.contains("completion_tokens: 5"));
    assert!(usage_info.contains("total_tokens: 15"));
}

#[test]
fn test_gemini_response_has_safety_issues() {
    use ai_proxy::providers::gemini::{SafetyRating, HarmCategory, HarmProbability};
    
    // Response with no safety issues
    let safe_response = GeminiResponse {
        candidates: vec![GeminiCandidate {
            content: GeminiContent {
                role: "model".to_string(),
                parts: vec![GeminiPart {
                    text: "Hello".to_string(),
                }],
            },
            finish_reason: Some("STOP".to_string()),
            index: Some(0),
            safety_ratings: Some(vec![SafetyRating {
                category: HarmCategory::Harassment,
                probability: HarmProbability::Low,
                blocked: Some(false),
            }]),
            citation_metadata: None,
        }],
        usage_metadata: None,
        prompt_feedback: None,
        error: None,
    };
    
    assert!(!safe_response.has_safety_issues());
    
    // Response with blocked content
    let blocked_response = GeminiResponse {
        candidates: vec![GeminiCandidate {
            content: GeminiContent {
                role: "model".to_string(),
                parts: vec![GeminiPart {
                    text: "Hello".to_string(),
                }],
            },
            finish_reason: Some("STOP".to_string()),
            index: Some(0),
            safety_ratings: Some(vec![SafetyRating {
                category: HarmCategory::Harassment,
                probability: HarmProbability::High,
                blocked: Some(true),
            }]),
            citation_metadata: None,
        }],
        usage_metadata: None,
        prompt_feedback: None,
        error: None,
    };
    
    assert!(blocked_response.has_safety_issues());
}

#[test]
fn test_gemini_stream_response_to_anthropic_events() {
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
    
    let events = stream_response.to_anthropic_events("gemini-pro", "msg_123").unwrap();
    
    assert!(!events.is_empty());
    // Should contain content delta and message stop events
    assert!(events.iter().any(|e| matches!(e, AnthropicStreamEvent::ContentBlockDelta { .. })));
    assert!(events.iter().any(|e| matches!(e, AnthropicStreamEvent::MessageStop)));
}