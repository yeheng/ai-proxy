use serde::{Deserialize, Serialize};

/// Standard request format based on Anthropic API
/// 
/// This serves as the unified request format that all providers
/// must accept and convert to their specific API format.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AnthropicRequest {
    pub model: String,
    pub messages: Vec<Message>,
    pub max_tokens: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,
}

/// Message structure for chat conversations
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Message {
    pub role: String, // "user" or "assistant"
    pub content: String,
}

impl Message {
    /// Validate individual message
    pub fn validate(&self) -> Result<(), String> {
        // Role validation
        if self.role != "user" && self.role != "assistant" {
            return Err(format!("Invalid role '{}': must be 'user' or 'assistant'", self.role));
        }
        
        // Content validation
        if self.content.is_empty() {
            return Err("Message content cannot be empty".to_string());
        }
        
        if self.content.len() > 100_000 {
            return Err("Message content too long (max 100KB)".to_string());
        }
        
        // Check for null bytes or other problematic characters
        if self.content.contains('\0') {
            return Err("Message content cannot contain null bytes".to_string());
        }
        
        Ok(())
    }
    
    /// Create a new user message
    pub fn user(content: String) -> Self {
        Self {
            role: "user".to_string(),
            content,
        }
    }
    
    /// Create a new assistant message
    pub fn assistant(content: String) -> Self {
        Self {
            role: "assistant".to_string(),
            content,
        }
    }
}

/// Standard response format based on Anthropic API
/// 
/// All providers must convert their responses to this format
/// to ensure consistent client experience.
#[derive(Serialize, Deserialize, Debug)]
pub struct AnthropicResponse {
    pub id: String,
    pub model: String,
    pub content: Vec<ContentBlock>,
    pub usage: Usage,
}

/// Content block within a response
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ContentBlock {
    #[serde(rename = "type")]
    pub type_field: String, // "text"
    pub text: String,
}

/// Token usage information
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Usage {
    pub input_tokens: u32,
    pub output_tokens: u32,
}

// Streaming event structures for Server-Sent Events

/// Streaming events that match Anthropic's streaming format
#[derive(Serialize, Debug, Clone)]
#[serde(tag = "type")]
pub enum AnthropicStreamEvent {
    #[serde(rename = "message_start")]
    MessageStart { message: StreamMessage },
    #[serde(rename = "content_block_start")]
    ContentBlockStart { index: u32, content_block: ContentBlockStart },
    #[serde(rename = "content_block_delta")]
    ContentBlockDelta { index: u32, delta: TextDelta },
    #[serde(rename = "content_block_stop")]
    ContentBlockStop { index: u32 },
    #[serde(rename = "message_delta")]
    MessageDelta { delta: MessageDelta },
    #[serde(rename = "message_stop")]
    MessageStop,
    #[serde(rename = "error")]
    Error { error: StreamError },
}

/// Simplified message structure for streaming start events
#[derive(Serialize, Debug, Clone)]
pub struct StreamMessage {
    pub id: String,
    pub model: String,
    pub role: String,
    pub content: Vec<ContentBlock>,
    pub usage: Usage,
}

/// Content block start event for streaming
#[derive(Serialize, Debug, Clone)]
pub struct ContentBlockStart {
    #[serde(rename = "type")]
    pub type_field: String, // "text"
    pub text: String,
}

/// Text delta for streaming content updates
#[derive(Serialize, Debug, Clone)]
pub struct TextDelta {
    #[serde(rename = "type")]
    pub type_field: String, // "text_delta"
    pub text: String,
}

/// Message delta for streaming updates
#[derive(Serialize, Debug, Clone)]
pub struct MessageDelta {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<Usage>,
}

/// Error event for streaming
#[derive(Serialize, Debug, Clone)]
pub struct StreamError {
    #[serde(rename = "type")]
    pub error_type: String,
    pub message: String,
}

/// Server-Sent Event wrapper for streaming responses
#[derive(Debug, Clone)]
pub struct SSEEvent {
    pub event: Option<String>,
    pub data: String,
    pub id: Option<String>,
}

impl SSEEvent {
    /// Create a new SSE event with data
    pub fn new(data: String) -> Self {
        Self {
            event: None,
            data,
            id: None,
        }
    }

    /// Create a new SSE event with event type and data
    pub fn with_event(event: String, data: String) -> Self {
        Self {
            event: Some(event),
            data,
            id: None,
        }
    }

