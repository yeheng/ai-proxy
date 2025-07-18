use ai_proxy::providers::anthropic::*;

#[test]
fn test_message_validation_valid() {
    let message = Message {
        role: "user".to_string(),
        content: "Hello, world!".to_string(),
    };
    assert!(message.validate().is_ok());
}

#[test]
fn test_message_validation_invalid_role() {
    let message = Message {
        role: "invalid".to_string(),
        content: "Hello".to_string(),
    };
    let result = message.validate();
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Invalid role"));
}

#[test]
fn test_message_validation_empty_content() {
    let message = Message {
        role: "user".to_string(),
        content: "".to_string(),
    };
    let result = message.validate();
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Message content cannot be empty"));
}

#[test]
fn test_message_validation_content_too_long() {
    let long_content = "a".repeat(1000001);
    let message = Message {
        role: "user".to_string(),
        content: long_content,
    };
    let result = message.validate();
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Message content too long"));
}

#[test]
fn test_message_validation_null_bytes() {
    let message = Message {
        role: "user".to_string(),
        content: "Hello\0world".to_string(),
    };
    let result = message.validate();
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Message content cannot contain null bytes"));
}

#[test]
fn test_request_validation_valid() {
    let request = AnthropicRequest {
        model: "claude-3-sonnet-20240229".to_string(),
        messages: vec![
            Message {
                role: "user".to_string(),
                content: "Hello".to_string(),
            }
        ],
        max_tokens: 1000,
        temperature: Some(0.7),
        top_p: Some(0.9),
        stream: Some(false),
    };
    assert!(request.validate().is_ok());
}

#[test]
fn test_request_validation_empty_model() {
    let request = AnthropicRequest {
        model: "".to_string(),
        messages: vec![
            Message {
                role: "user".to_string(),
                content: "Hello".to_string(),
            }
        ],
        max_tokens: 1000,
        temperature: None,
        top_p: None,
        stream: None,
    };
    let result = request.validate();
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Model name cannot be empty"));
}

#[test]
fn test_request_validation_model_too_long() {
    let long_model = "a".repeat(101);
    let request = AnthropicRequest {
        model: long_model,
        messages: vec![
            Message {
                role: "user".to_string(),
                content: "Hello".to_string(),
            }
        ],
        max_tokens: 1000,
        temperature: None,
        top_p: None,
        stream: None,
    };
    let result = request.validate();
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Model name too long"));
}

#[test]
fn test_request_validation_invalid_model_characters() {
    let request = AnthropicRequest {
        model: "invalid model!".to_string(),
        messages: vec![
            Message {
                role: "user".to_string(),
                content: "Hello".to_string(),
            }
        ],
        max_tokens: 1000,
        temperature: None,
        top_p: None,
        stream: None,
    };
    let result = request.validate();
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Model name contains invalid characters"));
}

#[test]
fn test_request_validation_empty_messages() {
    let request = AnthropicRequest {
        model: "claude-3-sonnet-20240229".to_string(),
        messages: vec![],
        max_tokens: 1000,
        temperature: None,
        top_p: None,
        stream: None,
    };
    let result = request.validate();
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Messages cannot be empty"));
}

#[test]
fn test_request_validation_too_many_messages() {
    let messages = vec![
        Message {
            role: "user".to_string(),
            content: "Hello".to_string(),
        };
        101
    ];
    let request = AnthropicRequest {
        model: "claude-3-sonnet-20240229".to_string(),
        messages,
        max_tokens: 1000,
        temperature: None,
        top_p: None,
        stream: None,
    };
    let result = request.validate();
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Too many messages"));
}

#[test]
fn test_request_validation_conversation_must_start_with_user() {
    let request = AnthropicRequest {
        model: "claude-3-sonnet-20240229".to_string(),
        messages: vec![
            Message {
                role: "assistant".to_string(),
                content: "Hello".to_string(),
            }
        ],
        max_tokens: 1000,
        temperature: None,
        top_p: None,
        stream: None,
    };
    let result = request.validate();

    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Invalid role sequence at message 0: expected 'user', got 'assistant'"));
}

#[test]
fn test_request_validation_invalid_role_sequence() {
    let request = AnthropicRequest {
        model: "claude-3-sonnet-20240229".to_string(),
        messages: vec![
            Message {
                role: "user".to_string(),
                content: "Hello".to_string(),
            },
            Message {
                role: "user".to_string(),
                content: "How are you?".to_string(),
            }
        ],
        max_tokens: 1000,
        temperature: None,
        top_p: None,
        stream: None,
    };
    let result = request.validate();
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Invalid role sequence"));
}

