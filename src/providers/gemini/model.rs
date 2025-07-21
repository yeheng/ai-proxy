use crate::errors::AppError;
use crate::providers::anthropic::{
    AnthropicRequest, AnthropicResponse, AnthropicStreamEvent, ContentBlockStart, MessageDelta,
    StreamMessage, TextDelta, Usage,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// Gemini-specific data structures for API communication

/// Gemini API request structure
#[derive(Serialize, Debug, Deserialize, Clone)]
pub struct GeminiRequest {
    pub contents: Vec<GeminiContent>,
    #[serde(rename = "generationConfig")]
    pub generation_config: GenerationConfig,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_instruction: Option<GeminiContent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub safety_settings: Option<Vec<SafetySetting>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<Tool>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_config: Option<ToolConfig>,
}

/// Content structure for Gemini messages
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GeminiContent {
    pub role: String,
    pub parts: Vec<GeminiPart>,
}

/// Part structure containing text content
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GeminiPart {
    pub text: String,
}

/// Generation configuration for Gemini API
#[derive(Serialize, Debug, Deserialize, Clone)]
pub struct GenerationConfig {
    #[serde(rename = "maxOutputTokens")]
    pub max_output_tokens: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "topP")]
    pub top_p: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "topK")]
    pub top_k: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_sequences: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "responseMimeType")]
    pub response_mime_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "responseSchema")]
    pub response_schema: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "candidateCount")]
    pub candidate_count: Option<i32>,
}

/// Gemini API response structure
#[derive(Deserialize, Debug)]
pub struct GeminiResponse {
    pub candidates: Vec<GeminiCandidate>,
    #[serde(rename = "usageMetadata")]
    pub usage_metadata: Option<UsageMetadata>,
    #[serde(rename = "promptFeedback")]
    pub prompt_feedback: Option<PromptFeedback>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<GeminiError>,
}

/// Individual candidate in Gemini response
#[derive(Deserialize, Debug)]
pub struct GeminiCandidate {
    pub content: GeminiContent,
    #[serde(rename = "finishReason")]
    pub finish_reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub index: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub safety_ratings: Option<Vec<SafetyRating>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub citation_metadata: Option<CitationMetadata>,
}

/// Safety settings for content filtering
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SafetySetting {
    pub category: HarmCategory,
    pub threshold: HarmBlockThreshold,
}

/// Safety rating for content
#[derive(Deserialize, Debug)]
pub struct SafetyRating {
    pub category: HarmCategory,
    pub probability: HarmProbability,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blocked: Option<bool>,
}

/// Harm categories for safety settings
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum HarmCategory {
    HarmCategoryUnspecified,
    Harassment,
    HateSpeech,
    SexuallyExplicit,
    DangerousContent,
}

/// Harm block threshold levels
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum HarmBlockThreshold {
    HarmBlockThresholdUnspecified,
    BlockLowAndAbove,
    BlockMediumAndAbove,
    BlockOnlyHigh,
    BlockNone,
}

/// Harm probability levels
#[derive(Deserialize, Debug)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum HarmProbability {
    HarmProbabilityUnspecified,
    Negligible,
    Low,
    Medium,
    High,
}

/// Citation metadata for sources
#[derive(Deserialize, Debug)]
pub struct CitationMetadata {
    pub citations: Vec<Citation>,
}

/// Individual citation
#[derive(Deserialize, Debug)]
pub struct Citation {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_index: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_index: Option<i32>,
    pub uri: String,
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub license: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub publication_date: Option<String>,
}

/// Prompt feedback information
#[derive(Deserialize, Debug)]
pub struct PromptFeedback {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub block_reason: Option<BlockReason>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub safety_ratings: Option<Vec<SafetyRating>>,
}

/// Block reasons for prompt feedback
#[derive(Deserialize, Debug)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum BlockReason {
    BlockReasonUnspecified,
    Safety,
    Other,
}

/// Gemini API error response
#[derive(Deserialize, Debug)]
pub struct GeminiError {
    pub code: i32,
    pub message: String,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<Vec<serde_json::Value>>,
}

