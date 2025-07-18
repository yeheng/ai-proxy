use serde::{Deserialize, Serialize};
use crate::errors::AppError;
use crate::providers::anthropic::{AnthropicRequest, AnthropicResponse, AnthropicStreamEvent, StreamMessage, ContentBlockStart, TextDelta, MessageDelta, Usage};

// Gemini-specific data structures for API communication

/// Gemini API request structure
#[derive(Serialize, Debug, Deserialize)]
pub struct GeminiRequest {
    pub contents: Vec<GeminiContent>,
    #[serde(rename = "generationConfig")]
    pub generation_config: GenerationConfig,
}

/// Content structure for Gemini messages
#[derive(Serialize, Deserialize, Debug)]
pub struct GeminiContent {
    pub role: String,
    pub parts: Vec<GeminiPart>,
}

/// Part structure containing text content
#[derive(Serialize, Deserialize, Debug)]
pub struct GeminiPart {
    pub text: String,
}

/// Generation configuration for Gemini API
#[derive(Serialize, Debug, Deserialize)]
pub struct GenerationConfig {
    #[serde(rename = "maxOutputTokens")]
    pub max_output_tokens: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "topP")]
    pub top_p: Option<f32>,
}

/// Gemini API response structure
#[derive(Deserialize, Debug)]
pub struct GeminiResponse {
    pub candidates: Vec<GeminiCandidate>,
    #[serde(rename = "usageMetadata")]
    pub usage_metadata: Option<UsageMetadata>,
}

/// Individual candidate in Gemini response
#[derive(Deserialize, Debug)]
pub struct GeminiCandidate {
    pub content: GeminiContent,
    #[serde(rename = "finishReason")]
    pub finish_reason: Option<String>,
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
                            "Invalid role: {}. Gemini supports 'user' and 'assistant' roles only",
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
            },
        })
    }
}

impl GeminiResponse {
    /// Convert Gemini response format to Anthropic format
    pub fn to_anthropic(&self, model: &str) -> Result<AnthropicResponse, AppError> {
        let candidate = self
            .candidates
            .first()
            .ok_or_else(|| AppError::ProviderError {
                status: 500,
                message: "No candidates in Gemini response".to_string(),
            })?;

        let text = candidate
            .content
            .parts
            .iter()
            .map(|part| part.text.as_str())
            .collect::<Vec<_>>()
            .join("");

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
}

impl GeminiStreamResponse {
    /// Convert Gemini streaming response to Anthropic streaming events
    pub fn to_anthropic_events(&self, _model: &str, _message_id: &str) -> Vec<AnthropicStreamEvent> {
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
                            index: candidate.index.unwrap_or(0),
                            delta: TextDelta {
                                type_field: "text_delta".to_string(),
                                text,
                            },
                        });
                    }
                }

                // Handle finish reason
                if let Some(finish_reason) = &candidate.finish_reason {
                    let stop_reason = match finish_reason.as_str() {
                        "STOP" => Some("end_turn".to_string()),
                        "MAX_TOKENS" => Some("max_tokens".to_string()),
                        "SAFETY" => Some("stop_sequence".to_string()),
                        "RECITATION" => Some("stop_sequence".to_string()),
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

        events
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
}