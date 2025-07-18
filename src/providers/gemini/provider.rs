use async_trait::async_trait;
use reqwest::Client;

use crate::{
    config::ProviderDetail,
    errors::AppError,
    providers::{AIProvider, HealthStatus, ModelInfo, StreamResponse, anthropic::*, gemini::*},
};

/// Google Gemini provider implementation
pub struct GeminiProvider {
    config: ProviderDetail,
    client: Client,
}

impl GeminiProvider {
    pub fn new(config: ProviderDetail, client: Client) -> Self {
        Self { config, client }
    }
}

impl GeminiProvider {
    /// Convert Anthropic request format to Gemini format
    fn convert_request(&self, request: &AnthropicRequest) -> Result<GeminiRequest, AppError> {
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
            },
        })
    }

    /// Convert Gemini response format to Anthropic format
    fn convert_response(
        &self,
        gemini_res: GeminiResponse,
        model: &str,
    ) -> Result<AnthropicResponse, AppError> {
        let candidate =
            gemini_res
                .candidates
                .into_iter()
                .next()
                .ok_or_else(|| AppError::ProviderError {
                    status: 500,
                    message: "No candidates in Gemini response".to_string(),
                })?;

        let text = candidate
            .content
            .parts
            .into_iter()
            .map(|part| part.text)
            .collect::<Vec<_>>()
            .join("");

        let usage = gemini_res.usage_metadata.unwrap_or(UsageMetadata {
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

#[async_trait]
impl AIProvider for GeminiProvider {
    async fn chat(&self, request: AnthropicRequest) -> Result<AnthropicResponse, AppError> {
        // Validate request
        request.validate().map_err(AppError::ValidationError)?;

        // Convert to Gemini format
        let gemini_req = self.convert_request(&request)?;

        // Build URL
        let url = format!(
            "{}{}:generateContent?key={}",
            self.config.api_base, request.model, self.config.api_key
        );

        // Send request
        let response = self
            .client
            .post(&url)
            .json(&gemini_req)
            .send()
            .await
            .map_err(|e| AppError::ProviderError {
                status: 500,
                message: format!("Failed to send request to Gemini: {}", e),
            })?;

        // Handle HTTP errors
        if !response.status().is_success() {
            let status = response.status().as_u16();
            let error_body = response.text().await.unwrap_or_default();
            return Err(AppError::ProviderError {
                status,
                message: format!("Gemini API error: {}", error_body),
            });
        }

        // Parse response
        let gemini_res =
            response
                .json::<GeminiResponse>()
                .await
                .map_err(|e| AppError::ProviderError {
                    status: 500,
                    message: format!("Failed to parse Gemini response: {}", e),
                })?;

        // Convert to standard format
        self.convert_response(gemini_res, &request.model)
    }

    async fn chat_stream(&self, _request: AnthropicRequest) -> Result<StreamResponse, AppError> {
        // TODO: Implement streaming support
        Err(AppError::InternalServerError(
            "Streaming not yet implemented for Gemini provider".to_string(),
        ))
    }

    async fn list_models(&self) -> Result<Vec<ModelInfo>, AppError> {
        let models = self
            .config
            .models
            .as_ref()
            .map(|m| m.clone())
            .unwrap_or_else(|| {
                vec![
                    "gemini-1.5-pro-latest".to_string(),
                    "gemini-1.5-flash-latest".to_string(),
                    "gemini-pro".to_string(),
                ]
            });

        Ok(models
            .into_iter()
            .map(|model| ModelInfo {
                id: model,
                object: "model".to_string(),
                created: 1714560000, // Static timestamp for now
                owned_by: "google".to_string(),
            })
            .collect())
    }

    async fn health_check(&self) -> Result<HealthStatus, AppError> {
        let start = std::time::Instant::now();

        // Simple health check by trying to list models
        let url = format!(
            "{}models?key={}",
            self.config
                .api_base
                .trim_end_matches('/')
                .replace("/v1beta/models", "/v1beta"),
            self.config.api_key
        );

        let result = self.client.get(&url).send().await;

        let latency = start.elapsed().as_millis() as u64;

        match result {
            Ok(response) if response.status().is_success() => Ok(HealthStatus {
                status: "healthy".to_string(),
                provider: "gemini".to_string(),
                latency_ms: Some(latency),
                error: None,
            }),
            Ok(response) => Ok(HealthStatus {
                status: "unhealthy".to_string(),
                provider: "gemini".to_string(),
                latency_ms: Some(latency),
                error: Some(format!("HTTP {}", response.status())),
            }),
            Err(e) => Ok(HealthStatus {
                status: "unhealthy".to_string(),
                provider: "gemini".to_string(),
                latency_ms: Some(latency),
                error: Some(e.to_string()),
            }),
        }
    }
}
