// Anthropic Provider Implementation
use async_trait::async_trait;
use reqwest::Client;

use crate::{
    config::ProviderDetail,
    errors::AppError,
    providers::{
        AIProvider, HealthStatus, ModelInfo, StreamResponse,
        anthropic::{AnthropicRequest, AnthropicResponse, Message},
    },
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
        let response = self
            .client
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
        let anthropic_res =
            response
                .json::<AnthropicResponse>()
                .await
                .map_err(|e| AppError::ProviderError {
                    status: 500,
                    message: format!("Failed to parse Anthropic response: {}", e),
                })?;

        Ok(anthropic_res)
    }

    async fn chat_stream(&self, _request: AnthropicRequest) -> Result<StreamResponse, AppError> {
        // TODO: Implement streaming support
        Err(AppError::InternalServerError(
            "Streaming not yet implemented for Anthropic provider".to_string(),
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
                    "claude-3-opus-20240229".to_string(),
                    "claude-3-sonnet-20240229".to_string(),
                    "claude-3-haiku-20240307".to_string(),
                ]
            });

        Ok(models
            .into_iter()
            .map(|model| ModelInfo {
                id: model,
                object: "model".to_string(),
                created: 1714560000, // Static timestamp for now
                owned_by: "anthropic".to_string(),
            })
            .collect())
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

        let result = self
            .client
            .post(&url)
            .header("x-api-key", &self.config.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("Content-Type", "application/json")
            .json(&test_request)
            .send()
            .await;

        let latency = start.elapsed().as_millis() as u64;

        match result {
            Ok(response) if response.status().is_success() => Ok(HealthStatus {
                status: "healthy".to_string(),
                provider: "anthropic".to_string(),
                latency_ms: Some(latency),
                error: None,
            }),
            Ok(response) => Ok(HealthStatus {
                status: "unhealthy".to_string(),
                provider: "anthropic".to_string(),
                latency_ms: Some(latency),
                error: Some(format!("HTTP {}", response.status())),
            }),
            Err(e) => Ok(HealthStatus {
                status: "unhealthy".to_string(),
                provider: "anthropic".to_string(),
                latency_ms: Some(latency),
                error: Some(e.to_string()),
            }),
        }
    }
}
