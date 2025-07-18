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
    /// 验证单个消息的有效性
    ///
    /// ## 功能说明
    /// 验证消息的角色和内容是否符合API规范要求
    ///
    /// ## 内部实现逻辑
    /// 1. 验证角色必须是"user"或"assistant"
    /// 2. 验证内容不能为空
    /// 3. 验证内容长度不超过100KB
    /// 4. 检查内容中不包含空字节等问题字符
    ///
    /// ## 验证规则
    /// - `role`: 必须是"user"或"assistant"
    /// - `content`: 不能为空，长度不超过100,000字符，不包含空字节
    ///
    /// ## 执行例子
    /// ```rust
    /// let message = Message::user("Hello, AI!".to_string());
    /// message.validate()?;
    /// ```
    pub fn validate(&self) -> Result<(), String> {
        // 角色验证
        if self.role != "user" && self.role != "assistant" {
            return Err(format!("Invalid role '{}': must be 'user' or 'assistant'", self.role));
        }

        // 内容验证
        if self.content.is_empty() {
            return Err("Message content cannot be empty".to_string());
        }

        if self.content.len() > 100_000 {
            return Err("Message content too long (max 100KB)".to_string());
        }

        // 检查空字节或其他问题字符
        if self.content.contains('\0') {
            return Err("Message content cannot contain null bytes".to_string());
        }

        Ok(())
    }

    /// 创建新的用户消息
    ///
    /// ## 功能说明
    /// 便捷方法，创建角色为"user"的消息实例
    ///
    /// ## 参数说明
    /// - `content`: 消息内容
    ///
    /// ## 执行例子
    /// ```rust
    /// let user_msg = Message::user("What's the weather like?".to_string());
    /// ```
    pub fn user(content: String) -> Self {
        Self {
            role: "user".to_string(),
            content,
        }
    }

    /// 创建新的助手消息
    ///
    /// ## 功能说明
    /// 便捷方法，创建角色为"assistant"的消息实例
    ///
    /// ## 参数说明
    /// - `content`: 消息内容
    ///
    /// ## 执行例子
    /// ```rust
    /// let assistant_msg = Message::assistant("The weather is sunny today.".to_string());
    /// ```
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
    /// 全面验证请求参数的有效性
    ///
    /// ## 功能说明
    /// 对聊天请求的所有参数进行全面验证，确保请求符合API规范
    ///
    /// ## 内部实现逻辑
    /// 1. 验证模型名称的格式和有效性
    /// 2. 验证消息数组的结构和内容
    /// 3. 验证token限制的合理性
    /// 4. 验证可选参数的取值范围
    /// 5. 验证总内容长度不超过限制
    ///
    /// ## 验证项目
    /// - **模型验证**: 名称格式、长度限制
    /// - **消息验证**: 数量限制、角色序列、内容有效性
    /// - **Token验证**: max_tokens范围检查
    /// - **参数验证**: temperature和top_p取值范围
    /// - **长度验证**: 总内容长度限制
    ///
    /// ## 执行例子
    /// ```rust
    /// let request = AnthropicRequest {
    ///     model: "claude-3-sonnet".to_string(),
    ///     messages: vec![Message::user("Hello".to_string())],
    ///     max_tokens: 1000,
    ///     temperature: Some(0.7),
    ///     top_p: Some(0.9),
    ///     stream: Some(false),
    /// };
    /// request.validate()?;
    /// ```
    pub fn validate(&self) -> Result<(), String> {
        // 模型验证
        self.validate_model()?;

        // 消息验证
        self.validate_messages()?;

        // Token限制验证
        self.validate_token_limits()?;

        // 参数范围验证
        self.validate_parameters()?;

        // 内容长度验证
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
        // if self.messages[0].role != "user" {
            // return Err("Conversation must start with a user message".to_string());
        // }
        
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
    
    /// 检查请求是否为流式传输
    ///
    /// ## 功能说明
    /// 检查请求是否启用了流式响应模式
    ///
    /// ## 执行例子
    /// ```rust
    /// if request.is_streaming() {
    ///     // 处理流式响应
    /// } else {
    ///     // 处理普通响应
    /// }
    /// ```
    ///
    /// ## 返回值
    /// - `true`: 启用流式传输
    /// - `false`: 使用普通响应模式
    pub fn is_streaming(&self) -> bool {
        self.stream.unwrap_or(false)
    }

    /// 估算输入token数量（粗略近似）
    ///
    /// ## 功能说明
    /// 基于字符数量粗略估算请求的输入token数量
    ///
    /// ## 内部实现逻辑
    /// 1. 计算所有消息内容和角色的总字符数
    /// 2. 使用1 token ≈ 4字符的粗略比例进行估算
    /// 3. 确保至少返回1个token
    ///
    /// ## 执行例子
    /// ```rust
    /// let estimated_tokens = request.estimate_input_tokens();
    /// println!("Estimated input tokens: {}", estimated_tokens);
    /// ```
    ///
    /// ## 返回值
    /// - `u32`: 估算的输入token数量
    pub fn estimate_input_tokens(&self) -> u32 {
        // 粗略估算：1 token ≈ 4 字符
        let total_chars: usize = self.messages.iter()
            .map(|m| m.content.len() + m.role.len())
            .sum();
        (total_chars / 4).max(1) as u32
    }
}

impl AnthropicResponse {
    /// 创建新的响应对象
    ///
    /// ## 功能说明
    /// 便捷方法，根据给定参数创建标准格式的Anthropic响应对象
    ///
    /// ## 内部实现逻辑
    /// 1. 设置响应ID和模型名称
    /// 2. 创建包含文本内容的ContentBlock
    /// 3. 设置token使用统计信息
    /// 4. 返回完整的响应对象
    ///
    /// ## 参数说明
    /// - `id`: 响应的唯一标识符
    /// - `model`: 使用的模型名称
    /// - `text`: 生成的文本内容
    /// - `input_tokens`: 输入消耗的token数量
    /// - `output_tokens`: 输出生成的token数量
    ///
    /// ## 执行例子
    /// ```rust
    /// let response = AnthropicResponse::new(
    ///     "msg_123".to_string(),
    ///     "claude-3-sonnet".to_string(),
    ///     "Hello! How can I help you?".to_string(),
    ///     10,
    ///     25
    /// );
    /// ```
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