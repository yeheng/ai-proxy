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
    /// 创建新的Gemini提供商实例
    ///
    /// ## 功能说明
    /// 使用给定的配置和HTTP客户端创建Google Gemini提供商实例
    ///
    /// ## 参数说明
    /// - `config`: Gemini提供商的详细配置，包含API密钥、基础URL等
    /// - `client`: 共享的HTTP客户端，用于发送API请求
    ///
    /// ## 执行例子
    /// ```rust
    /// let config = ProviderDetail {
    ///     api_key: "AIza...".to_string(),
    ///     api_base: "https://generativelanguage.googleapis.com/v1beta/".to_string(),
    ///     // ... 其他配置
    /// };
    /// let client = Client::new();
    /// let provider = GeminiProvider::new(config, client);
    /// ```
    pub fn new(config: ProviderDetail, client: Client) -> Self {
        Self { config, client }
    }

    /// Fetch models from Gemini API
    async fn fetch_models_from_api(&self) -> Result<Vec<ModelInfo>, AppError> {
        let url = format!(
            "{}models?key={}",
            self.config
                .api_base
                .trim_end_matches('/')
                .replace("/v1beta/models", "/v1beta"),
            self.config.api_key
        );

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| AppError::ProviderError {
                status: 500,
                message: format!("Failed to fetch models from Gemini: {}", e),
            })?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let error_body = response.text().await.unwrap_or_default();
            return Err(AppError::ProviderError {
                status,
                message: format!("Gemini models API error: {}", error_body),
            });
        }

        let models_response: serde_json::Value = response
            .json()
            .await
            .map_err(|e| AppError::ProviderError {
                status: 500,
                message: format!("Failed to parse Gemini models response: {}", e),
            })?;

        // Parse the models from Gemini's response format
        let models = models_response
            .get("models")
            .and_then(|models| models.as_array())
            .ok_or_else(|| AppError::ProviderError {
                status: 500,
                message: "Invalid models response format from Gemini".to_string(),
            })?
            .iter()
            .filter_map(|model| {
                let name = model.get("name")?.as_str()?;
                // Extract model ID from the full name (e.g., "models/gemini-pro" -> "gemini-pro")
                let id = name.strip_prefix("models/").unwrap_or(name).to_string();

                Some(ModelInfo {
                    id,
                    object: "model".to_string(),
                    created: 1714560000, // Static timestamp for now
                    owned_by: "google".to_string(),
                })
            })
            .collect();

        Ok(models)
    }
}

impl GeminiProvider {
    /// Convert Anthropic request format to Gemini format
    fn convert_request(&self, request: &AnthropicRequest) -> Result<GeminiRequest, AppError> {
        GeminiRequest::from_anthropic(request)
    }

    /// Convert Gemini response format to Anthropic format
    fn convert_response(
        &self,
        gemini_res: GeminiResponse,
        model: &str,
    ) -> Result<AnthropicResponse, AppError> {
        gemini_res.to_anthropic(model)
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
            "{}/{}:generateContent?key={}",
            self.config.api_base.trim_end_matches('/'),
            request.model,
            self.config.api_key
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
                message: format!("Gemini API error: {}", error_body.replace("Gemini API error: ", "")),
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
        // Try to fetch models from Gemini API first
        match self.fetch_models_from_api().await {
            Ok(models) => {
                tracing::info!("Successfully fetched {} models from Gemini API", models.len());
                Ok(models)
            }
            Err(e) => {
                tracing::warn!("Failed to fetch models from Gemini API: {}, falling back to configured models", e);
                // Fall back to configured models
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
        }
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