/// Tool definition for function calling
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Tool {
    #[serde(rename = "functionDeclarations")]
    pub function_declarations: Vec<FunctionDeclaration>,
}

/// Function declaration within a tool
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FunctionDeclaration {
    pub name: String,
    pub description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parameters: Option<Schema>,
}

/// Schema for function parameters
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Schema {
    #[serde(rename = "type")]
    pub type_field: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub properties: Option<HashMap<String, SchemaProperty>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required: Option<Vec<String>>,
}

/// Individual property within a schema
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SchemaProperty {
    #[serde(rename = "type")]
    pub type_field: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub items: Option<Box<SchemaProperty>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enum_values: Option<Vec<serde_json::Value>>,
}

/// Tool configuration for controlling function calling
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ToolConfig {
    #[serde(rename = "functionCallingConfig")]
    pub function_calling_config: FunctionCallingConfig,
}

/// Function calling configuration
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FunctionCallingConfig {
    pub mode: FunctionCallingMode,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allowed_function_names: Option<Vec<String>>,
}

/// Function calling modes
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum FunctionCallingMode {
    FunctionCallingModeUnspecified,
    Auto,
    None,
    Any,
}

/// Token usage metadata from Gemini
#[derive(Deserialize, Debug)]
pub struct UsageMetadata {
    #[serde(rename = "promptTokenCount")]
    pub prompt_token_count: Option<u32>,
    #[serde(rename = "candidatesTokenCount")]
    pub candidates_token_count: Option<u32>,
    #[serde(rename = "totalTokenCount")]
    pub total_token_count: Option<u32>,
}

// Streaming-specific structures for Gemini

/// Gemini streaming response structure
#[derive(Deserialize, Debug)]
pub struct GeminiStreamResponse {
    pub candidates: Option<Vec<GeminiStreamCandidate>>,
    #[serde(rename = "usageMetadata")]
    pub usage_metadata: Option<UsageMetadata>,
}

/// Streaming candidate structure
#[derive(Deserialize, Debug)]
pub struct GeminiStreamCandidate {
    pub content: Option<GeminiContent>,
    #[serde(rename = "finishReason")]
    pub finish_reason: Option<String>,
    pub index: Option<u32>,
}

/// Conversion functions for Gemini format
impl GeminiRequest {
    /// Convert Anthropic request format to Gemini format
    pub fn from_anthropic(request: &AnthropicRequest) -> Result<Self, AppError> {
        let contents = request
            .messages
            .iter()
            .map(|msg| {
                let role = match msg.role.as_str() {
                    "user" => "user",
                    "assistant" => "model", // Gemini uses "model" instead of "assistant"
                    _ => {
                        return Err(AppError::ValidationError(format!(
                            "Invalid role: {}",
                            msg.role
                        )));
                    }
                };

                Ok(GeminiContent {
                    role: role.to_string(),
                    parts: vec![GeminiPart {
                        text: msg.content.clone(),
                    }],
                })
            })
            .collect::<Result<Vec<_>, AppError>>()?;

        Ok(GeminiRequest {
            contents,
            generation_config: GenerationConfig {
                max_output_tokens: request.max_tokens,
                temperature: request.temperature,
                top_p: request.top_p,
                top_k: None,
                stop_sequences: None,
                response_mime_type: None,
                response_schema: None,
                candidate_count: None,
            },
            system_instruction: None,
            safety_settings: None,
            tools: None,
            tool_config: None,
        })
    }

    /// Create a new Gemini request with default values
    pub fn new(contents: Vec<GeminiContent>, max_output_tokens: u32) -> Self {
        Self {
            contents,
            generation_config: GenerationConfig {
                max_output_tokens,
                temperature: None,
                top_p: None,
                top_k: None,
                stop_sequences: None,
                response_mime_type: None,
                response_schema: None,
                candidate_count: None,
            },
            system_instruction: None,
            safety_settings: None,
            tools: None,
            tool_config: None,
        }
    }

