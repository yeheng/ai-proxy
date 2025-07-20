use ai_proxy::
    providers::{
        anthropic::{AnthropicRequest, AnthropicResponse, Message, Usage, SSEEvent, AnthropicStreamEvent},
        openai::{OpenAIRequest, OpenAIResponse, OpenAIMessage, OpenAIChoice, OpenAIUsage},
        gemini::{GeminiRequest, GeminiResponse, GeminiContent, GeminiPart, GeminiCandidate, UsageMetadata},
    }
;

/// Test edge cases and error conditions in data transformation functions
/// These tests cover scenarios that might not be covered in the main transformation tests

#[test]
fn test_anthropic_request_validation_edge_cases() {
    // Test with Unicode content
    let unicode_request = AnthropicRequest {
        model: "claude-3-sonnet".to_string(),
        messages: vec![Message::user("Hello 世界! 🌍".to_string())],
        max_tokens: 100,
        stream: None,
        temperature: None,
        top_p: None,
    };
    assert!(unicode_request.validate().is_ok());

    // Test with very long model name
    let long_model_request = AnthropicRequest {
        model: "a".repeat(101),
        messages: vec![Message::user("Hello".to_string())],
        max_tokens: 100,
        stream: None,
        temperature: None,
        top_p: None,
    };
    assert!(long_model_request.validate().is_err());

    // Test with special characters in model name
    let special_char_request = AnthropicRequest {
        model: "claude@3#sonnet".to_string(),
        messages: vec![Message::user("Hello".to_string())],
        max_tokens: 100,
        stream: None,
        temperature: None,
        top_p: None,
    };
    assert!(special_char_request.validate().is_err());

    // Test with NaN temperature
    let nan_temp_request = AnthropicRequest {
        model: "claude-3-sonnet".to_string(),
        messages: vec![Message::user("Hello".to_string())],
        max_tokens: 100,
        stream: None,
        temperature: Some(f32::NAN),
        top_p: None,
    };
    assert!(nan_temp_request.validate().is_err());

    // Test with infinite temperature
    let inf_temp_request = AnthropicRequest {
        model: "claude-3-sonnet".to_string(),
        messages: vec![Message::user("Hello".to_string())],
        max_tokens: 100,
        stream: None,
        temperature: Some(f32::INFINITY),
        top_p: None,
    };
    assert!(inf_temp_request.validate().is_err());
}

#[test]
fn test_message_validation_edge_cases() {
    // Test with whitespace-only content
    let whitespace_msg = Message {
        role: "user".to_string(),
        content: "   \n\t  ".to_string(),
    };
    // This should pass validation as whitespace is technically content
    assert!(whitespace_msg.validate().is_ok());

    // Test with very long content (exactly at limit)
    let max_content = "a".repeat(100_000);
    let max_msg = Message {
        role: "user".to_string(),
        content: max_content,
    };
    assert!(max_msg.validate().is_ok());

    // Test with content just over limit
    let over_limit_content = "a".repeat(100_001);
    let over_limit_msg = Message {
        role: "user".to_string(),
        content: over_limit_content,
    };
    assert!(over_limit_msg.validate().is_err());

    // Test with mixed case role
    let mixed_case_msg = Message {
        role: "User".to_string(),
        content: "Hello".to_string(),
    };
    assert!(mixed_case_msg.validate().is_err());

    // Test with control characters
    let control_char_msg = Message {
        role: "user".to_string(),
        content: "Hello\x01World".to_string(),
    };
    // Control characters should be allowed (only null bytes are forbidden)
    assert!(control_char_msg.validate().is_ok());
}

#[test]
fn test_openai_request_conversion_edge_cases() {
    // Test conversion with all optional parameters
    let full_anthropic_request = AnthropicRequest {
        model: "gpt-4".to_string(),
        messages: vec![
            Message::user("Hello".to_string()),
            Message::assistant("Hi there".to_string()),
            Message::user("How are you?".to_string()),
        ],
        max_tokens: 2048,
        stream: Some(true),
        temperature: Some(1.5),
        top_p: Some(0.1),
    };

    let openai_request = OpenAIRequest::from_anthropic(&full_anthropic_request).unwrap();
    assert_eq!(openai_request.model, "gpt-4");
    assert_eq!(openai_request.messages.len(), 3);
    assert_eq!(openai_request.max_tokens, 2048);
    assert_eq!(openai_request.stream, Some(true));
    assert_eq!(openai_request.temperature, Some(1.5));
    assert_eq!(openai_request.top_p, Some(0.1));

    // Test conversion with minimal parameters
    let minimal_anthropic_request = AnthropicRequest {
        model: "gpt-3.5-turbo".to_string(),
        messages: vec![Message::user("Test".to_string())],
        max_tokens: 1,
        stream: None,
        temperature: None,
        top_p: None,
    };

    let minimal_openai_request = OpenAIRequest::from_anthropic(&minimal_anthropic_request).unwrap();
    assert_eq!(minimal_openai_request.model, "gpt-3.5-turbo");
    assert_eq!(minimal_openai_request.messages.len(), 1);
    assert_eq!(minimal_openai_request.max_tokens, 1);
    assert_eq!(minimal_openai_request.stream, None);
    assert_eq!(minimal_openai_request.temperature, None);
    assert_eq!(minimal_openai_request.top_p, None);
}

