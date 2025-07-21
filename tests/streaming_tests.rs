use ai_proxy::providers::{
    anthropic::{SSEEvent, AnthropicStreamEvent, StreamMessage, ContentBlockStart, TextDelta, MessageDelta, StreamError, Usage},
    openai::{OpenAIStreamResponse, OpenAIStreamChoice, OpenAIStreamDelta},
    gemini::{GeminiStreamResponse, GeminiStreamCandidate, GeminiContent, GeminiPart, UsageMetadata},
};

/// Test streaming functionality and Server-Sent Events processing
/// These tests focus on streaming response handling and SSE formatting

#[test]
fn test_sse_event_basic_formatting() {
    let event = SSEEvent::new("Hello World".to_string());
    let formatted = event.to_sse_string();
    assert_eq!(formatted, "data: Hello World\n\n");
}

#[test]
fn test_sse_event_with_event_type() {
    let event = SSEEvent::with_event("message".to_string(), "Hello World".to_string());
    let formatted = event.to_sse_string();
    assert_eq!(formatted, "event: message\ndata: Hello World\n\n");
}

#[test]
fn test_sse_event_multiline_data() {
    let multiline_data = "Line 1\nLine 2\nLine 3".to_string();
    let event = SSEEvent::new(multiline_data);
    let formatted = event.to_sse_string();
    assert_eq!(formatted, "data: Line 1\ndata: Line 2\ndata: Line 3\n\n");
}

#[test]
fn test_sse_event_with_id() {
    let mut event = SSEEvent::new("Test data".to_string());
    event.id = Some("event-123".to_string());
    let formatted = event.to_sse_string();
    assert_eq!(formatted, "id: event-123\ndata: Test data\n\n");
}

#[test]
fn test_sse_event_complete() {
    let mut event = SSEEvent::with_event("update".to_string(), "Status update".to_string());
    event.id = Some("update-456".to_string());
    let formatted = event.to_sse_string();
    assert_eq!(formatted, "event: update\nid: update-456\ndata: Status update\n\n");
}

#[test]
fn test_anthropic_stream_event_message_start() {
    let message_start = AnthropicStreamEvent::MessageStart {
        message: StreamMessage {
            id: "msg_123".to_string(),
            model: "claude-3-sonnet".to_string(),
            role: "assistant".to_string(),
            content: vec![],
            usage: Usage {
                input_tokens: 15,
                output_tokens: 0,
            },
        },
    };

    let json = serde_json::to_string(&message_start).unwrap();
    assert!(json.contains("\"type\":\"message_start\""));
    assert!(json.contains("\"id\":\"msg_123\""));
    assert!(json.contains("\"model\":\"claude-3-sonnet\""));
    assert!(json.contains("\"input_tokens\":15"));
}

#[test]
fn test_anthropic_stream_event_content_block_start() {
    let content_start = AnthropicStreamEvent::ContentBlockStart {
        index: 0,
        content_block: ContentBlockStart {
            type_field: "text".to_string(),
            text: "".to_string(),
        },
    };

    let json = serde_json::to_string(&content_start).unwrap();
    assert!(json.contains("\"type\":\"content_block_start\""));
    assert!(json.contains("\"index\":0"));
}

#[test]
fn test_anthropic_stream_event_content_block_delta() {
    let content_delta = AnthropicStreamEvent::ContentBlockDelta {
        index: 0,
        delta: TextDelta {
            type_field: "text_delta".to_string(),
            text: "Hello".to_string(),
        },
    };

    let json = serde_json::to_string(&content_delta).unwrap();
    assert!(json.contains("\"type\":\"content_block_delta\""));
    assert!(json.contains("\"text\":\"Hello\""));
}

#[test]
fn test_anthropic_stream_event_content_block_stop() {
    let content_stop = AnthropicStreamEvent::ContentBlockStop { index: 0 };

    let json = serde_json::to_string(&content_stop).unwrap();
    assert!(json.contains("\"type\":\"content_block_stop\""));
    assert!(json.contains("\"index\":0"));
}

#[test]
fn test_anthropic_stream_event_message_delta() {
    let message_delta = AnthropicStreamEvent::MessageDelta {
        delta: MessageDelta {
            stop_reason: Some("end_turn".to_string()),
            usage: Some(Usage {
                input_tokens: 15,
                output_tokens: 25,
            }),
        },
    };

    let json = serde_json::to_string(&message_delta).unwrap();
    assert!(json.contains("\"type\":\"message_delta\""));
    assert!(json.contains("\"stop_reason\":\"end_turn\""));
    assert!(json.contains("\"output_tokens\":25"));
}

#[test]
fn test_anthropic_stream_event_message_stop() {
    let message_stop = AnthropicStreamEvent::MessageStop;

    let json = serde_json::to_string(&message_stop).unwrap();
    assert!(json.contains("\"type\":\"message_stop\""));
}