    /// Format as SSE string
    pub fn to_sse_string(&self) -> String {
        let mut result = String::new();
        
        if let Some(event) = &self.event {
            result.push_str(&format!("event: {}\n", event));
        }
        
        if let Some(id) = &self.id {
            result.push_str(&format!("id: {}\n", id));
        }
        
        // Handle multi-line data
        for line in self.data.lines() {
            result.push_str(&format!("data: {}\n", line));
        }
        
        result.push('\n');
        result
    }
}

impl AnthropicRequest {
    /// Validate the request parameters with comprehensive checks
    pub fn validate(&self) -> Result<(), String> {
        // Model validation
        self.validate_model()?;
        
        // Messages validation
        self.validate_messages()?;
        
        // Token limits validation
        self.validate_token_limits()?;
        
        // Parameter ranges validation
        self.validate_parameters()?;
        
        // Content length validation
        self.validate_content_length()?;
        
        Ok(())
    }
    
    /// Validate model name
    fn validate_model(&self) -> Result<(), String> {
        if self.model.is_empty() {
            return Err("Model name cannot be empty".to_string());
        }
        
        if self.model.len() > 100 {
            return Err("Model name too long (max 100 characters)".to_string());
        }
        
        // Check for valid model name format (alphanumeric, hyphens, underscores, dots)
        if !self.model.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_' || c == '.') {
            return Err("Model name contains invalid characters".to_string());
        }
        
        Ok(())
    }
    
    /// Validate messages array
    fn validate_messages(&self) -> Result<(), String> {
        if self.messages.is_empty() {
            return Err("Messages cannot be empty".to_string());
        }
        
        if self.messages.len() > 100 {
            return Err("Too many messages (max 100)".to_string());
        }
        
        // Validate conversation flow (should start with user, alternate properly)
        if self.messages[0].role != "user" {
            return Err("Conversation must start with a user message".to_string());
        }
        
        // Check for proper role alternation and validate each message
        let mut expected_role = "user";
        for (i, message) in self.messages.iter().enumerate() {
            message.validate()?;
            
            if message.role != expected_role && !(i == 0 && message.role == "user") {
                return Err(format!(
                    "Invalid role sequence at message {}: expected '{}', got '{}'",
                    i, expected_role, message.role
                ));
            }
            
            expected_role = if message.role == "user" { "assistant" } else { "user" };
        }
        
        Ok(())
    }
    
    /// Validate token limits
    fn validate_token_limits(&self) -> Result<(), String> {
        if self.max_tokens == 0 {
            return Err("max_tokens must be greater than 0".to_string());
        }
        
        if self.max_tokens > 8192 {
            return Err("max_tokens cannot exceed 8192".to_string());
        }
        
        Ok(())
    }
    
    /// Validate parameter ranges
    fn validate_parameters(&self) -> Result<(), String> {
        if let Some(temp) = self.temperature {
            if temp < 0.0 || temp > 2.0 {
                return Err("temperature must be between 0.0 and 2.0".to_string());
            }
            if temp.is_nan() || temp.is_infinite() {
                return Err("temperature must be a valid number".to_string());
            }
        }
        
        if let Some(top_p) = self.top_p {
            if top_p < 0.0 || top_p > 1.0 {
                return Err("top_p must be between 0.0 and 1.0".to_string());
            }
            if top_p.is_nan() || top_p.is_infinite() {
                return Err("top_p must be a valid number".to_string());
            }
        }
        
        Ok(())
    }
    
    /// Validate total content length
    fn validate_content_length(&self) -> Result<(), String> {
        let total_content_length: usize = self.messages.iter()
            .map(|m| m.content.len())
            .sum();
            
        if total_content_length > 200_000 {
            return Err("Total content length exceeds maximum (200KB)".to_string());
        }
        
        Ok(())
    }
    
    /// Check if request is for streaming
    pub fn is_streaming(&self) -> bool {
        self.stream.unwrap_or(false)
    }
    
    /// Get estimated token count (rough approximation)
    pub fn estimate_input_tokens(&self) -> u32 {
        // Rough estimation: 1 token ≈ 4 characters
        let total_chars: usize = self.messages.iter()
            .map(|m| m.content.len() + m.role.len())
            .sum();
        (total_chars / 4).max(1) as u32
    }
}

