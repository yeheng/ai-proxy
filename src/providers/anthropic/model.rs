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
            if temp.is_nan() || temp.is_infinite() {
                return Err("temperature must be a valid number".to_string());
            }
            if temp < 0.0 || temp > 2.0 {
                return Err("temperature must be between 0.0 and 2.0".to_string());
            }
        }
        
        if let Some(top_p) = self.top_p {
            if top_p.is_nan() || top_p.is_infinite() {
                return Err("top_p must be a valid number".to_string());
            }
            if top_p < 0.0 || top_p > 1.0 {
                return Err("top_p must be between 0.0 and 1.0".to_string());
            }
        }
        
        Ok(())
    }
    
    /// Validate total content length
    fn validate_content_length(&self) -> Result<(), String> {
        let total_content_length: usize = self.messages.iter()
            .map(|m| m.content.len())
            .sum();
        
        if total_content_length > 100_000 {
            return Err("Total content length exceeds maximum (100KB)".to_string());
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