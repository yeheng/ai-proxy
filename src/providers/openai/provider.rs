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
        let url = format!("{}/models", self.config.api_base.trim_end_matches('/'));

        tracing::info!("Fetching models from URL: {}", url);

        let response = self
            .client
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.config.api_key))
            .header("User-Agent", "ai-proxy/0.1.0")
            .send()
            .await
            .map_err(|e| AppError::ProviderError {
                status: 500,
                message: format!("Failed to fetch models from OpenAI: {}", e),
            })?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let error_body = response.text().await.unwrap_or_default();
            tracing::warn!("OpenAI models API error: status={}, body={}", status, error_body);
            return Err(AppError::ProviderError {
                status,
                message: format!("OpenAI models API error: {}", openai_utils::parse_error_response(&error_body)),
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

                // Filter out non-chat models
                if id.contains("embedding") || id.contains("whisper") || id.contains("tts") || id.contains("dall-e") {
                    return None;
                }

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
    fn convert_request(&self, request: &AnthropicRequest) -> Result<OpenAIRequest, AppError> {
        OpenAIRequest::from_anthropic(request)
    }

    /// Convert OpenAI response format to Anthropic format
    fn convert_response(&self, openai_res: OpenAIResponse) -> Result<AnthropicResponse, AppError> {
        openai_res.to_anthropic()
    }

    /// Handle OpenAI API errors with proper error parsing
    fn handle_api_error(&self, status: u16, error_body: &str) -> AppError {
        let parsed_message = openai_utils::parse_error_response(error_body);
        
        match status {
            400 => AppError::BadRequest(format!("OpenAI API: {}", parsed_message)),
            401 => AppError::ProviderError {
                status,
                message: "OpenAI API: Invalid API key or authentication failed".to_string(),
            },
            403 => AppError::ProviderError {
                status,
                message: "OpenAI API: Access forbidden - check your API key permissions".to_string(),
            },
            404 => AppError::ProviderError {
                status,
                message: "OpenAI API: Model not found or endpoint not available".to_string(),
            },
            429 => AppError::ProviderError {
                status,
                message: format!("OpenAI API: Rate limit exceeded - {}", parsed_message),
            },
            500..=599 => AppError::ProviderError {
                status,
                message: format!("OpenAI API: Server error - {}", parsed_message),
            },
            _ => AppError::ProviderError {
                status,
                message: format!("OpenAI API: Unexpected error - {}", parsed_message),
            },
        }
    }
}

#[async_trait]
impl AIProvider for OpenAIProvider {
    async fn chat(&self, request: AnthropicRequest) -> Result<AnthropicResponse, AppError> {
        // Validate request
        request.validate().map_err(AppError::ValidationError)?;

        // Validate model name for OpenAI
        openai_utils::validate_model_name(&request.model)?;

        // Convert to OpenAI format
        let mut openai_req = self.convert_request(&request)?;
        
        // Ensure streaming is disabled for non-streaming chat
        openai_req.stream = Some(false);
        
        // Validate the converted request
        openai_req.validate()?;

        // Build URL
        let url = format!("{}/chat/completions", self.config.api_base.trim_end_matches('/'));

        tracing::info!("Sending OpenAI chat request to: {} with model: {}", url, request.model);

        // Send request with proper headers
        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.config.api_key))
            .header("Content-Type", "application/json")
            .header("User-Agent", "ai-proxy/0.1.0")
            .json(&openai_req)
            .send()
            .await
            .map_err(|e| AppError::ProviderError {
                status: 500,
                message: format!("Failed to send request to OpenAI: {}", e),
            })?;

        // Handle HTTP errors with proper error parsing
        if !response.status().is_success() {
            let status = response.status().as_u16();
            let error_body = response.text().await.unwrap_or_default();
            tracing::warn!("OpenAI API error: status={}, body={}", status, error_body);
            return Err(self.handle_api_error(status, &error_body));
        }

        // Parse response
        let openai_res = response
            .json::<OpenAIResponse>()
            .await
            .map_err(|e| AppError::ProviderError {
                status: 500,
                message: format!("Failed to parse OpenAI response: {}", e),
            })?;

        // Check for response issues
        if openai_res.has_issues() {
            return Err(AppError::ProviderError {
                status: 500,
                message: "OpenAI returned empty or invalid response".to_string(),
            });
        }

        tracing::info!("OpenAI chat completed successfully: {}", openai_res.get_usage_info());

        // Convert to standard format
        self.convert_response(openai_res)
    }

    async fn chat_stream(&self, request: AnthropicRequest) -> Result<StreamResponse, AppError> {
        use futures::StreamExt;
        
        // Validate request
        request.validate().map_err(AppError::ValidationError)?;

        // Validate model name for OpenAI
        openai_utils::validate_model_name(&request.model)?;

        // Check if model supports streaming
        if !openai_utils::supports_streaming(&request.model) {
            return Err(AppError::ValidationError(format!(
                "Model {} does not support streaming",
                request.model
            )));
        }

        // Convert to OpenAI format
        let mut openai_req = self.convert_request(&request)?;
        
        // Enable streaming
        openai_req.stream = Some(true);
        
        // Validate the converted request
        openai_req.validate()?;

        // Build streaming URL
        let url = format!("{}/chat/completions", self.config.api_base.trim_end_matches('/'));

        tracing::info!("Starting OpenAI streaming request to: {} with model: {}", url, request.model);

        // Send streaming request
        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.config.api_key))
            .header("Content-Type", "application/json")
            .header("User-Agent", "ai-proxy/0.1.0")
            .header("Accept", "text/event-stream")
            .json(&openai_req)
            .send()
            .await
            .map_err(|e| AppError::ProviderError {
                status: 500,
                message: format!("Failed to send streaming request to OpenAI: {}", e),
            })?;

        // Check for HTTP errors
        if !response.status().is_success() {
            let status = response.status().as_u16();
            let error_body = response.text().await.unwrap_or_default();
            tracing::warn!("OpenAI streaming API error: status={}, body={}", status, error_body);
            return Err(self.handle_api_error(status, &error_body));
        }

        // Get the response body as a stream
        let body = response.bytes_stream();
        
        // Generate unique message ID for this streaming session
        let message_id = format!("msg_{}", uuid::Uuid::new_v4().simple());
        let model_name = request.model.clone();
        
        // Create initial streaming events
        let initial_events = {
            use crate::providers::anthropic::{AnthropicStreamEvent, StreamMessage, ContentBlockStart, Usage};

            let mut events = Vec::new();

            // Message start event
            let message_start = AnthropicStreamEvent::MessageStart {
                message: StreamMessage {
                    id: message_id.clone(),
                    model: model_name.clone(),
                    role: "assistant".to_string(),
                    content: vec![],
                    usage: Usage {
                        input_tokens: 0,
                        output_tokens: 0,
                    },
                },
            };
            if let Ok(json) = serde_json::to_string(&message_start) {
                events.push(format!("event: message_start\ndata: {}\n\n", json));
            }

            // Content block start event
            let content_start = AnthropicStreamEvent::ContentBlockStart {
                index: 0,
                content_block: ContentBlockStart {
                    type_field: "text".to_string(),
                    text: "".to_string(),
                },
            };
            if let Ok(json) = serde_json::to_string(&content_start) {
                events.push(format!("event: content_block_start\ndata: {}\n\n", json));
            }

            events.join("")
        };

        // Clone initial events for use in the closure
        let initial_events_clone = initial_events.clone();

        // Process streaming bytes and convert to SSE events
        let sse_stream = body
            .enumerate()
            .filter_map(move |(chunk_index, chunk_result)| {
                let message_id = message_id.clone();
                let model_name = model_name.clone();
                let initial_events = initial_events_clone.clone();

                async move {
                    match chunk_result {
                        Ok(bytes) => {
                            // Convert bytes to string
                            let chunk_str = String::from_utf8_lossy(&bytes);

                            // Debug: Log the raw chunk
                            tracing::debug!("OpenAI streaming chunk {}: {}", chunk_index, chunk_str);

                            // Process Server-Sent Events from OpenAI
                            let mut sse_events = Vec::new();

                            // Add initial events for the first chunk
                            if chunk_index == 0 {
                                sse_events.push(initial_events);
                            }

                            let lines: Vec<&str> = chunk_str.lines().collect();
                            
                            for (line_index, line) in lines.iter().enumerate() {
                                // Skip empty lines and comments
                                if line.trim().is_empty() || line.starts_with(':') {
                                    continue;
                                }

                                // Parse SSE data lines
                                if let Some(data) = line.strip_prefix("data: ") {
                                    // Check for end of stream
                                    if data.trim() == "[DONE]" {
                                        // Add content block stop and message stop events
                                        let content_stop = AnthropicStreamEvent::ContentBlockStop { index: 0 };
                                        if let Ok(json) = serde_json::to_string(&content_stop) {
                                            sse_events.push(format!("event: content_block_stop\ndata: {}\n\n", json));
                                        }
                                        
                                        let message_stop = AnthropicStreamEvent::MessageStop;
                                        if let Ok(json) = serde_json::to_string(&message_stop) {
                                            sse_events.push(format!("event: message_stop\ndata: {}\n\n", json));
                                        }
                                        continue;
                                    }

                                    // Parse JSON data from OpenAI streaming response
                                    match serde_json::from_str::<OpenAIStreamResponse>(data) {
                                        Ok(openai_stream) => {
                                            // Add message start event if this is the first chunk
                                            if chunk_index == 0 && line_index == 0 {
                                                let start_event = OpenAIStreamResponse::create_message_start_event(&model_name, &message_id);
                                                if let Ok(start_json) = serde_json::to_string(&start_event) {
                                                    sse_events.push(format!("event: message_start\ndata: {}\n\n", start_json));
                                                }
                                                
                                                // Add content block start event
                                                let content_start_event = OpenAIStreamResponse::create_content_block_start_event();
                                                if let Ok(content_json) = serde_json::to_string(&content_start_event) {
                                                    sse_events.push(format!("event: content_block_start\ndata: {}\n\n", content_json));
                                                }
                                            }
                                            
                                            // Convert to Anthropic streaming events
                                            match openai_stream.to_anthropic_events(&message_id) {
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
                                                    tracing::error!("Failed to convert OpenAI stream to Anthropic events: {}", e);
                                                    let error_event = OpenAIStreamResponse::create_error_event(&e);
                                                    if let Ok(json) = serde_json::to_string(&error_event) {
                                                        sse_events.push(format!("event: error\ndata: {}\n\n", json));
                                                    }
                                                }
                                            }
                                        }
                                        Err(parse_err) => {
                                            tracing::warn!("Failed to parse OpenAI streaming response: {} - Error: {}", data, parse_err);
                                            // Skip malformed data but continue streaming
                                        }
                                    }
                                }
                            }
                            
                            if !sse_events.is_empty() {
                                let result = sse_events.join("");
                                tracing::debug!("OpenAI streaming result for chunk {}: {}", chunk_index, result);
                                Some(Ok(result))
                            } else {
                                tracing::debug!("OpenAI streaming chunk {} produced no events", chunk_index);
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

        tracing::info!("OpenAI streaming response initialized successfully");
        Ok(Box::pin(sse_stream))
    }

    async fn list_models(&self) -> Result<Vec<ModelInfo>, AppError> {
        // Try to fetch models from OpenAI API first
        match self.fetch_models_from_api().await {
            Ok(mut models) => {
                tracing::info!("Successfully fetched {} models from OpenAI API", models.len());
                
                // Sort models by name for consistent ordering
                models.sort_by(|a, b| a.id.cmp(&b.id));
                
                // Filter to only include chat-capable models
                let chat_models: Vec<ModelInfo> = models
                    .into_iter()
                    .filter(|model| {
                        let id = &model.id;
                        id.starts_with("gpt-") && 
                        !id.contains("embedding") && 
                        !id.contains("whisper") && 
                        !id.contains("tts") && 
                        !id.contains("dall-e")
                    })
                    .collect();
                
                if chat_models.is_empty() {
                    tracing::warn!("No chat-capable models found in OpenAI API response, falling back to defaults");
                    return self.get_fallback_models();
                }
                
                Ok(chat_models)
            }
            Err(e) => {
                tracing::warn!("Failed to fetch models from OpenAI API: {}, falling back to configured models", e);
                self.get_fallback_models()
            }
        }
    }

    async fn health_check(&self) -> Result<HealthStatus, AppError> {
        let start = std::time::Instant::now();

        // Comprehensive health check with multiple endpoints
        let health_result = self.perform_comprehensive_health_check().await;
        let latency = start.elapsed().as_millis() as u64;

        match health_result {
            Ok(()) => Ok(HealthStatus {
                status: "healthy".to_string(),
                provider: "openai".to_string(),
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
                            500..=599 => ("unhealthy".to_string(), format!("OpenAI server error: {}", message)),
                            _ => ("unhealthy".to_string(), message.clone()),
                        }
                    }
                    _ => ("unhealthy".to_string(), e.to_string()),
                };

                Ok(HealthStatus {
                    status,
                    provider: "openai".to_string(),
                    latency_ms: Some(latency),
                    error: Some(error_msg),
                })
            }
        }
    }
}

impl OpenAIProvider {
    /// Get fallback models when API is unavailable
    fn get_fallback_models(&self) -> Result<Vec<ModelInfo>, AppError> {
        let models = self
            .config
            .models
            .as_ref()
            .map(|m| m.clone())
            .unwrap_or_else(|| {
                vec![
                    "gpt-4".to_string(),
                    "gpt-4-turbo".to_string(),
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
                created: 1714560000, // Static timestamp for fallback
                owned_by: "openai".to_string(),
            })
            .collect())
    }

    /// Perform comprehensive health check
    async fn perform_comprehensive_health_check(&self) -> Result<(), AppError> {
        // First, try to list models (lightweight check)
        let models_url = format!("{}/models", self.config.api_base.trim_end_matches('/'));
        
        let models_response = self
            .client
            .get(&models_url)
            .header("Authorization", format!("Bearer {}", self.config.api_key))
            .header("User-Agent", "ai-proxy/0.1.0")
            .timeout(std::time::Duration::from_secs(10))
            .send()
            .await
            .map_err(|e| AppError::ProviderError {
                status: 500,
                message: format!("Failed to connect to OpenAI: {}", e),
            })?;

        if !models_response.status().is_success() {
            let status = models_response.status().as_u16();
            let error_body = models_response.text().await.unwrap_or_default();
            return Err(self.handle_api_error(status, &error_body));
        }

        // Verify we can parse the models response
        let _models_data: serde_json::Value = models_response
            .json()
            .await
            .map_err(|e| AppError::ProviderError {
                status: 500,
                message: format!("Failed to parse OpenAI models response: {}", e),
            })?;

        // Optional: Test a minimal chat completion to verify full functionality
        // This is commented out to avoid unnecessary API calls during health checks
        // but can be enabled for more thorough health verification
        /*
        let test_request = openai_utils::create_simple_request(
            "test".to_string(),
            "gpt-3.5-turbo".to_string(),
            1
        );

        let chat_url = format!("{}/chat/completions", self.config.api_base.trim_end_matches('/'));
        let chat_response = self
            .client
            .post(&chat_url)
            .header("Authorization", format!("Bearer {}", self.config.api_key))
            .header("Content-Type", "application/json")
            .header("User-Agent", "ai-proxy/0.1.0")
            .json(&test_request)
            .timeout(std::time::Duration::from_secs(30))
            .send()
            .await
            .map_err(|e| AppError::ProviderError {
                status: 500,
                message: format!("Failed to test chat completion: {}", e),
            })?;

        if !chat_response.status().is_success() {
            let status = chat_response.status().as_u16();
            let error_body = chat_response.text().await.unwrap_or_default();
            return Err(self.handle_api_error(status, &error_body));
        }
        */

        Ok(())
    }
}