impl AnthropicResponse {
    /// Create a new response with the given parameters
    pub fn new(id: String, model: String, text: String, input_tokens: u32, output_tokens: u32) -> Self {
        Self {
            id,
            model,
            content: vec![ContentBlock {
                type_field: "text".to_string(),
                text,
            }],
            usage: Usage {
                input_tokens,
                output_tokens,
            },
        }
    }
}

// Anthropic Provider Implementation
use async_trait::async_trait;
use reqwest::Client;

use crate::{
    config::ProviderDetail,
    errors::AppError,
    providers::{AIProvider, StreamResponse, ModelInfo, HealthStatus},
};

/// Anthropic provider implementation (native format)
/// 
/// This provider handles Anthropic's native API format directly,
/// requiring minimal conversion since our standard format is based on Anthropic.
pub struct AnthropicProvider {
    config: ProviderDetail,
    client: Client,
}

impl AnthropicProvider {
    pub fn new(config: ProviderDetail, client: Client) -> Self {
        Self { config, client }
    }
}

#[async_trait]
impl AIProvider for AnthropicProvider {
    async fn chat(&self, request: AnthropicRequest) -> Result<AnthropicResponse, AppError> {
        // Validate request
        request.validate().map_err(AppError::ValidationError)?;

        // Build URL
        let url = format!("{}messages", self.config.api_base);

        // Send request (minimal conversion needed since we use Anthropic format)
        let response = self.client
            .post(&url)
            .header("x-api-key", &self.config.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|e| AppError::ProviderError {
                status: 500,
                message: format!("Failed to send request to Anthropic: {}", e),
            })?;

        // Handle HTTP errors
        if !response.status().is_success() {
            let status = response.status().as_u16();
            let error_body = response.text().await.unwrap_or_default();
            return Err(AppError::ProviderError {
                status,
                message: format!("Anthropic API error: {}", error_body),
            });
        }

        // Parse response (direct format match)
        let anthropic_res = response.json::<AnthropicResponse>().await
            .map_err(|e| AppError::ProviderError {
                status: 500,
                message: format!("Failed to parse Anthropic response: {}", e),
            })?;

        Ok(anthropic_res)
    }

    async fn chat_stream(&self, _request: AnthropicRequest) -> Result<StreamResponse, AppError> {
        // TODO: Implement streaming support
        Err(AppError::InternalServerError(
            "Streaming not yet implemented for Anthropic provider".to_string()
        ))
    }

    async fn list_models(&self) -> Result<Vec<ModelInfo>, AppError> {
        let models = self.config.models.as_ref()
            .map(|m| m.clone())
            .unwrap_or_else(|| vec![
                "claude-3-opus-20240229".to_string(),
                "claude-3-sonnet-20240229".to_string(),
                "claude-3-haiku-20240307".to_string(),
            ]);

        Ok(models.into_iter().map(|model| ModelInfo {
            id: model,
            object: "model".to_string(),
            created: 1714560000, // Static timestamp for now
            owned_by: "anthropic".to_string(),
        }).collect())
    }