#[test]
fn test_request_validation_zero_max_tokens() {
    let request = AnthropicRequest {
        model: "claude-3-sonnet-20240229".to_string(),
        messages: vec![
            Message {
                role: "user".to_string(),
                content: "Hello".to_string(),
            }
        ],
        max_tokens: 0,
        temperature: None,
        top_p: None,
        stream: None,
    };
    let result = request.validate();
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("max_tokens must be greater than 0"));
}

#[test]
fn test_request_validation_max_tokens_too_high() {
    let request = AnthropicRequest {
        model: "claude-3-sonnet-20240229".to_string(),
        messages: vec![
            Message {
                role: "user".to_string(),
                content: "Hello".to_string(),
            }
        ],
        max_tokens: 1000001,
        temperature: None,
        top_p: None,
        stream: None,
    };
    let result = request.validate();
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("max_tokens cannot exceed 8192"));
}

#[test]
fn test_request_validation_invalid_temperature() {
    let request = AnthropicRequest {
        model: "claude-3-sonnet-20240229".to_string(),
        messages: vec![
            Message {
                role: "user".to_string(),
                content: "Hello".to_string(),
            }
        ],
        max_tokens: 1000,
        temperature: Some(2.1),
        top_p: None,
        stream: None,
    };
    let result = request.validate();
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("temperature must be between 0.0 and 2.0"));
}

#[test]
fn test_request_validation_invalid_top_p() {
    let request = AnthropicRequest {
        model: "claude-3-sonnet-20240229".to_string(),
        messages: vec![
            Message {
                role: "user".to_string(),
                content: "Hello".to_string(),
            }
        ],
        max_tokens: 1000,
        temperature: None,
        top_p: Some(1.1),
        stream: None,
    };
    let result = request.validate();
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("top_p must be between 0.0 and 1.0"));
}

#[test]
fn test_request_validation_content_too_long() {
    let long_content = "a".repeat(1000001);
    let request = AnthropicRequest {
        model: "claude-3-sonnet-20240229".to_string(),
        messages: vec![
            Message {
                role: "user".to_string(),
                content: long_content,
            }
        ],
        max_tokens: 1000,
        temperature: None,
        top_p: None,
        stream: None,
    };
    let result = request.validate();
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Message content too long"));
}

#[test]
fn test_request_is_streaming() {
    let request = AnthropicRequest {
        model: "claude-3-sonnet-20240229".to_string(),
        messages: vec![
            Message {
                role: "user".to_string(),
                content: "Hello".to_string(),
            }
        ],
        max_tokens: 1000,
        temperature: None,
        top_p: None,
        stream: Some(true),
    };
    assert!(request.is_streaming());
}

#[test]
fn test_request_estimate_input_tokens() {
    let request = AnthropicRequest {
        model: "claude-3-sonnet-20240229".to_string(),
        messages: vec![
            Message {
                role: "user".to_string(),
                content: "Hello, world!".to_string(),
            }
        ],
        max_tokens: 1000,
        temperature: None,
        top_p: None,
        stream: None,
    };
    let estimated = request.estimate_input_tokens();
    assert!(estimated > 0);
}

#[test]
fn test_anthropic_response_creation() {
    let response = AnthropicResponse {
        id: "test-id".to_string(),
        model: "claude-3-sonnet-20240229".to_string(),
        content: vec![ContentBlock {
            type_field: "text".to_string(),
            text: "Hello!".to_string(),
        }],
        usage: Usage {
            input_tokens: 10,
            output_tokens: 5,
        },
    };
    
    assert_eq!(response.id, "test-id");
    assert_eq!(response.model, "claude-3-sonnet-20240229");
    assert_eq!(response.content.len(), 1);
    assert_eq!(response.usage.input_tokens, 10);
    assert_eq!(response.usage.output_tokens, 5);
}

#[test]
fn test_message_constructors() {
    let user_message = Message::user("Hello".to_string());
    assert_eq!(user_message.role, "user");
    assert_eq!(user_message.content, "Hello");

    let assistant_message = Message::assistant("Hi there!".to_string());
    assert_eq!(assistant_message.role, "assistant");
    assert_eq!(assistant_message.content, "Hi there!");
}

#[test]
fn test_valid_conversation_flow() {
    let messages = vec![
        Message::user("Hello".to_string()),
        Message::assistant("Hi! How can I help you?".to_string()),
        Message::user("What's the weather like?".to_string()),
    ];
    
    let request = AnthropicRequest {
        model: "claude-3-sonnet-20240229".to_string(),
        messages,
        max_tokens: 1000,
        temperature: None,
        top_p: None,
        stream: None,
    };
    
    assert!(request.validate().is_ok());
}