#[test]
fn test_openai_response_conversion_edge_cases() {
    // Test response with multiple choices (should use first choice)
    let multi_choice_response = OpenAIResponse {
        id: "chatcmpl-123".to_string(),
        object: "chat.completion".to_string(),
        created: 1234567890,
        model: "gpt-4".to_string(),
        choices: vec![
            OpenAIChoice {
                index: 0,
                message: OpenAIMessage {
                    role: "assistant".to_string(),
                    content: "First choice".to_string(),
                    name: None,
                },
                finish_reason: Some("stop".to_string()),
                logprobs: None,
            },
            OpenAIChoice {
                index: 1,
                message: OpenAIMessage {
                    role: "assistant".to_string(),
                    content: "Second choice".to_string(),
                    name: None,
                },
                finish_reason: Some("stop".to_string()),
                logprobs: None,
            },
        ],
        usage: OpenAIUsage {
            prompt_tokens: 10,
            completion_tokens: 5,
            total_tokens: 15,
        },
        system_fingerprint: Some("fp_123".to_string()),
    };

    let anthropic_response = multi_choice_response.to_anthropic().unwrap();
    assert_eq!(anthropic_response.content[0].text, "First choice");

    // Test response with different finish reasons
    let length_finish_response = OpenAIResponse {
        id: "chatcmpl-456".to_string(),
        object: "chat.completion".to_string(),
        created: 1234567890,
        model: "gpt-4".to_string(),
        choices: vec![OpenAIChoice {
            index: 0,
            message: OpenAIMessage {
                role: "assistant".to_string(),
                content: "Truncated response".to_string(),
                name: None,
            },
            finish_reason: Some("length".to_string()),
            logprobs: None,
        }],
        usage: OpenAIUsage {
            prompt_tokens: 10,
            completion_tokens: 100,
            total_tokens: 110,
        },
        system_fingerprint: None,
    };

    let anthropic_response = length_finish_response.to_anthropic().unwrap();
    assert_eq!(anthropic_response.content[0].text, "Truncated response");

    // Test response with zero usage tokens
    let zero_usage_response = OpenAIResponse {
        id: "chatcmpl-789".to_string(),
        object: "chat.completion".to_string(),
        created: 1234567890,
        model: "gpt-4".to_string(),
        choices: vec![OpenAIChoice {
            index: 0,
            message: OpenAIMessage {
                role: "assistant".to_string(),
                content: "Response".to_string(),
                name: None,
            },
            finish_reason: Some("stop".to_string()),
            logprobs: None,
        }],
        usage: OpenAIUsage {
            prompt_tokens: 0,
            completion_tokens: 0,
            total_tokens: 0,
        },
        system_fingerprint: None,
    };

    let anthropic_response = zero_usage_response.to_anthropic().unwrap();
    assert_eq!(anthropic_response.usage.input_tokens, 0);
    assert_eq!(anthropic_response.usage.output_tokens, 0);
}