    /// Set system instruction for the request
    pub fn with_system_instruction(mut self, instruction: String) -> Self {
        self.system_instruction = Some(GeminiContent {
            role: "system".to_string(),
            parts: vec![GeminiPart { text: instruction }],
        });
        self
    }

    /// Set safety settings for the request
    pub fn with_safety_settings(mut self, settings: Vec<SafetySetting>) -> Self {
        self.safety_settings = Some(settings);
        self
    }

    /// Set tools for function calling
    pub fn with_tools(mut self, tools: Vec<Tool>) -> Self {
        self.tools = Some(tools);
        self
    }

    /// Set tool configuration
    pub fn with_tool_config(mut self, config: ToolConfig) -> Self {
        self.tool_config = Some(config);
        self
    }

    /// Create safety settings with default values
    pub fn default_safety_settings() -> Vec<SafetySetting> {
        vec![
            SafetySetting {
                category: HarmCategory::Harassment,
                threshold: HarmBlockThreshold::BlockMediumAndAbove,
            },
            SafetySetting {
                category: HarmCategory::HateSpeech,
                threshold: HarmBlockThreshold::BlockMediumAndAbove,
            },
            SafetySetting {
                category: HarmCategory::SexuallyExplicit,
                threshold: HarmBlockThreshold::BlockMediumAndAbove,
            },
            SafetySetting {
                category: HarmCategory::DangerousContent,
                threshold: HarmBlockThreshold::BlockMediumAndAbove,
            },
        ]
    }

    /// Set safety settings with custom thresholds
    pub fn with_custom_safety_settings(
        mut self,
        settings: Vec<(HarmCategory, HarmBlockThreshold)>,
    ) -> Self {
        self.safety_settings = Some(
            settings
                .into_iter()
                .map(|(category, threshold)| SafetySetting {
                    category,
                    threshold,
                })
                .collect(),
        );
        self
    }

    /// Add a single safety setting
    pub fn with_safety_setting(
        mut self,
        category: HarmCategory,
        threshold: HarmBlockThreshold,
    ) -> Self {
        let mut settings = self.safety_settings.unwrap_or_default();
        settings.push(SafetySetting {
            category,
            threshold,
        });
        self.safety_settings = Some(settings);
        self
    }