#[test]
fn test_anthropic_stream_event_error() {
    let error_event = AnthropicStreamEvent::Error {
        error: StreamError {
            error_type: "rate_limit_error".to_string(),
            message: "Rate limit exceeded".to_string(),
        },
    };

    let json = serde_json::to_string(&error_event).unwrap();
    assert!(json.contains("\"type\":\"error\""));
    assert!(json.contains("\"type\":\"error\""));
    assert!(json.contains("\"message\":\"Rate limit exceeded\""));
    assert!(json.contains("\"message\":\"Rate limit exceeded\""));
}

#[test]
fn test_openai_stream_response_conversion() {
    let openai_stream = OpenAIStreamResponse {
        id: "chatcmpl-stream-123".to_string(),
        object: "chat.completion.chunk".to_string(),
        created: 1234567890,
        model: "gpt-4".to_string(),
        choices: vec![OpenAIStreamChoice {
            index: 0,
            delta: OpenAIStreamDelta {
                role: Some("assistant".to_string()),
                content: Some("Hello".to_string()),
            },
            finish_reason: None,
            logprobs: None,
        }],
        system_fingerprint: None,
    };

    // Test conversion to Anthropic stream events
    let events = openai_stream.to_anthropic_events("msg_123").unwrap();
    assert!(!events.is_empty());

    // Verify the events contain expected content
    let events_json: Vec<String> = events.iter()
        .map(|e| serde_json::to_string(e).unwrap())
        .collect();
    
    let combined_json = events_json.join("");
    assert!(combined_json.contains("Hello"));
}

#[test]
fn test_openai_stream_response_with_finish_reason() {
    let openai_stream = OpenAIStreamResponse {
        id: "chatcmpl-stream-456".to_string(),
        object: "chat.completion.chunk".to_string(),
        created: 1234567890,
        model: "gpt-4".to_string(),
        choices: vec![OpenAIStreamChoice {
            index: 0,
            delta: OpenAIStreamDelta {
                role: None,
                content: None,
            },
            finish_reason: Some("stop".to_string()),
            logprobs: None,
        }],
        system_fingerprint: None,
    };

    let events = openai_stream.to_anthropic_events("msg_456").unwrap();
    assert!(!events.is_empty());

    // Should include message_stop event
    let events_json: Vec<String> = events.iter()
        .map(|e| serde_json::to_string(e).unwrap())
        .collect();
    
    let combined_json = events_json.join("");
    assert!(combined_json.contains("message_stop") || combined_json.contains("message_delta"));
}

#[test]
fn test_gemini_stream_response_conversion() {
    let gemini_stream = GeminiStreamResponse {
        candidates: Some(vec![GeminiStreamCandidate {
            content: Some(GeminiContent {
                role: "model".to_string(),
                parts: vec![GeminiPart {
                    text: "Hello from Gemini".to_string(),
                }],
            }),
            finish_reason: None,
            index: Some(0),
        }]),
        usage_metadata: None,
    };

    let events = gemini_stream.to_anthropic_events("gemini-pro", "msg_789").unwrap();
    assert!(!events.is_empty());

    let events_json: Vec<String> = events.iter()
        .map(|e| serde_json::to_string(e).unwrap())
        .collect();
    
    let combined_json = events_json.join("");
    assert!(combined_json.contains("Hello from Gemini"));
}

