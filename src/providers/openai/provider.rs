use async_trait::async_trait;
use reqwest::Client;

use crate::{
    config::ProviderDetail,
    errors::AppError,
    providers::{AIProvider, HealthStatus, ModelInfo, StreamResponse, anthropic::*, openai::*},
};

/// OpenAI provider implementation
pub struct OpenAIProvider {
    config: ProviderDetail,
    client: Client,
}

impl OpenAIProvider {
    pub fn new(config: ProviderDetail, client: Client) -> Self {
        Self { config, client }
    }
}

impl OpenAIProvider {
    /// Convert Anthropic request format to OpenAI format
    fn convert_request(&self, request: &AnthropicRequest) -> OpenAIRequest {
        let messages = request
            .messages
            .iter()
            .map(|msg| OpenAIMessage {
                role: msg.role.clone(),
                content: msg.content.clone(),
            })
            .collect();

        OpenAIRequest {
            model: request.model.clone(),
            messages,
            max_tokens: request.max_tokens,
            stream: request.stream,
            temperature: request.temperature,
            top_p: request.top_p,
        }
    }

    /// Convert OpenAI response format to Anthropic format
    fn convert_response(&self, openai_res: OpenAIResponse) -> Result<AnthropicResponse, AppError> {
        let choice =
            openai_res
                .choices
                .into_iter()
                .next()
                .ok_or_else(|| AppError::ProviderError {
                    status: 500,
                    message: "No choices in OpenAI response".to_string(),
                })?;

        Ok(AnthropicResponse::new(
            openai_res.id,
            openai_res.model,
            choice.message.content,
            openai_res.usage.prompt_tokens,
            openai_res.usage.completion_tokens,
        ))
    }
}

#[async_trait]
impl AIProvider for OpenAIProvider {
    async fn chat(&self, request: AnthropicRequest) -> Result<AnthropicResponse, AppError> {
        // Validate request
        request.validate().map_err(AppError::ValidationError)?;

        // Convert to OpenAI format
        let openai_req = self.convert_request(&request);

        // Build URL
        let url = format!("{}chat/completions", self.config.api_base);

        // Send request
        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.config.api_key))
            .header("Content-Type", "application/json")
            .json(&openai_req)
            .send()
            .await
            .map_err(|e| AppError::ProviderError {
                status: 500,
                message: format!("Failed to send request to OpenAI: {}", e),
            })?;

        // Handle HTTP errors
        if !response.status().is_success() {
            let status = response.status().as_u16();
            let error_body = response.text().await.unwrap_or_default();
            return Err(AppError::ProviderError {
                status,
                message: format!("OpenAI API error: {}", error_body),
            });
        }

        // Parse response
        let openai_res =
            response
                .json::<OpenAIResponse>()
                .await
                .map_err(|e| AppError::ProviderError {
                    status: 500,
                    message: format!("Failed to parse OpenAI response: {}", e),
                })?;

        // Convert to standard format
        self.convert_response(openai_res)
    }

    async fn chat_stream(&self, _request: AnthropicRequest) -> Result<StreamResponse, AppError> {
        // TODO: Implement streaming support
        Err(AppError::InternalServerError(
            "Streaming not yet implemented for OpenAI provider".to_string(),
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
                    "gpt-4".to_string(),
                    "gpt-4-turbo-preview".to_string(),
                    "gpt-3.5-turbo".to_string(),
                    "gpt-3.5-turbo-16k".to_string(),
                ]
            });

        Ok(models
            .into_iter()
            .map(|model| ModelInfo {
                id: model,
                object: "model".to_string(),
                created: 1714560000, // Static timestamp for now
                owned_by: "openai".to_string(),
            })
            .collect())
    }

    async fn health_check(&self) -> Result<HealthStatus, AppError> {
        let start = std::time::Instant::now();

        // Simple health check by trying to list models
        let url = format!("{}models", self.config.api_base);

        let result = self
            .client
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.config.api_key))
            .send()
            .await;

        let latency = start.elapsed().as_millis() as u64;

        match result {
            Ok(response) if response.status().is_success() => Ok(HealthStatus {
                status: "healthy".to_string(),
                provider: "openai".to_string(),
                latency_ms: Some(latency),
                error: None,
            }),
            Ok(response) => Ok(HealthStatus {
                status: "unhealthy".to_string(),
                provider: "openai".to_string(),
                latency_ms: Some(latency),
                error: Some(format!("HTTP {}", response.status())),
            }),
            Err(e) => Ok(HealthStatus {
                status: "unhealthy".to_string(),
                provider: "openai".to_string(),
                latency_ms: Some(latency),
                error: Some(e.to_string()),
            }),
        }
    }
}
