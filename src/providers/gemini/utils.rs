use std::collections::HashMap;

use crate::{AppError, providers::gemini::model::*};

/// Create a simple Gemini request from text content
pub fn create_simple_request(content: String, max_tokens: u32) -> GeminiRequest {
    let gemini_content = GeminiContent {
        role: "user".to_string(),
        parts: vec![GeminiPart { text: content }],
    };

    GeminiRequest::new(vec![gemini_content], max_tokens)
}

/// Create a conversation request from multiple messages
pub fn create_conversation_request(
    messages: Vec<(String, String)>,
    max_tokens: u32,
) -> Result<GeminiRequest, AppError> {
    let contents = messages
        .into_iter()
        .map(|(role, content)| {
            let gemini_role = match role.as_str() {
                "user" => "user",
                "assistant" | "model" => "model",
                _ => {
                    return Err(AppError::ValidationError(format!(
                        "Invalid role: {}. Use 'user' or 'assistant'",
                        role
                    )));
                }
            };

            Ok(GeminiContent {
                role: gemini_role.to_string(),
                parts: vec![GeminiPart { text: content }],
            })
        })
        .collect::<Result<Vec<_>, AppError>>()?;

    Ok(GeminiRequest::new(contents, max_tokens))
}

/// Extract text content from Gemini response
pub fn extract_text_content(response: &GeminiResponse) -> Result<String, AppError> {
    // Validate response structure
    if response.error.is_some() {
        return Err(AppError::ProviderError {
            status: 500,
            message: "Response contains API error".to_string(),
        });
    }

    if response.candidates.is_empty() {
        return Err(AppError::ProviderError {
            status: 500,
            message: "No candidates in response".to_string(),
        });
    }

    let candidate = response.candidates.first().unwrap();
    if candidate.content.parts.is_empty() {
        return Err(AppError::ProviderError {
            status: 500,
            message: "Candidate has no content parts".to_string(),
        });
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
            message: "Empty text content in response".to_string(),
        });
    }

    Ok(text)
}

/// Parse safety settings from string configuration
pub fn parse_safety_settings(config: &[(&str, &str)]) -> Result<Vec<SafetySetting>, AppError> {
    config
        .iter()
        .map(|(category_str, threshold_str)| {
            let category = match *category_str {
                "harassment" => HarmCategory::Harassment,
                "hate_speech" => HarmCategory::HateSpeech,
                "sexually_explicit" => HarmCategory::SexuallyExplicit,
                "dangerous_content" => HarmCategory::DangerousContent,
                _ => {
                    return Err(AppError::ValidationError(format!(
                        "Invalid harm category: {}",
                        category_str
                    )));
                }
            };

            let threshold = match *threshold_str {
                "block_low_and_above" => HarmBlockThreshold::BlockLowAndAbove,
                "block_medium_and_above" => HarmBlockThreshold::BlockMediumAndAbove,
                "block_only_high" => HarmBlockThreshold::BlockOnlyHigh,
                "block_none" => HarmBlockThreshold::BlockNone,
                _ => {
                    return Err(AppError::ValidationError(format!(
                        "Invalid harm block threshold: {}",
                        threshold_str
                    )));
                }
            };

            Ok(SafetySetting {
                category,
                threshold,
            })
        })
        .collect()
}

/// Create a tool definition for simple function calling
pub fn create_simple_tool(name: String, description: String, parameters: Option<Schema>) -> Tool {
    Tool {
        function_declarations: vec![FunctionDeclaration {
            name,
            description,
            parameters,
        }],
    }
}

/// Create a schema for function parameters
pub fn create_schema(
    type_field: String,
    properties: Option<HashMap<String, SchemaProperty>>,
    required: Option<Vec<String>>,
) -> Schema {
    Schema {
        type_field,
        properties,
        required,
    }
}

/// Create a schema property
pub fn create_schema_property(type_field: String, description: Option<String>) -> SchemaProperty {
    SchemaProperty {
        type_field,
        description,
        items: None,
        enum_values: None,
    }
}

/// Validate Gemini response structure
pub fn validate_response_structure(response: &GeminiResponse) -> Result<(), AppError> {
    if response.error.is_some() {
        return Err(AppError::ProviderError {
            status: 500,
            message: "Response contains API error".to_string(),
        });
    }

    if response.candidates.is_empty() {
        return Err(AppError::ProviderError {
            status: 500,
            message: "No candidates in response".to_string(),
        });
    }

    for (i, candidate) in response.candidates.iter().enumerate() {
        if candidate.content.parts.is_empty() {
            return Err(AppError::ProviderError {
                status: 500,
                message: format!("Candidate {} has no content parts", i),
            });
        }
    }

    Ok(())
}