    async fn health_check(&self) -> Result<HealthStatus, AppError> {
        let start = std::time::Instant::now();
        
        // Simple health check by making a minimal request
        let url = format!("{}messages", self.config.api_base);
        
        // Create a minimal test request
        let test_request = AnthropicRequest {
            model: "claude-3-haiku-20240307".to_string(),
            messages: vec![Message {
                role: "user".to_string(),
                content: "Hi".to_string(),
            }],
            max_tokens: 1,
            stream: None,
            temperature: None,
            top_p: None,
        };

        let result = self.client
            .post(&url)
            .header("x-api-key", &self.config.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("Content-Type", "application/json")
            .json(&test_request)
            .send()
            .await;

        let latency = start.elapsed().as_millis() as u64;

        match result {
            Ok(response) if response.status().is_success() => {
                Ok(HealthStatus {
                    status: "healthy".to_string(),
                    provider: "anthropic".to_string(),
                    latency_ms: Some(latency),
                    error: None,
                })
            }
            Ok(response) => {
                Ok(HealthStatus {
                    status: "unhealthy".to_string(),
                    provider: "anthropic".to_string(),
                    latency_ms: Some(latency),
                    error: Some(format!("HTTP {}", response.status())),
                })
            }
            Err(e) => {
                Ok(HealthStatus {
                    status: "unhealthy".to_string(),
                    provider: "anthropic".to_string(),
                    latency_ms: Some(latency),
                    error: Some(e.to_string()),
                })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Helper function to create a valid request for testing
    fn create_valid_request() -> AnthropicRequest {
        AnthropicRequest {
            model: "claude-3-haiku-20240307".to_string(),
            messages: vec![Message::user("Hello, world!".to_string())],
            max_tokens: 100,
            stream: None,
            temperature: Some(0.7),
            top_p: Some(0.9),
        }
    }

    #[test]
    fn test_message_validation_valid() {
        let message = Message::user("Hello, world!".to_string());
        assert!(message.validate().is_ok());

        let message = Message::assistant("Hi there!".to_string());
        assert!(message.validate().is_ok());
    }

    #[test]
    fn test_message_validation_invalid_role() {
        let message = Message {
            role: "system".to_string(),
            content: "Hello".to_string(),
        };
        let result = message.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Invalid role 'system'"));
    }

    #[test]
    fn test_message_validation_empty_content() {
        let message = Message::user("".to_string());
        let result = message.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Message content cannot be empty"));
    }

    #[test]
    fn test_message_validation_content_too_long() {
        let long_content = "a".repeat(100_001);
        let message = Message::user(long_content);
        let result = message.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Message content too long"));
    }

    #[test]
    fn test_message_validation_null_bytes() {
        let message = Message::user("Hello\0world".to_string());
        let result = message.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("cannot contain null bytes"));
    }

    #[test]
    fn test_request_validation_valid() {
        let request = create_valid_request();
        assert!(request.validate().is_ok());
    }

    #[test]
    fn test_request_validation_empty_model() {
        let mut request = create_valid_request();
        request.model = "".to_string();
        let result = request.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Model name cannot be empty"));
    }

    #[test]
    fn test_request_validation_model_too_long() {
        let mut request = create_valid_request();
        request.model = "a".repeat(101);
        let result = request.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Model name too long"));
    }

    #[test]
    fn test_request_validation_invalid_model_characters() {
        let mut request = create_valid_request();
        request.model = "model@name!".to_string();
        let result = request.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("invalid characters"));
    }

    #[test]
    fn test_request_validation_empty_messages() {
        let mut request = create_valid_request();
        request.messages = vec![];
        let result = request.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Messages cannot be empty"));
    }

    #[test]
    fn test_request_validation_too_many_messages() {
        let mut request = create_valid_request();
        request.messages = (0..101).map(|i| {
            if i % 2 == 0 {
                Message::user(format!("Message {}", i))
            } else {
                Message::assistant(format!("Response {}", i))
            }
        }).collect();
        let result = request.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Too many messages"));
    }

    #[test]
    fn test_request_validation_conversation_must_start_with_user() {
        let mut request = create_valid_request();
        request.messages = vec![Message::assistant("Hello!".to_string())];
        let result = request.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Conversation must start with a user message"));
    }

    #[test]
    fn test_request_validation_invalid_role_sequence() {
        let mut request = create_valid_request();
        request.messages = vec![
            Message::user("Hello".to_string()),
            Message::user("Another user message".to_string()),
        ];
        let result = request.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Invalid role sequence"));
    }

    #[test]
    fn test_request_validation_zero_max_tokens() {
        let mut request = create_valid_request();
        request.max_tokens = 0;
        let result = request.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("max_tokens must be greater than 0"));
    }

    #[test]
    fn test_request_validation_max_tokens_too_high() {
        let mut request = create_valid_request();
        request.max_tokens = 8193;
        let result = request.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("max_tokens cannot exceed 8192"));
    }

    #[test]
    fn test_request_validation_invalid_temperature() {
        let mut request = create_valid_request();
        request.temperature = Some(-0.1);
        let result = request.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("temperature must be between 0.0 and 2.0"));

