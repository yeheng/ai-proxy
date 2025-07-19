use serde::{Deserialize, Serialize};
use crate::errors::AppError;
use crate::providers::anthropic::{AnthropicRequest, AnthropicResponse, AnthropicStreamEvent, StreamMessage, ContentBlockStart, TextDelta, MessageDelta, Usage};

// OpenAI-specific data structures for API communication

/// OpenAI API request structure
#[derive(Serialize, Debug, Deserialize, Clone)]
pub struct OpenAIRequest {
    pub model: String,
    pub messages: Vec<OpenAIMessage>,
    pub max_tokens: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frequency_penalty: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub presence_penalty: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user: Option<String>,
}

/// Message structure for OpenAI conversations
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct OpenAIMessage {
    pub role: String, // "system", "user", "assistant"
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

/// OpenAI API response structure
#[derive(Deserialize, Debug)]
pub struct OpenAIResponse {
    pub id: String,
    pub object: String,
    pub created: u64,
    pub model: String,
    pub choices: Vec<OpenAIChoice>,
    pub usage: OpenAIUsage,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_fingerprint: Option<String>,
}

/// Individual choice in OpenAI response
#[derive(Deserialize, Debug)]
pub struct OpenAIChoice {
    pub index: u32,
    pub message: OpenAIMessage,
    pub finish_reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logprobs: Option<serde_json::Value>,
}

/// Token usage information from OpenAI
#[derive(Deserialize, Debug)]
pub struct OpenAIUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

/// OpenAI API error response
#[derive(Deserialize, Debug)]
pub struct OpenAIError {
    pub error: OpenAIErrorDetail,
}

/// Detailed error information from OpenAI
#[derive(Deserialize, Debug)]
pub struct OpenAIErrorDetail {
    pub message: String,
    #[serde(rename = "type")]
    pub error_type: String,
    pub param: Option<String>,
    pub code: Option<String>,
}

// Streaming-specific structures for OpenAI

/// OpenAI streaming response structure
#[derive(Deserialize, Debug)]
pub struct OpenAIStreamResponse {
    pub id: String,
    pub object: String,
    pub created: u64,
    pub model: String,
    pub choices: Vec<OpenAIStreamChoice>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_fingerprint: Option<String>,
}

/// Streaming choice structure
#[derive(Deserialize, Debug)]
pub struct OpenAIStreamChoice {
    pub index: u32,
    pub delta: OpenAIStreamDelta,
    pub finish_reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logprobs: Option<serde_json::Value>,
}

/// Delta structure for streaming updates
#[derive(Deserialize, Debug)]
pub struct OpenAIStreamDelta {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
}

/// Conversion functions for OpenAI format
impl OpenAIRequest {
    /// Convert Anthropic request format to OpenAI format
    pub fn from_anthropic(request: &AnthropicRequest) -> Result<Self, AppError> {
        let messages = request
            .messages
            .iter()
            .map(|msg| OpenAIMessage {
                role: msg.role.clone(),
                content: msg.content.clone(),
                name: None,
            })
            .collect();

        Ok(OpenAIRequest {
            model: request.model.clone(),
            messages,
            max_tokens: request.max_tokens,
            stream: request.stream,
            temperature: request.temperature,
            top_p: request.top_p,
            frequency_penalty: None,
            presence_penalty: None,
            stop: None,
            user: None,
        })
    }

    /// Create a new OpenAI request with default values
    pub fn new(model: String, messages: Vec<OpenAIMessage>, max_tokens: u32) -> Self {
        Self {
            model,
            messages,
            max_tokens,
            stream: None,
            temperature: None,
            top_p: None,
            frequency_penalty: None,
            presence_penalty: None,
            stop: None,
            user: None,
        }
    }

    /// Set streaming mode
    pub fn with_stream(mut self, stream: bool) -> Self {
        self.stream = Some(stream);
        self
    }

    /// Set temperature parameter
    pub fn with_temperature(mut self, temperature: f32) -> Self {
        self.temperature = Some(temperature);
        self
    }

    /// Set top_p parameter
    pub fn with_top_p(mut self, top_p: f32) -> Self {
        self.top_p = Some(top_p);
        self
    }

    /// Set frequency penalty
    pub fn with_frequency_penalty(mut self, penalty: f32) -> Self {
        self.frequency_penalty = Some(penalty);
        self
    }

    /// Set presence penalty
    pub fn with_presence_penalty(mut self, penalty: f32) -> Self {
        self.presence_penalty = Some(penalty);
        self
    }

    /// Set stop sequences
    pub fn with_stop(mut self, stop: Vec<String>) -> Self {
        self.stop = Some(stop);
        self
    }

    /// Set user identifier
    pub fn with_user(mut self, user: String) -> Self {
        self.user = Some(user);
        self
    }

    /// Validate the OpenAI request
    pub fn validate(&self) -> Result<(), AppError> {
        // Validate model
        if self.model.is_empty() {
            return Err(AppError::ValidationError("Model cannot be empty".to_string()));
        }

        // Validate messages
        if self.messages.is_empty() {
            return Err(AppError::ValidationError("Messages cannot be empty".to_string()));
        }

        if self.messages.len() > 100 {
            return Err(AppError::ValidationError("Too many messages (max 100)".to_string()));
        }

        // Validate max_tokens
        if self.max_tokens == 0 {
            return Err(AppError::ValidationError("max_tokens must be greater than 0".to_string()));
        }

        if self.max_tokens > 4096 {
            return Err(AppError::ValidationError("max_tokens cannot exceed 4096".to_string()));
        }

        // Validate temperature
        if let Some(temp) = self.temperature {
            if temp.is_nan() || temp.is_infinite() {
                return Err(AppError::ValidationError("temperature must be a valid number".to_string()));
            }
            if temp < 0.0 || temp > 2.0 {
                return Err(AppError::ValidationError("temperature must be between 0.0 and 2.0".to_string()));
            }
        }

        // Validate top_p
        if let Some(top_p) = self.top_p {
            if top_p.is_nan() || top_p.is_infinite() {
                return Err(AppError::ValidationError("top_p must be a valid number".to_string()));
            }
            if top_p < 0.0 || top_p > 1.0 {
                return Err(AppError::ValidationError("top_p must be between 0.0 and 1.0".to_string()));
            }
        }

        // Validate frequency_penalty
        if let Some(penalty) = self.frequency_penalty {
            if penalty.is_nan() || penalty.is_infinite() {
                return Err(AppError::ValidationError("frequency_penalty must be a valid number".to_string()));
            }
            if penalty < -2.0 || penalty > 2.0 {
                return Err(AppError::ValidationError("frequency_penalty must be between -2.0 and 2.0".to_string()));
            }
        }

        // Validate presence_penalty
        if let Some(penalty) = self.presence_penalty {
            if penalty.is_nan() || penalty.is_infinite() {
                return Err(AppError::ValidationError("presence_penalty must be a valid number".to_string()));
            }
            if penalty < -2.0 || penalty > 2.0 {
                return Err(AppError::ValidationError("presence_penalty must be between -2.0 and 2.0".to_string()));
            }
        }

        // Validate stop sequences
        if let Some(stop) = &self.stop {
            if stop.len() > 4 {
                return Err(AppError::ValidationError("Too many stop sequences (max 4)".to_string()));
            }
        }

        Ok(())
    }

    /// Convert to JSON string for debugging
    pub fn to_json_string(&self) -> Result<String, AppError> {
        serde_json::to_string_pretty(self)
            .map_err(|e| AppError::ValidationError(format!("Failed to serialize request: {}", e)))
    }
}

impl OpenAIResponse {
    /// Convert OpenAI response format to Anthropic format
    pub fn to_anthropic(&self) -> Result<AnthropicResponse, AppError> {
        let choice = self
            .choices
            .first()
            .ok_or_else(|| AppError::ProviderError {
                status: 500,
                message: "No choices in OpenAI response".to_string(),
            })?;

        let text = choice.message.content.clone();

        if text.is_empty() {
            return Err(AppError::ProviderError {
                status: 500,
                message: "Empty response content from OpenAI".to_string(),
            });
        }

        Ok(AnthropicResponse::new(
            self.id.clone(),
            self.model.clone(),
            text,
            self.usage.prompt_tokens,
            self.usage.completion_tokens,
        ))
    }

    /// Get finish reason as human-readable string
    pub fn get_finish_reason(&self) -> Option<String> {
        self.choices
            .first()
            .and_then(|c| c.finish_reason.as_ref())
            .map(|reason| match reason.as_str() {
                "stop" => "Response completed naturally".to_string(),
                "length" => "Response reached maximum token limit".to_string(),
                "content_filter" => "Response blocked by content filter".to_string(),
                "function_call" => "Response ended with function call".to_string(),
                "tool_calls" => "Response ended with tool calls".to_string(),
                _ => format!("Unknown finish reason: {}", reason),
            })
    }

    /// Get usage information as a string for logging
    pub fn get_usage_info(&self) -> String {
        format!(
            "prompt_tokens: {}, completion_tokens: {}, total_tokens: {}",
            self.usage.prompt_tokens,
            self.usage.completion_tokens,
            self.usage.total_tokens
        )
    }

    /// Check if response has any issues
    pub fn has_issues(&self) -> bool {
        self.choices.is_empty() || 
        self.choices.iter().any(|c| c.message.content.is_empty())
    }
}

impl OpenAIStreamResponse {
    /// Convert OpenAI streaming response to Anthropic streaming events
    pub fn to_anthropic_events(&self, _message_id: &str) -> Result<Vec<AnthropicStreamEvent>, AppError> {
        let mut events = Vec::new();

        for choice in &self.choices {
            // Handle content delta
            if let Some(content) = &choice.delta.content {
                if !content.is_empty() {
                    events.push(AnthropicStreamEvent::ContentBlockDelta {
                        index: choice.index,
                        delta: TextDelta {
                            type_field: "text_delta".to_string(),
                            text: content.clone(),
                        },
                    });
                }
            }

            // Handle finish reason
            if let Some(finish_reason) = &choice.finish_reason {
                let stop_reason = match finish_reason.as_str() {
                    "stop" => Some("end_turn".to_string()),
                    "length" => Some("max_tokens".to_string()),
                    "content_filter" => Some("stop_sequence".to_string()),
                    "function_call" => Some("tool_use".to_string()),
                    "tool_calls" => Some("tool_use".to_string()),
                    _ => Some("stop_sequence".to_string()),
                };

                events.push(AnthropicStreamEvent::MessageDelta {
                    delta: MessageDelta {
                        stop_reason,
                        usage: None, // OpenAI doesn't provide usage in streaming
                    },
                });

                events.push(AnthropicStreamEvent::MessageStop);
            }
        }

        Ok(events)
    }

    /// Create initial streaming events for message start
    pub fn create_message_start_event(model: &str, message_id: &str) -> AnthropicStreamEvent {
        AnthropicStreamEvent::MessageStart {
            message: StreamMessage {
                id: message_id.to_string(),
                model: model.to_string(),
                role: "assistant".to_string(),
                content: vec![],
                usage: Usage {
                    input_tokens: 0,
                    output_tokens: 0,
                },
            },
        }
    }

    /// Create content block start event
    pub fn create_content_block_start_event() -> AnthropicStreamEvent {
        AnthropicStreamEvent::ContentBlockStart {
            index: 0,
            content_block: ContentBlockStart {
                type_field: "text".to_string(),
                text: String::new(),
            },
        }
    }

    /// Create error event for streaming
    pub fn create_error_event(error: &AppError) -> AnthropicStreamEvent {
        use crate::providers::anthropic::StreamError;
        AnthropicStreamEvent::Error {
            error: StreamError {
                error_type: "provider_error".to_string(),
                message: error.to_string(),
            },
        }
    }

    /// Check if streaming response has any issues
    pub fn has_streaming_issues(&self) -> bool {
        self.choices.is_empty()
    }
}

/// Utility functions for OpenAI data transformations
pub mod openai_utils {
    use super::*;

    /// Create a simple OpenAI request from text content
    pub fn create_simple_request(content: String, model: String, max_tokens: u32) -> OpenAIRequest {
        let message = OpenAIMessage {
            role: "user".to_string(),
            content,
            name: None,
        };
        
        OpenAIRequest::new(model, vec![message], max_tokens)
    }

    /// Create a conversation request from multiple messages
    pub fn create_conversation_request(
        messages: Vec<(String, String)>,
        model: String,
        max_tokens: u32,
    ) -> Result<OpenAIRequest, AppError> {
        let openai_messages = messages
            .into_iter()
            .map(|(role, content)| {
                // Validate role
                if !matches!(role.as_str(), "user" | "assistant" | "system") {
                    return Err(AppError::ValidationError(format!(
                        "Invalid role: {}. Use 'user', 'assistant', or 'system'",
                        role
                    )));
                }

                Ok(OpenAIMessage {
                    role,
                    content,
                    name: None,
                })
            })
            .collect::<Result<Vec<_>, AppError>>()?;

        Ok(OpenAIRequest::new(model, openai_messages, max_tokens))
    }

    /// Create a system message
    pub fn create_system_message(content: String) -> OpenAIMessage {
        OpenAIMessage {
            role: "system".to_string(),
            content,
            name: None,
        }
    }

    /// Create a user message
    pub fn create_user_message(content: String) -> OpenAIMessage {
        OpenAIMessage {
            role: "user".to_string(),
            content,
            name: None,
        }
    }

    /// Create an assistant message
    pub fn create_assistant_message(content: String) -> OpenAIMessage {
        OpenAIMessage {
            role: "assistant".to_string(),
            content,
            name: None,
        }
    }

    /// Parse OpenAI error response
    pub fn parse_error_response(error_body: &str) -> String {
        match serde_json::from_str::<OpenAIError>(error_body) {
            Ok(error) => error.error.message,
            Err(_) => error_body.to_string(),
        }
    }

    /// Check if model supports streaming
    pub fn supports_streaming(model: &str) -> bool {
        // Most OpenAI models support streaming
        !model.contains("embedding") && !model.contains("whisper") && !model.contains("tts")
    }

    /// Get recommended max_tokens for model
    pub fn get_recommended_max_tokens(model: &str) -> u32 {
        match model {
            m if m.contains("gpt-4") => 4096,
            m if m.contains("gpt-3.5-turbo-16k") => 16384,
            m if m.contains("gpt-3.5") => 4096,
            _ => 2048,
        }
    }

    /// Validate model name format
    pub fn validate_model_name(model: &str) -> Result<(), AppError> {
        if model.is_empty() {
            return Err(AppError::ValidationError("Model name cannot be empty".to_string()));
        }

        if model.len() > 100 {
            return Err(AppError::ValidationError("Model name too long (max 100 characters)".to_string()));
        }

        // Check for valid OpenAI model name patterns
        let valid_prefixes = ["gpt-", "text-", "code-", "davinci", "curie", "babbage", "ada"];
        if !valid_prefixes.iter().any(|prefix| model.starts_with(prefix)) {
            return Err(AppError::ValidationError(format!(
                "Invalid OpenAI model name: {}. Must start with one of: {}",
                model,
                valid_prefixes.join(", ")
            )));
        }

        Ok(())
    }
}
