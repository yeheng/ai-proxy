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
            "{}/models?key={}",
            self.config
                .api_base
                .trim_end_matches('/')
                .replace("/v1beta/models", "/v1beta"),
            self.config.api_key
        );

        tracing::info!("Fetching models from URL: {}", url);

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
            tracing::warn!("Gemini models API error: status={}, body={}", status, error_body);
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
            "{}/models/{}:generateContent?key={}",
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

    async fn chat_stream(&self, request: AnthropicRequest) -> Result<StreamResponse, AppError> {
        use futures::StreamExt;
        
        // Validate request
        request.validate().map_err(AppError::ValidationError)?;

        // Convert to Gemini format
        let gemini_req = self.convert_request(&request)?;

        // Build streaming URL
        let url = format!(
            "{}/models/{}:streamGenerateContent?key={}",
            self.config.api_base.trim_end_matches('/'),
            request.model,
            self.config.api_key
        );

        tracing::info!("Starting Gemini streaming request to: {}", url);

        // Send streaming request
        let response = self
            .client
            .post(&url)
            .json(&gemini_req)
            .send()
            .await
            .map_err(|e| AppError::ProviderError {
                status: 500,
                message: format!("Failed to send streaming request to Gemini: {}", e),
            })?;

        // Check for HTTP errors
        if !response.status().is_success() {
            let status = response.status().as_u16();
            let error_body = response.text().await.unwrap_or_default();
            return Err(AppError::ProviderError {
                status,
                message: format!("Gemini streaming API error: {}", error_body),
            });
        }

        // Get the response body as a stream
        let body = response.bytes_stream();
        
        // Generate unique message ID for this streaming session
        let message_id = format!("msg_{}", uuid::Uuid::new_v4().simple());
        let model_name = request.model.clone();
        
        // Process streaming bytes and convert to SSE events
        let sse_stream = body
            .enumerate()
            .filter_map(move |(chunk_index, chunk_result)| {
                let message_id = message_id.clone();
                let model_name = model_name.clone();
                
                async move {
                    match chunk_result {
                        Ok(bytes) => {
                            // Convert bytes to string
                            let chunk_str = String::from_utf8_lossy(&bytes);
                            
                            // Process complete lines from chunk
                            let mut sse_events = Vec::new();
                            let lines: Vec<&str> = chunk_str.lines().collect();
                            
                            for (line_index, line) in lines.iter().enumerate() {
                                // Skip empty lines
                                if line.trim().is_empty() {
                                    continue;
                                }

                                // Parse JSON line from Gemini streaming response
                                match serde_json::from_str::<GeminiStreamResponse>(line) {
                                    Ok(gemini_stream) => {
                                        // Add message start event if this is the first chunk
                                        if chunk_index == 0 && line_index == 0 {
                                            let start_event = GeminiStreamResponse::create_message_start_event(&model_name, &message_id);
                                            if let Ok(start_json) = serde_json::to_string(&start_event) {
                                                sse_events.push(format!("event: message_start\ndata: {}\n\n", start_json));
                                            }
                                            
                                            // Add content block start event
                                            let content_start_event = GeminiStreamResponse::create_content_block_start_event();
                                            if let Ok(content_json) = serde_json::to_string(&content_start_event) {
                                                sse_events.push(format!("event: content_block_start\ndata: {}\n\n", content_json));
                                            }
                                        }
                                        
                                        // Convert to Anthropic streaming events
                                        match gemini_stream.to_anthropic_events(&model_name, &message_id) {
                                            Ok(events) => {
                                                // Convert each event to SSE format
                                                for event in events {
                                                    match event {
                                                        AnthropicStreamEvent::ContentBlockDelta { .. } => {
                                                            if let Ok(json) = serde_json::to_string(&event) {
                                                                sse_events.push(format!("event: content_block_delta\ndata: {}\n\n", json));
                                                            }
                                                        }
                                                        AnthropicStreamEvent::MessageDelta { .. } => {
                                                            if let Ok(json) = serde_json::to_string(&event) {
                                                                sse_events.push(format!("event: message_delta\ndata: {}\n\n", json));
                                                            }
                                                        }
                                                        AnthropicStreamEvent::MessageStop => {
                                                            // Add content block stop first
                                                            let content_stop = AnthropicStreamEvent::ContentBlockStop { index: 0 };
                                                            if let Ok(json) = serde_json::to_string(&content_stop) {
                                                                sse_events.push(format!("event: content_block_stop\ndata: {}\n\n", json));
                                                            }
                                                            
                                                            // Then add message stop
                                                            if let Ok(json) = serde_json::to_string(&event) {
                                                                sse_events.push(format!("event: message_stop\ndata: {}\n\n", json));
                                                            }
                                                        }
                                                        _ => {
                                                            if let Ok(json) = serde_json::to_string(&event) {
                                                                sse_events.push(format!("data: {}\n\n", json));
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                            Err(e) => {
                                                tracing::error!("Failed to convert Gemini stream to Anthropic events: {}", e);
                                                let error_event = GeminiStreamResponse::create_error_event(&e);
                                                if let Ok(json) = serde_json::to_string(&error_event) {
                                                    sse_events.push(format!("event: error\ndata: {}\n\n", json));
                                                }
                                            }
                                        }
                                    }
                                    Err(parse_err) => {
                                        tracing::warn!("Failed to parse Gemini streaming response line: {} - Error: {}", line, parse_err);
                                        // Skip malformed lines but continue streaming
                                    }
                                }
                            }
                            
                            if !sse_events.is_empty() {
                                Some(Ok(sse_events.join("")))
                            } else {
                                None
                            }
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

        tracing::info!("Gemini streaming response initialized successfully");
        Ok(Box::pin(sse_stream))
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
            "{}/models?key={}",
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