        request.temperature = Some(2.1);
        let result = request.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("temperature must be between 0.0 and 2.0"));

        request.temperature = Some(f32::NAN);
        let result = request.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("temperature must be a valid number"));
    }

    #[test]
    fn test_request_validation_invalid_top_p() {
        let mut request = create_valid_request();
        request.top_p = Some(-0.1);
        let result = request.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("top_p must be between 0.0 and 1.0"));

        request.top_p = Some(1.1);
        let result = request.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("top_p must be between 0.0 and 1.0"));

        request.top_p = Some(f32::INFINITY);
        let result = request.validate();
        assert!(result.is_err());
        let error_msg = result.unwrap_err();
        println!("Actual error message: {}", error_msg);
        assert!(error_msg.contains("top_p must be a valid number"));
    }

    #[test]
    fn test_request_validation_content_too_long() {
        let mut request = create_valid_request();
        let long_content = "a".repeat(200_001);
        request.messages = vec![Message::user(long_content)];
        let result = request.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Total content length exceeds maximum"));
    }

    #[test]
    fn test_request_is_streaming() {
        let mut request = create_valid_request();
        assert!(!request.is_streaming());

        request.stream = Some(false);
        assert!(!request.is_streaming());

        request.stream = Some(true);
        assert!(request.is_streaming());
    }

    #[test]
    fn test_request_estimate_input_tokens() {
        let request = create_valid_request();
        let tokens = request.estimate_input_tokens();
        assert!(tokens > 0);
        assert!(tokens < 100); // Should be reasonable for "Hello, world!" + role
    }

    #[test]
    fn test_anthropic_response_creation() {
        let response = AnthropicResponse::new(
            "msg_123".to_string(),
            "claude-3-haiku".to_string(),
            "Hello there!".to_string(),
            10,
            5,
        );

        assert_eq!(response.id, "msg_123");
        assert_eq!(response.model, "claude-3-haiku");
        assert_eq!(response.content.len(), 1);
        assert_eq!(response.content[0].text, "Hello there!");
        assert_eq!(response.usage.input_tokens, 10);
        assert_eq!(response.usage.output_tokens, 5);
    }

    #[test]
    fn test_sse_event_creation() {
        let event = SSEEvent::new("test data".to_string());
        assert_eq!(event.data, "test data");
        assert!(event.event.is_none());
        assert!(event.id.is_none());

        let event = SSEEvent::with_event("message".to_string(), "test data".to_string());
        assert_eq!(event.data, "test data");
        assert_eq!(event.event, Some("message".to_string()));
    }

    #[test]
    fn test_sse_event_formatting() {
        let event = SSEEvent::new("simple data".to_string());
        let formatted = event.to_sse_string();
        assert_eq!(formatted, "data: simple data\n\n");

        let event = SSEEvent::with_event("test".to_string(), "event data".to_string());
        let formatted = event.to_sse_string();
        assert_eq!(formatted, "event: test\ndata: event data\n\n");

        // Test multi-line data
        let event = SSEEvent::new("line1\nline2".to_string());
        let formatted = event.to_sse_string();
        assert_eq!(formatted, "data: line1\ndata: line2\n\n");
    }

    #[test]
    fn test_streaming_event_serialization() {
        let stream_msg = StreamMessage {
            id: "msg_123".to_string(),
            model: "claude-3-haiku".to_string(),
            role: "assistant".to_string(),
            content: vec![ContentBlock {
                type_field: "text".to_string(),
                text: "".to_string(),
            }],
            usage: Usage {
                input_tokens: 10,
                output_tokens: 0,
            },
        };

        let event = AnthropicStreamEvent::MessageStart { message: stream_msg };
        let serialized = serde_json::to_string(&event);
        assert!(serialized.is_ok());
        assert!(serialized.unwrap().contains("message_start"));

        let delta_event = AnthropicStreamEvent::ContentBlockDelta {
            index: 0,
            delta: TextDelta {
                type_field: "text_delta".to_string(),
                text: "Hello".to_string(),
            },
        };
        let serialized = serde_json::to_string(&delta_event);
        assert!(serialized.is_ok());
        assert!(serialized.unwrap().contains("content_block_delta"));
    }

    #[test]
    fn test_message_constructors() {
        let user_msg = Message::user("User message".to_string());
        assert_eq!(user_msg.role, "user");
        assert_eq!(user_msg.content, "User message");

        let assistant_msg = Message::assistant("Assistant response".to_string());
        assert_eq!(assistant_msg.role, "assistant");
        assert_eq!(assistant_msg.content, "Assistant response");
    }

    #[test]
    fn test_valid_conversation_flow() {
        let request = AnthropicRequest {
            model: "claude-3-haiku".to_string(),
            messages: vec![
                Message::user("Hello".to_string()),
                Message::assistant("Hi there!".to_string()),
                Message::user("How are you?".to_string()),
            ],
            max_tokens: 100,
            stream: None,
            temperature: None,
            top_p: None,
        };

        assert!(request.validate().is_ok());
    }
}