#[test]
fn test_gemini_request_conversion_edge_cases() {
    // Test conversion with system message (should fail)
    let system_message_request = AnthropicRequest {
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

    let result = GeminiRequest::from_anthropic(&system_message_request);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Invalid role: system"));

    // Test conversion with alternating roles
    let alternating_request = AnthropicRequest {
        model: "gemini-pro".to_string(),
        messages: vec![
            Message::user("Hello".to_string()),
            Message::assistant("Hi".to_string()),
            Message::user("How are you?".to_string()),
            Message::assistant("I'm good".to_string()),
        ],
        max_tokens: 100,
        stream: Some(false),
        temperature: Some(0.5),
        top_p: Some(0.8),
    };

    let gemini_request = GeminiRequest::from_anthropic(&alternating_request).unwrap();
    assert_eq!(gemini_request.contents.len(), 4);
    assert_eq!(gemini_request.contents[0].role, "user");
    assert_eq!(gemini_request.contents[1].role, "model");
    assert_eq!(gemini_request.contents[2].role, "user");
    assert_eq!(gemini_request.contents[3].role, "model");
}

#[test]
fn test_gemini_response_conversion_edge_cases() {
    // Test response with no usage metadata
    let no_usage_response = GeminiResponse {
        candidates: vec![GeminiCandidate {
            content: GeminiContent {
                role: "model".to_string(),
                parts: vec![GeminiPart {
                    text: "Response without usage".to_string(),
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

    let anthropic_response = no_usage_response.to_anthropic("gemini-pro").unwrap();
    assert_eq!(anthropic_response.usage.input_tokens, 0);
    assert_eq!(anthropic_response.usage.output_tokens, 0);

    // Test response with partial usage metadata
    let partial_usage_response = GeminiResponse {
        candidates: vec![GeminiCandidate {
            content: GeminiContent {
                role: "model".to_string(),
                parts: vec![GeminiPart {
                    text: "Partial usage response".to_string(),
                }],
            },
            finish_reason: Some("STOP".to_string()),
            index: Some(0),
            safety_ratings: None,
            citation_metadata: None,
        }],
        usage_metadata: Some(UsageMetadata {
            prompt_token_count: Some(15),
            candidates_token_count: None,
            total_token_count: Some(20),
        }),
        prompt_feedback: None,
        error: None,
    };

    let anthropic_response = partial_usage_response.to_anthropic("gemini-pro").unwrap();
    assert_eq!(anthropic_response.usage.input_tokens, 15);
    assert_eq!(anthropic_response.usage.output_tokens, 0); // candidates_token_count is None, defaults to 0

    // Test response with multiple parts in content
    let multi_part_response = GeminiResponse {
        candidates: vec![GeminiCandidate {
            content: GeminiContent {
                role: "model".to_string(),
                parts: vec![
                    GeminiPart {
                        text: "First part. ".to_string(),
                    },
                    GeminiPart {
                        text: "Second part.".to_string(),
                    },
                ],
            },
            finish_reason: Some("STOP".to_string()),
            index: Some(0),
            safety_ratings: None,
            citation_metadata: None,
        }],
        usage_metadata: Some(UsageMetadata {
            prompt_token_count: Some(5),
            candidates_token_count: Some(10),
            total_token_count: Some(15),
        }),
        prompt_feedback: None,
        error: None,
    };

    let anthropic_response = multi_part_response.to_anthropic("gemini-pro").unwrap();
    assert_eq!(anthropic_response.content[0].text, "First part. Second part.");
}

#[test]
fn test_sse_event_formatting_edge_cases() {
    // Test with empty data
    let empty_event = SSEEvent::new("".to_string());
    let formatted = empty_event.to_sse_string();
    // Empty string has no lines, so only the final newline is added
    assert_eq!(formatted, "\n");

    // Test with data containing colons
    let colon_event = SSEEvent::new("data: with: colons".to_string());
    let formatted = colon_event.to_sse_string();
    assert_eq!(formatted, "data: data: with: colons\n\n");

    // Test with data containing newlines at the end
    let trailing_newline_event = SSEEvent::new("Line 1\nLine 2\n".to_string());
    let formatted = trailing_newline_event.to_sse_string();
    // lines() doesn't include the final empty line after the trailing newline
    assert_eq!(formatted, "data: Line 1\ndata: Line 2\n\n");

    // Test with event type and ID
    let mut full_event = SSEEvent::with_event("message".to_string(), "Hello World".to_string());
    full_event.id = Some("event-123".to_string());
    let formatted = full_event.to_sse_string();
    assert_eq!(formatted, "event: message\nid: event-123\ndata: Hello World\n\n");

    // Test with Unicode in data
    let unicode_event = SSEEvent::new("Hello 世界! 🌍".to_string());
    let formatted = unicode_event.to_sse_string();
    assert_eq!(formatted, "data: Hello 世界! 🌍\n\n");
}

#[test]
fn test_anthropic_stream_event_serialization() {
    use ai_proxy::providers::anthropic::{StreamMessage, TextDelta, StreamError};

    // Test message start event
    let message_start = AnthropicStreamEvent::MessageStart {
        message: StreamMessage {
            id: "msg_123".to_string(),
            model: "claude-3-sonnet".to_string(),
            role: "assistant".to_string(),
            content: vec![],
            usage: Usage {
                input_tokens: 10,
                output_tokens: 0,
            },
        },
    };

    let serialized = serde_json::to_string(&message_start).unwrap();
    assert!(serialized.contains("message_start"));
    assert!(serialized.contains("msg_123"));

    // Test content block delta event
    let content_delta = AnthropicStreamEvent::ContentBlockDelta {
        index: 0,
        delta: TextDelta {
            type_field: "text_delta".to_string(),
            text: "Hello".to_string(),
        },
    };

    let serialized = serde_json::to_string(&content_delta).unwrap();
    assert!(serialized.contains("content_block_delta"));
    assert!(serialized.contains("Hello"));

    // Test error event
    let error_event = AnthropicStreamEvent::Error {
        error: StreamError {
            error_type: "rate_limit_error".to_string(),
            message: "Rate limit exceeded".to_string(),
        },
    };

    let serialized = serde_json::to_string(&error_event).unwrap();
    assert!(serialized.contains("error"));
    assert!(serialized.contains("rate_limit_error"));
}

#[test]
fn test_token_estimation_accuracy() {
    // Test token estimation with various content types
    let short_request = AnthropicRequest {
        model: "claude-3-sonnet".to_string(),
        messages: vec![Message::user("Hi".to_string())],
        max_tokens: 100,
        stream: None,
        temperature: None,
        top_p: None,
    };
    let short_tokens = short_request.estimate_input_tokens();
    assert!(short_tokens >= 1);
    assert!(short_tokens <= 5);

    let long_request = AnthropicRequest {
        model: "claude-3-sonnet".to_string(),
        messages: vec![Message::user("This is a much longer message that should result in more estimated tokens because it contains significantly more text content.".to_string())],
        max_tokens: 100,
        stream: None,
        temperature: None,
        top_p: None,
    };
    let long_tokens = long_request.estimate_input_tokens();
    assert!(long_tokens > short_tokens);
    assert!(long_tokens >= 20);

    // Test with multiple messages
    let multi_message_request = AnthropicRequest {
        model: "claude-3-sonnet".to_string(),
        messages: vec![
            Message::user("First message".to_string()),
            Message::assistant("Assistant response".to_string()),
            Message::user("Second user message".to_string()),
        ],
        max_tokens: 100,
        stream: None,
        temperature: None,
        top_p: None,
    };
    let multi_tokens = multi_message_request.estimate_input_tokens();
    assert!(multi_tokens > short_tokens);
}

#[test]
fn test_response_creation_edge_cases() {
    // Test response with empty text
    let empty_response = AnthropicResponse::new(
        "msg_empty".to_string(),
        "claude-3-sonnet".to_string(),
        "".to_string(),
        5,
        0,
    );
    assert_eq!(empty_response.content[0].text, "");
    assert_eq!(empty_response.usage.output_tokens, 0);

    // Test response with very long text
    let long_text = "a".repeat(10000);
    let long_response = AnthropicResponse::new(
        "msg_long".to_string(),
        "claude-3-sonnet".to_string(),
        long_text.clone(),
        100,
        2500,
    );
    assert_eq!(long_response.content[0].text, long_text);
    assert_eq!(long_response.usage.output_tokens, 2500);

    // Test response with Unicode text
    let unicode_response = AnthropicResponse::new(
        "msg_unicode".to_string(),
        "claude-3-sonnet".to_string(),
        "Hello 世界! 🌍 Здравствуй мир!".to_string(),
        10,
        15,
    );
    assert!(unicode_response.content[0].text.contains("世界"));
    assert!(unicode_response.content[0].text.contains("🌍"));
    assert!(unicode_response.content[0].text.contains("мир"));
}

#[test]
fn test_content_validation_comprehensive() {
    // Test various content patterns that should be valid
    let valid_contents = vec![
        "Simple text",
        "Text with numbers 123",
        "Text with symbols !@#$%^&*()",
        "Text\nwith\nnewlines",
        "Text\twith\ttabs",
        "Text with 'quotes' and \"double quotes\"",
        "Text with [brackets] and {braces}",
        "Mixed case TEXT with CamelCase",
        "Text with émojis 😀 and ñ characters",
        "Very long text that goes on and on and on and should still be valid as long as it doesn't exceed the maximum length limit",
    ];

    for content in valid_contents {
        let message = Message::user(content.to_string());
        assert!(message.validate().is_ok(), "Content should be valid: {}", content);
    }

    // Test content that should be invalid
    let invalid_contents = vec![
        "", // Empty content
        "Content with null byte\0here",
    ];

    for content in invalid_contents {
        let message = Message::user(content.to_string());
        assert!(message.validate().is_err(), "Content should be invalid: {}", content);
    }
}