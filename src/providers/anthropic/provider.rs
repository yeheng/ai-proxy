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
    /// 创建新的Anthropic提供商实例
    ///
    /// ## 功能说明
    /// 使用给定的配置和HTTP客户端创建Anthropic提供商实例。
    /// 由于系统的标准格式基于Anthropic API，此提供商需要最少的格式转换。
    ///
    /// ## 参数说明
    /// - `config`: Anthropic提供商的详细配置，包含API密钥、基础URL等
    /// - `client`: 共享的HTTP客户端，用于发送API请求
    ///
    /// ## 执行例子
    /// ```rust
    /// let config = ProviderDetail {
    ///     api_key: "sk-ant-...".to_string(),
    ///     api_base: "https://api.anthropic.com/v1/".to_string(),
    ///     // ... 其他配置
    /// };
    /// let client = Client::new();
    /// let provider = AnthropicProvider::new(config, client);
    /// ```
    pub fn new(config: ProviderDetail, client: Client) -> Self {
        Self { config, client }
    }

    /// Validate model name for Anthropic
    fn validate_model_name(&self, model: &str) -> Result<(), AppError> {
        // Check if model name starts with "claude-"
        if !model.starts_with("claude-") {
            return Err(AppError::ValidationError(format!(
                "Invalid Anthropic model name '{}': must start with 'claude-'",
                model
            )));
        }

        // Check for known Anthropic model patterns
        let valid_patterns = [
            "claude-3-5-sonnet",
            "claude-3-5-haiku", 
            "claude-3-opus",
            "claude-3-sonnet",
            "claude-3-haiku",
            "claude-2.1",
            "claude-2.0",
            "claude-instant",
        ];

        let is_valid = valid_patterns.iter().any(|pattern| model.starts_with(pattern));
        
        if !is_valid {
            return Err(AppError::ValidationError(format!(
                "Unsupported Anthropic model: {}",
                model
            )));
        }

        Ok(())
    }

    /// Handle Anthropic API errors with proper error parsing
    fn handle_api_error(&self, status: u16, error_body: &str) -> AppError {
        // Try to parse Anthropic error format
        let parsed_message = if let Ok(error_json) = serde_json::from_str::<serde_json::Value>(error_body) {
            error_json
                .get("error")
                .and_then(|e| e.get("message"))
                .and_then(|m| m.as_str())
                .unwrap_or(error_body)
                .to_string()
        } else {
            error_body.to_string()
        };
        
        match status {
            400 => AppError::BadRequest(format!("Anthropic API: {}", parsed_message)),
            401 => AppError::ProviderError {
                status,
                message: "Anthropic API: Invalid API key or authentication failed".to_string(),
            },
            403 => AppError::ProviderError {
                status,
                message: "Anthropic API: Access forbidden - check your API key permissions".to_string(),
            },
            404 => AppError::ProviderError {
                status,
                message: "Anthropic API: Model not found or endpoint not available".to_string(),
            },
            429 => AppError::ProviderError {
                status,
                message: format!("Anthropic API: Rate limit exceeded - {}", parsed_message),
            },
            500..=599 => AppError::ProviderError {
                status,
                message: format!("Anthropic API: Server error - {}", parsed_message),
            },
            _ => AppError::ProviderError {
                status,
                message: format!("Anthropic API: Unexpected error - {}", parsed_message),
            },
        }
    }

    /// Fetch models from Anthropic API (currently returns configured models as Anthropic doesn't have a public models endpoint)
    async fn fetch_models_from_api(&self) -> Result<Vec<ModelInfo>, AppError> {
        // Note: Anthropic doesn't currently provide a public models endpoint
        // So we return the configured models or default models
        let models = self
            .config
            .models
            .as_ref()
            .map(|m| m.clone())
            .unwrap_or_else(|| {
                vec![
                    "claude-3-5-sonnet-20241022".to_string(),
                    "claude-3-5-haiku-20241022".to_string(),
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

    /// Get fallback models when API is unavailable
    fn get_fallback_models(&self) -> Result<Vec<ModelInfo>, AppError> {
        let models = self
            .config
            .models
            .as_ref()
            .map(|m| m.clone())
            .unwrap_or_else(|| {
                vec![
                    "claude-3-5-sonnet-20241022".to_string(),
                    "claude-3-5-haiku-20241022".to_string(),
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
                created: 1714560000, // Static timestamp for fallback
                owned_by: "anthropic".to_string(),
            })
            .collect())
    }

    /// Perform comprehensive health check with retry logic
    async fn perform_comprehensive_health_check(&self) -> Result<(), AppError> {
        // First attempt: lightweight connectivity check
        let connectivity_result = self.check_connectivity().await;
        
        match connectivity_result {
            Ok(()) => {
                // If connectivity is good, perform a more thorough check
                self.check_api_functionality().await
            }
            Err(e) => {
                // If connectivity fails, try one more time with a shorter timeout
                tracing::warn!("Initial connectivity check failed: {}, retrying...", e);
                tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                self.check_connectivity().await
            }
        }
    }

    /// Check basic connectivity to Anthropic API
    async fn check_connectivity(&self) -> Result<(), AppError> {
        let url = format!("{}messages", self.config.api_base.trim_end_matches('/'));
        
        // Create a minimal request just to test connectivity
        let test_request = AnthropicRequest {
            model: "claude-3-haiku-20240307".to_string(),
            messages: vec![Message {
                role: "user".to_string(),
                content: "test".to_string(),
            }],
            max_tokens: 1,
            stream: Some(false),
            temperature: None,
            top_p: None,
        };

        let response = self
            .client
            .post(&url)
            .header("x-api-key", &self.config.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("Content-Type", "application/json")
            .header("User-Agent", "ai-proxy/0.1.0")
            .json(&test_request)
            .timeout(std::time::Duration::from_secs(10))
            .send()
            .await
            .map_err(|e| AppError::ProviderError {
                status: 500,
                message: format!("Failed to connect to Anthropic: {}", e),
            })?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let error_body = response.text().await.unwrap_or_default();
            return Err(self.handle_api_error(status, &error_body));
        }

        Ok(())
    }

    /// Check API functionality with a more comprehensive test
    async fn check_api_functionality(&self) -> Result<(), AppError> {
        let url = format!("{}messages", self.config.api_base.trim_end_matches('/'));
        
        let test_request = AnthropicRequest {
            model: "claude-3-haiku-20240307".to_string(),
            messages: vec![Message {
                role: "user".to_string(),
                content: "Hi".to_string(),
            }],
            max_tokens: 1,
            stream: Some(false),
            temperature: None,
            top_p: None,
        };

        let response = self
            .client
            .post(&url)
            .header("x-api-key", &self.config.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("Content-Type", "application/json")
            .header("User-Agent", "ai-proxy/0.1.0")
            .json(&test_request)
            .timeout(std::time::Duration::from_secs(30))
            .send()
            .await
            .map_err(|e| AppError::ProviderError {
                status: 500,
                message: format!("Failed to connect to Anthropic: {}", e),
            })?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let error_body = response.text().await.unwrap_or_default();
            return Err(self.handle_api_error(status, &error_body));
        }

        // Verify we can parse the response
        let response_data: AnthropicResponse = response
            .json()
            .await
            .map_err(|e| AppError::ProviderError {
                status: 500,
                message: format!("Failed to parse Anthropic response: {}", e),
            })?;

        // Validate response structure
        if response_data.content.is_empty() {
            return Err(AppError::ProviderError {
                status: 500,
                message: "Anthropic returned empty response content".to_string(),
            });
        }

        Ok(())
    }
}

#[async_trait]
impl AIProvider for AnthropicProvider {
    async fn chat(&self, request: AnthropicRequest) -> Result<AnthropicResponse, AppError> {
        // Validate request
        request.validate().map_err(AppError::ValidationError)?;

        // Validate model name for Anthropic
        self.validate_model_name(&request.model)?;

        // Build URL
        let url = format!("{}messages", self.config.api_base.trim_end_matches('/'));

        tracing::info!("Sending Anthropic chat request to: {} with model: {}", url, request.model);

        // Send request (minimal conversion needed since we use Anthropic format)
        let response = self
            .client
            .post(&url)
            .header("x-api-key", &self.config.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("Content-Type", "application/json")
            .header("User-Agent", "ai-proxy/0.1.0")
            .json(&request)
            .send()
            .await
            .map_err(|e| AppError::ProviderError {
                status: 500,
                message: format!("Failed to send request to Anthropic: {}", e),
            })?;

        // Handle HTTP errors with proper error parsing
        if !response.status().is_success() {
            let status = response.status().as_u16();
            let error_body = response.text().await.unwrap_or_default();
            tracing::warn!("Anthropic API error: status={}, body={}", status, error_body);
            return Err(self.handle_api_error(status, &error_body));
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

        // Validate response has content
        if anthropic_res.content.is_empty() {
            return Err(AppError::ProviderError {
                status: 500,
                message: "Anthropic returned empty response".to_string(),
            });
        }

        tracing::info!("Anthropic chat completed successfully: input_tokens={}, output_tokens={}", 
                      anthropic_res.usage.input_tokens, anthropic_res.usage.output_tokens);

        Ok(anthropic_res)
    }

    async fn chat_stream(&self, request: AnthropicRequest) -> Result<StreamResponse, AppError> {
        use futures::StreamExt;
        
        // Validate request
        request.validate().map_err(AppError::ValidationError)?;

        // Validate model name for Anthropic
        self.validate_model_name(&request.model)?;

        // Create streaming request with stream enabled
        let mut streaming_request = request.clone();
        streaming_request.stream = Some(true);

        // Build streaming URL
        let url = format!("{}messages", self.config.api_base.trim_end_matches('/'));

        tracing::info!("Starting Anthropic streaming request to: {} with model: {}", url, request.model);

        // Send streaming request
        let response = self
            .client
            .post(&url)
            .header("x-api-key", &self.config.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("Content-Type", "application/json")
            .header("User-Agent", "ai-proxy/0.1.0")
            .header("Accept", "text/event-stream")
            .json(&streaming_request)
            .send()
            .await
            .map_err(|e| AppError::ProviderError {
                status: 500,
                message: format!("Failed to send streaming request to Anthropic: {}", e),
            })?;

        // Check for HTTP errors
        if !response.status().is_success() {
            let status = response.status().as_u16();
            let error_body = response.text().await.unwrap_or_default();
            tracing::warn!("Anthropic streaming API error: status={}, body={}", status, error_body);
            return Err(self.handle_api_error(status, &error_body));
        }

        // Get the response body as a stream
        let body = response.bytes_stream();
        
        // Process streaming bytes and convert to SSE events
        // Since Anthropic already returns SSE format, we can forward it directly
        let sse_stream = body
            .filter_map(move |chunk_result| {
                async move {
                    match chunk_result {
                        Ok(bytes) => {
                            // Convert bytes to string
                            let chunk_str = String::from_utf8_lossy(&bytes);
                            
                            // Anthropic returns proper SSE format, so we can forward directly
                            // But we need to validate and potentially filter the content
                            if chunk_str.trim().is_empty() {
                                return None;
                            }

                            // Forward the SSE chunk as-is since Anthropic uses the standard format
                            Some(Ok(chunk_str.to_string()))
                        }
                        Err(e) => {
                            tracing::error!("Error reading streaming response chunk: {}", e);
                            let app_error = AppError::ProviderError {
                                status: 500,
                                message: format!("Streaming read error: {}", e),
                            };
                            Some(Err(app_error))
                        }
                    }
                }
            });

        tracing::info!("Anthropic streaming response initialized successfully");
        Ok(Box::pin(sse_stream))
    }

    async fn list_models(&self) -> Result<Vec<ModelInfo>, AppError> {
        // Use the fetch_models_from_api method for consistency
        match self.fetch_models_from_api().await {
            Ok(mut models) => {
                tracing::info!("Successfully retrieved {} Anthropic models", models.len());
                
                // Sort models by name for consistent ordering
                models.sort_by(|a, b| a.id.cmp(&b.id));
                
                // Filter to ensure we only return valid Claude models
                let claude_models: Vec<ModelInfo> = models
                    .into_iter()
                    .filter(|model| {
                        let id = &model.id;
                        id.starts_with("claude-") && !id.contains("deprecated")
                    })
                    .collect();
                
                if claude_models.is_empty() {
                    tracing::warn!("No valid Claude models found, returning default models");
                    return self.get_fallback_models();
                }
                
                Ok(claude_models)
            }
            Err(e) => {
                tracing::warn!("Failed to fetch models from Anthropic: {}, falling back to configured models", e);
                self.get_fallback_models()
            }
        }
    }

    async fn health_check(&self) -> Result<HealthStatus, AppError> {
        let start = std::time::Instant::now();

        // Comprehensive health check with multiple validation steps
        let health_result = self.perform_comprehensive_health_check().await;
        let latency = start.elapsed().as_millis() as u64;

        match health_result {
            Ok(()) => Ok(HealthStatus {
                status: "healthy".to_string(),
                provider: "anthropic".to_string(),
                latency_ms: Some(latency),
                error: None,
            }),
            Err(e) => {
                let (status, error_msg) = match &e {
                    AppError::ProviderError { status, message } => {
                        match *status {
                            401 => ("unhealthy".to_string(), "Authentication failed - check API key".to_string()),
                            403 => ("unhealthy".to_string(), "Access forbidden - check API key permissions".to_string()),
                            429 => ("degraded".to_string(), "Rate limited - service may be slow".to_string()),
                            500..=599 => ("unhealthy".to_string(), format!("Anthropic server error: {}", message)),
                            _ => ("unhealthy".to_string(), message.clone()),
                        }
                    }
                    _ => ("unhealthy".to_string(), e.to_string()),
                };

                Ok(HealthStatus {
                    status,
                    provider: "anthropic".to_string(),
                    latency_ms: Some(latency),
                    error: Some(error_msg),
                })
            }
        }
    }
}