#[test]
fn test_gemini_stream_response_with_finish() {
    let gemini_stream = GeminiStreamResponse {
        candidates: Some(vec![GeminiStreamCandidate {
            content: Some(GeminiContent {
                role: "model".to_string(),
                parts: vec![GeminiPart {
                    text: "Final message".to_string(),
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

    let events = gemini_stream.to_anthropic_events("gemini-pro", "msg_final").unwrap();
    assert!(!events.is_empty());

    let events_json: Vec<String> = events.iter()
        .map(|e| serde_json::to_string(e).unwrap())
        .collect();
    
    let combined_json = events_json.join("");
    assert!(combined_json.contains("Final message"));
    assert!(combined_json.contains("message_stop") || combined_json.contains("message_delta"));
}

#[test]
fn test_stream_event_sequence() {
    // Test a complete streaming sequence
    let events = vec![
        AnthropicStreamEvent::MessageStart {
            message: StreamMessage {
                id: "msg_seq".to_string(),
                model: "claude-3-sonnet".to_string(),
                role: "assistant".to_string(),
                content: vec![],
                usage: Usage {
                    input_tokens: 10,
                    output_tokens: 0,
                },
            },
        },
        AnthropicStreamEvent::ContentBlockStart {
            index: 0,
            content_block: ContentBlockStart {
                type_field: "text".to_string(),
                text: "".to_string(),
            },
        },
        AnthropicStreamEvent::ContentBlockDelta {
            index: 0,
            delta: TextDelta {
                type_field: "text_delta".to_string(),
                text: "Hello".to_string(),
            },
        },
        AnthropicStreamEvent::ContentBlockDelta {
            index: 0,
            delta: TextDelta {
                type_field: "text_delta".to_string(),
                text: " World".to_string(),
            },
        },
        AnthropicStreamEvent::ContentBlockStop { index: 0 },
        AnthropicStreamEvent::MessageDelta {
            delta: MessageDelta {
                stop_reason: Some("end_turn".to_string()),
                usage: Some(Usage {
                    input_tokens: 10,
                    output_tokens: 5,
                }),
            },
        },
        AnthropicStreamEvent::MessageStop,
    ];

    // Convert each event to SSE format
    let sse_events: Vec<String> = events.iter()
        .map(|event| {
            let json = serde_json::to_string(event).unwrap();
            SSEEvent::new(json).to_sse_string()
        })
        .collect();

    // Verify the sequence
    assert_eq!(sse_events.len(), 7);
    assert!(sse_events[0].contains("message_start"));
    assert!(sse_events[1].contains("content_block_start"));
    assert!(sse_events[2].contains("Hello"));
    assert!(sse_events[3].contains(" World"));
    assert!(sse_events[4].contains("content_block_stop"));
    assert!(sse_events[5].contains("message_delta"));
    assert!(sse_events[6].contains("message_stop"));
}

#[test]
fn test_sse_event_special_characters() {
    // Test SSE formatting with various special characters
    let special_chars = vec![
        ("newline", "Line 1\nLine 2"),
        ("carriage_return", "Line 1\rLine 2"),
        ("tab", "Column 1\tColumn 2"),
        ("colon", "key: value"),
        ("quotes", "He said \"Hello\""),
        ("unicode", "Hello 世界 🌍"),
        ("json", r#"{"key": "value"}"#),
    ];

    for (name, content) in special_chars {
        let event = SSEEvent::new(content.to_string());
        let formatted = event.to_sse_string();
        
        // Should end with double newline
        assert!(formatted.ends_with("\n\n"), "Failed for {}: {}", name, formatted);
        
        // Should contain the content
        let expected_lines: Vec<&str> = content.lines().collect();
        let data_lines: Vec<&str> = formatted.lines().filter(|l| l.starts_with("data: ")).collect();
        assert_eq!(expected_lines.len(), data_lines.len(), "Failed for {}: {}", name, formatted);
        for (expected, data_line) in expected_lines.iter().zip(data_lines) {
            assert!(data_line.ends_with(expected), "Failed for {}: Expected '{}' in '{}'", name, expected, data_line);
        }
    }
}

#[test]
fn test_stream_error_handling() {
    // Test various error scenarios in streaming
    let error_scenarios = vec![
        ("rate_limit", "Rate limit exceeded"),
        ("authentication", "Invalid API key"),
        ("server_error", "Internal server error"),
        ("timeout", "Request timeout"),
        ("invalid_request", "Invalid request format"),
    ];

    for (error_type, error_message) in error_scenarios {
        let error_event = AnthropicStreamEvent::Error {
            error: StreamError {
                error_type: error_type.to_string(),
                message: error_message.to_string(),
            },
        };

        let json = serde_json::to_string(&error_event).unwrap();
        assert!(json.contains(error_type));
        assert!(json.contains(error_message));

        let sse_event = SSEEvent::new(json);
        let formatted = sse_event.to_sse_string();
        assert!(formatted.contains(error_type));
        assert!(formatted.contains(error_message));
    }
}

#[test]
fn test_stream_response_empty_content() {
    // Test handling of empty content in streaming responses
    let empty_delta = AnthropicStreamEvent::ContentBlockDelta {
        index: 0,
        delta: TextDelta {
            type_field: "text_delta".to_string(),
            text: "".to_string(),
        },
    };

    let json = serde_json::to_string(&empty_delta).unwrap();
    assert!(json.contains("\"text\":\"\""));

    let sse_event = SSEEvent::new(json);
    let formatted = sse_event.to_sse_string();
    assert!(formatted.contains("data: "));
    assert!(formatted.ends_with("\n\n"));
}

#[test]
fn test_stream_response_large_content() {
    // Test handling of large content chunks in streaming
    let large_text = "A".repeat(1000);
    let large_delta = AnthropicStreamEvent::ContentBlockDelta {
        index: 0,
        delta: TextDelta {
            type_field: "text_delta".to_string(),
            text: large_text.clone(),
        },
    };

    let json = serde_json::to_string(&large_delta).unwrap();
    assert!(json.contains(&large_text));

    let sse_event = SSEEvent::new(json);
    let formatted = sse_event.to_sse_string();
    assert!(formatted.len() > 1000);
    assert!(formatted.ends_with("\n\n"));
}

#[test]
fn test_usage_metadata_in_streams() {
    // Test usage metadata handling in streaming responses
    let message_delta_with_usage = AnthropicStreamEvent::MessageDelta {
        delta: MessageDelta {
            stop_reason: Some("end_turn".to_string()),
            usage: Some(Usage {
                input_tokens: 25,
                output_tokens: 50,
            }),
        },
    };

    let json = serde_json::to_string(&message_delta_with_usage).unwrap();
    assert!(json.contains("\"input_tokens\":25"));
    assert!(json.contains("\"output_tokens\":50"));

    // Test message delta without usage
    let message_delta_no_usage = AnthropicStreamEvent::MessageDelta {
        delta: MessageDelta {
            stop_reason: Some("end_turn".to_string()),
            usage: None,
        },
    };

    let json = serde_json::to_string(&message_delta_no_usage).unwrap();
    assert!(json.contains("\"stop_reason\":\"end_turn\""));
    assert!(!json.contains("usage"));
}