    /// Validate the Gemini request
    pub fn validate(&self) -> Result<(), AppError> {
        // Validate contents
        if self.contents.is_empty() {
            return Err(AppError::ValidationError(
                "Contents cannot be empty".to_string(),
            ));
        }

        if self.contents.len() > 100 {
            return Err(AppError::ValidationError(
                "Too many contents (max 100)".to_string(),
            ));
        }

        // Validate generation config
        if self.generation_config.max_output_tokens == 0 {
            return Err(AppError::ValidationError(
                "max_output_tokens must be greater than 0".to_string(),
            ));
        }

        if self.generation_config.max_output_tokens > 8192 {
            return Err(AppError::ValidationError(
                "max_output_tokens cannot exceed 8192".to_string(),
            ));
        }

        // Validate temperature
        if let Some(temp) = self.generation_config.temperature {
            if temp.is_nan() || temp.is_infinite() {
                return Err(AppError::ValidationError(
                    "temperature must be a valid number".to_string(),
                ));
            }
            if temp < 0.0 || temp > 2.0 {
                return Err(AppError::ValidationError(
                    "temperature must be between 0.0 and 2.0".to_string(),
                ));
            }
        }

        // Validate top_p
        if let Some(top_p) = self.generation_config.top_p {
            if top_p.is_nan() || top_p.is_infinite() {
                return Err(AppError::ValidationError(
                    "top_p must be a valid number".to_string(),
                ));
            }
            if top_p < 0.0 || top_p > 1.0 {
                return Err(AppError::ValidationError(
                    "top_p must be between 0.0 and 1.0".to_string(),
                ));
            }
        }

        // Validate top_k
        if let Some(top_k) = self.generation_config.top_k {
            if top_k < 1 || top_k > 40 {
                return Err(AppError::ValidationError(
                    "top_k must be between 1 and 40".to_string(),
                ));
            }
        }

        // Validate candidate count
        if let Some(candidate_count) = self.generation_config.candidate_count {
            if candidate_count < 1 || candidate_count > 8 {
                return Err(AppError::ValidationError(
                    "candidate_count must be between 1 and 8".to_string(),
                ));
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

impl GeminiResponse {
    /// Convert Gemini response format to Anthropic format
    pub fn to_anthropic(&self, model: &str) -> Result<AnthropicResponse, AppError> {
        // Check for API error first
        if let Some(error) = &self.error {
            return Err(AppError::ProviderError {
                status: error.code as u16,
                message: error.message.clone(),
            });
        }

        // Check for prompt feedback that might block the response
        if let Some(feedback) = &self.prompt_feedback {
            if let Some(block_reason) = &feedback.block_reason {
                return Err(AppError::ProviderError {
                    status: 400,
                    message: format!("Prompt blocked: {:?}", block_reason),
                });
            }
        }

        let candidate = self
            .candidates
            .first()
            .ok_or_else(|| AppError::ProviderError {
                status: 500,
                message: "No candidates in Gemini response".to_string(),
            })?;

        // Check if response was blocked by safety ratings
        if let Some(safety_ratings) = &candidate.safety_ratings {
            for rating in safety_ratings {
                if rating.blocked.unwrap_or(false) {
                    return Err(AppError::ProviderError {
                        status: 400,
                        message: format!(
                            "Response blocked by safety filter: {:?}",
                            rating.category
                        ),
                    });
                }
            }
        }

        let text = candidate
            .content
            .parts
            .iter()
            .map(|part| part.text.as_str())
            .collect::<Vec<_>>()
            .join("");

        if text.is_empty() {
            return Err(AppError::ProviderError {
                status: 500,
                message: "Empty response content from Gemini".to_string(),
            });
        }

        let usage = self.usage_metadata.as_ref().unwrap_or(&UsageMetadata {
            prompt_token_count: Some(0),
            candidates_token_count: Some(0),
            total_token_count: Some(0),
        });

        Ok(AnthropicResponse::new(
            format!("msg_{}", uuid::Uuid::new_v4().simple()),
            model.to_string(),
            text,
            usage.prompt_token_count.unwrap_or(0),
            usage.candidates_token_count.unwrap_or(0),
        ))
    }

    /// Check if response contains any safety issues
    pub fn has_safety_issues(&self) -> bool {
        if self.error.is_some() {
            return true;
        }

        if let Some(feedback) = &self.prompt_feedback {
            if feedback.block_reason.is_some() {
                return true;
            }
        }

        for candidate in &self.candidates {
            if let Some(safety_ratings) = &candidate.safety_ratings {
                for rating in safety_ratings {
                    if rating.blocked.unwrap_or(false) {
                        return true;
                    }
                }
            }
        }

        false
    }

    /// Get finish reason as human-readable string
    pub fn get_finish_reason(&self) -> Option<String> {
        self.candidates
            .first()
            .and_then(|c| c.finish_reason.as_ref())
            .map(|reason| match reason.as_str() {
                "STOP" => "Response completed naturally".to_string(),
                "MAX_TOKENS" => "Response reached maximum token limit".to_string(),
                "SAFETY" => "Response blocked for safety reasons".to_string(),
                "RECITATION" => "Response blocked due to recitation".to_string(),
                "OTHER" => "Response stopped for other reasons".to_string(),
                _ => format!("Unknown finish reason: {}", reason),
            })
    }

    /// Get usage information as a string for logging
    pub fn get_usage_info(&self) -> String {
        match &self.usage_metadata {
            Some(usage) => {
                format!(
                    "prompt_tokens: {}, completion_tokens: {}, total_tokens: {}",
                    usage.prompt_token_count.unwrap_or(0),
                    usage.candidates_token_count.unwrap_or(0),
                    usage.total_token_count.unwrap_or(0)
                )
            }
            None => "No usage information available".to_string(),
        }
    }

    /// Get safety information as a string for logging
    pub fn get_safety_info(&self) -> String {
        let mut safety_info = Vec::new();

        if let Some(feedback) = &self.prompt_feedback {
            if let Some(reason) = &feedback.block_reason {
                safety_info.push(format!("Prompt blocked: {:?}", reason));
            }
            if let Some(ratings) = &feedback.safety_ratings {
                for rating in ratings {
                    safety_info.push(format!(
                        "Prompt safety: {:?} ({:?})",
                        rating.category, rating.probability
                    ));
                }
            }
        }

        for (i, candidate) in self.candidates.iter().enumerate() {
            if let Some(ratings) = &candidate.safety_ratings {
                for rating in ratings {
                    let blocked = rating.blocked.unwrap_or(false);
                    safety_info.push(format!(
                        "Candidate {}: {:?} ({:?}) blocked: {}",
                        i, rating.category, rating.probability, blocked
                    ));
                }
            }
        }

        if safety_info.is_empty() {
            "No safety issues detected".to_string()
        } else {
            safety_info.join("; ")
        }
    }

    /// Check if any safety ratings indicate high risk
    pub fn has_high_risk_safety_rating(&self) -> bool {
        for candidate in &self.candidates {
            if let Some(safety_ratings) = &candidate.safety_ratings {
                for rating in safety_ratings {
                    if matches!(rating.probability, HarmProbability::High) {
                        return true;
                    }
                }
            }
        }
        false
    }

    /// Get all safety ratings across all candidates
    pub fn get_all_safety_ratings(&self) -> Vec<&SafetyRating> {
        let mut ratings = Vec::new();

        for candidate in &self.candidates {
            if let Some(candidate_ratings) = &candidate.safety_ratings {
                ratings.extend(candidate_ratings);
            }
        }

        if let Some(feedback) = &self.prompt_feedback {
            if let Some(feedback_ratings) = &feedback.safety_ratings {
                ratings.extend(feedback_ratings);
            }
        }

        ratings
    }
}

impl GeminiStreamResponse {
    /// Convert Gemini streaming response to Anthropic streaming events
    pub fn to_anthropic_events(
        &self,
        _model: &str,
        _message_id: &str,
    ) -> Result<Vec<AnthropicStreamEvent>, AppError> {
        let mut events = Vec::new();

        if let Some(candidates) = &self.candidates {
            for candidate in candidates {
                if let Some(content) = &candidate.content {
                    // Extract text from parts
                    let text = content
                        .parts
                        .iter()
                        .map(|part| part.text.as_str())
                        .collect::<Vec<_>>()
                        .join("");

                    if !text.is_empty() {
                        // Create content block delta event
                        events.push(AnthropicStreamEvent::ContentBlockDelta {
                            index: candidate.index.unwrap_or(0) as u32,
                            delta: TextDelta {
                                type_field: "text_delta".to_string(),
                                text,
                            },
                        });
                    }

                    // Handle finish reason
                    if let Some(finish_reason) = &candidate.finish_reason {
                        let stop_reason = match finish_reason.as_str() {
                            "STOP" => Some("end_turn".to_string()),
                            "MAX_TOKENS" => Some("max_tokens".to_string()),
                            "SAFETY" => Some("stop_sequence".to_string()),
                            "RECITATION" => Some("stop_sequence".to_string()),
                            "OTHER" => Some("stop_sequence".to_string()),
                            _ => Some("stop_sequence".to_string()),
                        };

                        events.push(AnthropicStreamEvent::MessageDelta {
                            delta: MessageDelta {
                                stop_reason,
                                usage: self.usage_metadata.as_ref().map(|usage| Usage {
                                    input_tokens: usage.prompt_token_count.unwrap_or(0),
                                    output_tokens: usage.candidates_token_count.unwrap_or(0),
                                }),
                            },
                        });

                        events.push(AnthropicStreamEvent::MessageStop);
                    }
                }
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
        if let Some(_candidates) = &self.candidates {
            // Note: streaming candidates don't have safety_ratings field
            // This is a simplified check for streaming issues
        }
        false
    }
}