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
    /// 创建新的OpenAI提供商实例
    ///
    /// ## 功能说明
    /// 使用给定的配置和HTTP客户端创建OpenAI提供商实例
    ///
    /// ## 参数说明
    /// - `config`: OpenAI提供商的详细配置，包含API密钥、基础URL等
    /// - `client`: 共享的HTTP客户端，用于发送API请求
    ///
    /// ## 执行例子
    /// ```rust
    /// let config = ProviderDetail {
    ///     api_key: "sk-...".to_string(),
    ///     api_base: "https://api.openai.com/v1/".to_string(),
    ///     // ... 其他配置
    /// };
    /// let client = Client::new();
    /// let provider = OpenAIProvider::new(config, client);
    /// ```
    pub fn new(config: ProviderDetail, client: Client) -> Self {
        Self { config, client }
    }

    /// Fetch models from OpenAI API
    async fn fetch_models_from_api(&self) -> Result<Vec<ModelInfo>, AppError> {
        let url = format!("{}models", self.config.api_base);

        let response = self
            .client
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.config.api_key))
            .send()
            .await
            .map_err(|e| AppError::ProviderError {
                status: 500,
                message: format!("Failed to fetch models from OpenAI: {}", e),
            })?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let error_body = response.text().await.unwrap_or_default();
            return Err(AppError::ProviderError {
                status,
                message: format!("OpenAI models API error: {}", error_body),
            });
        }

        let models_response: serde_json::Value = response
            .json()
            .await
            .map_err(|e| AppError::ProviderError {
                status: 500,
                message: format!("Failed to parse OpenAI models response: {}", e),
            })?;

        // Parse the models from OpenAI's response format
        let models = models_response
            .get("data")
            .and_then(|data| data.as_array())
            .ok_or_else(|| AppError::ProviderError {
                status: 500,
                message: "Invalid models response format from OpenAI".to_string(),
            })?
            .iter()
            .filter_map(|model| {
                let id = model.get("id")?.as_str()?.to_string();
                let object = model.get("object")?.as_str().unwrap_or("model").to_string();
                let created = model.get("created")?.as_u64().unwrap_or(1714560000);
                let owned_by = model.get("owned_by")?.as_str().unwrap_or("openai").to_string();

                Some(ModelInfo {
                    id,
                    object,
                    created,
                    owned_by,
                })
            })
            .collect();

        Ok(models)
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
        // Try to fetch models from OpenAI API first
        match self.fetch_models_from_api().await {
            Ok(models) => {
                tracing::info!("Successfully fetched {} models from OpenAI API", models.len());
                Ok(models)
            }
            Err(e) => {
                tracing::warn!("Failed to fetch models from OpenAI API: {}, falling back to configured models", e);
                // Fall back to configured models
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
        }
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
