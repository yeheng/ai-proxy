/// Integration Testing Framework for AI Proxy
///
/// This module provides comprehensive testing utilities for end-to-end integration testing
/// including mock server setup, streaming response validation, and multi-provider testing.
use ai_proxy::{
    config::{
        Config, LoggingConfig, PerformanceConfig, ProviderDetail, SecurityConfig, ServerConfig,
    },
    providers::ProviderRegistry,
    providers::anthropic::{AnthropicRequest, Message},
    server::AppState,
};
use axum::{body::Body, http::Request, response::Response};
use reqwest::Client;
use serde_json::{Value, json};
use std::{collections::HashMap, sync::Arc, time::Duration};
use tokio::sync::RwLock;
use wiremock::{
    Mock, MockServer, ResponseTemplate,
    matchers::{body_json, header as wiremock_header, method, path, query_param},
};

/// Mock server management for integration tests
pub struct IntegrationTestFramework {
    pub openai_server: Option<MockServer>,
    pub anthropic_server: Option<MockServer>,
    pub gemini_server: Option<MockServer>,
}

impl IntegrationTestFramework {
    /// Create a new integration test framework
    pub async fn new() -> Self {
        Self {
            openai_server: None,
            anthropic_server: None,
            gemini_server: None,
        }
    }

    /// Initialize OpenAI mock server
    pub async fn with_openai(mut self) -> Self {
        let server = MockServer::start().await;
        self.setup_openai_mocks(&server).await;
        self.openai_server = Some(server);
        self
    }

    /// Initialize Anthropic mock server
    pub async fn with_anthropic(mut self) -> Self {
        let server = MockServer::start().await;
        self.setup_anthropic_mocks(&server).await;
        self.anthropic_server = Some(server);
        self
    }

    /// Initialize Gemini mock server
    pub async fn with_gemini(mut self) -> Self {
        let server = MockServer::start().await;
        self.setup_gemini_mocks(&server).await;
        self.gemini_server = Some(server);
        self
    }

    /// Get provider URLs for configuration
    pub fn get_provider_urls(&self) -> HashMap<String, String> {
        let mut urls = HashMap::new();

        if let Some(server) = &self.openai_server {
            urls.insert("openai".to_string(), server.uri());
        }

        if let Some(server) = &self.anthropic_server {
            urls.insert("anthropic".to_string(), server.uri());
        }

        if let Some(server) = &self.gemini_server {
            urls.insert("gemini".to_string(), server.uri());
        }

        urls
    }

    /// Create test configuration with mock servers
    pub fn create_test_config(&self) -> Config {
        let provider_urls = self.get_provider_urls();
        let mut providers = HashMap::new();

        // Configure OpenAI provider
        if let Some(openai_url) = provider_urls.get("openai") {
            providers.insert(
                "openai".to_string(),
                ProviderDetail {
                    api_key: "test-openai-key-1234567890".to_string(),
                    api_base: format!("{}/v1/", openai_url),
                    models: Some(vec!["gpt-4".to_string(), "gpt-3.5-turbo".to_string()]),
                    timeout_seconds: 30,
                    max_retries: 3,
                    enabled: true,
                    rate_limit: None,
                },
            );
        }

        // Configure Anthropic provider
        if let Some(anthropic_url) = provider_urls.get("anthropic") {
            providers.insert(
                "anthropic".to_string(),
                ProviderDetail {
                    api_key: "test-anthropic-key-1234567890".to_string(),
                    api_base: format!("{}/v1/", anthropic_url),
                    models: Some(vec![
                        "claude-3-sonnet-20240229".to_string(),
                        "claude-3-haiku-20240307".to_string(),
                        "claude-3-sonnet".to_string(),
                        "claude-3-haiku".to_string(),
                    ]),
                    timeout_seconds: 30,
                    max_retries: 3,
                    enabled: true,
                    rate_limit: None,
                },
            );
        }

        // Configure Gemini provider
        if let Some(gemini_url) = provider_urls.get("gemini") {
            providers.insert(
                "gemini".to_string(),
                ProviderDetail {
                    api_key: "test-gemini-key-1234567890".to_string(),
                    api_base: format!("{}/v1/", gemini_url),
                    models: Some(vec![
                        "gemini-pro".to_string(),
                        "gemini-pro-vision".to_string(),
                        "gemini-1.5-pro-latest".to_string(),
                        "gemini-1.5-flash-latest".to_string(),
                    ]),
                    timeout_seconds: 30,
                    max_retries: 3,
                    enabled: true,
                    rate_limit: None,
                },
            );
        }

        Config {
            server: ServerConfig {
                host: "127.0.0.1".to_string(),
                port: 0, // Use random port for tests
                request_timeout_seconds: 30,
                max_request_size_bytes: 1024 * 1024,
            },
            providers,
            logging: LoggingConfig {
                level: "debug".to_string(),
                format: "json".to_string(),
                log_requests: true,
                log_responses: false,
            },
            security: SecurityConfig::default(),
            performance: PerformanceConfig::default(),
        }
    }

    /// Create test app state
    pub async fn create_app_state(&self) -> AppState {
        let config = self.create_test_config();
        let http_client = Client::new();
        let provider_registry = Arc::new(RwLock::new(
            ProviderRegistry::new(&config, http_client.clone()).unwrap(),
        ));
        let metrics = Arc::new(ai_proxy::metrics::MetricsCollector::new());

        AppState {
            config: Arc::new(config),
            http_client,
            provider_registry,
            metrics,
        }
    }

    /// Setup comprehensive OpenAI mock responses
    async fn setup_openai_mocks(&self, server: &MockServer) {
        // Streaming chat completion - match requests with stream: true first
        Mock::given(method("POST"))
            .and(path("/v1/chat/completions"))
            .and(|req: &wiremock::Request| {
                if let Ok(body) = std::str::from_utf8(&req.body) {
                    body.contains("\"stream\":true") || body.contains("\"stream\": true")
                } else {
                    false
                }
            })
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_string(Self::create_openai_stream_response())
                    .insert_header("content-type", "text/event-stream")
                    .insert_header("cache-control", "no-cache")
                    .insert_header("connection", "keep-alive"),
            )
            .mount(server)
            .await;

        // Standard chat completion (non-streaming fallback)
        Mock::given(method("POST"))
            .and(path("/v1/chat/completions"))
            .and(wiremock_header(
                "authorization",
                "Bearer test-openai-key-1234567890",
            ))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "id": "chatcmpl-test123",
                "object": "chat.completion",
                "created": 1234567890,
                "model": "gpt-4",
                "choices": [{
                    "index": 0,
                    "message": {
                        "role": "assistant",
                        "content": "Hello! How can I help you today?"
                    },
                    "finish_reason": "stop"
                }],
                "usage": {
                    "prompt_tokens": 10,
                    "completion_tokens": 25,
                    "total_tokens": 35
                }
            })))
            .mount(server)
            .await;

        // Models endpoint
        Mock::given(method("GET"))
            .and(path("/v1/models"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "object": "list",
                "data": [
                    {
                        "id": "gpt-4",
                        "object": "model",
                        "created": 1234567890,
                        "owned_by": "openai"
                    },
                    {
                        "id": "gpt-3.5-turbo",
                        "object": "model",
                        "created": 1234567890,
                        "owned_by": "openai"
                    }
                ]
            })))
            .mount(server)
            .await;

        // Error scenarios
        Mock::given(method("POST"))
            .and(path("/v1/chat/completions"))
            .and(body_json(json!({"error_scenario": "rate_limit"})))
            .respond_with(ResponseTemplate::new(429).set_body_json(json!({
                "error": {
                    "message": "Rate limit exceeded",
                    "type": "rate_limit_error",
                    "code": "rate_limit_exceeded"
                }
            })))
            .mount(server)
            .await;
    }

    /// Setup comprehensive Anthropic mock responses
    async fn setup_anthropic_mocks(&self, server: &MockServer) {
        // Streaming chat response - allow any path and headers for now
        Mock::given(method("POST"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_string(Self::create_anthropic_stream_response())
                    .insert_header("content-type", "text/event-stream")
                    .insert_header("cache-control", "no-cache")
                    .insert_header("connection", "keep-alive"),
            )
            .mount(server)
            .await;

        // Models endpoint
        Mock::given(method("GET"))
            .and(path("/v1/models"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "data": [
                    {
                        "id": "claude-3-sonnet-20240229",
                        "object": "model",
                        "created": 1234567890,
                        "owned_by": "anthropic"
                    },
                    {
                        "id": "claude-3-haiku-20240307",
                        "object": "model",
                        "created": 1234567890,
                        "owned_by": "anthropic"
                    },
                    {
                        "id": "claude-3-sonnet",
                        "object": "model",
                        "created": 1234567890,
                        "owned_by": "anthropic"
                    },
                    {
                        "id": "claude-3-haiku",
                        "object": "model",
                        "created": 1234567890,
                        "owned_by": "anthropic"
                    }
                ]
            })))
            .mount(server)
            .await;
    }

    /// Setup comprehensive Gemini mock responses
    async fn setup_gemini_mocks(&self, server: &MockServer) {
        // Standard chat completion
        Mock::given(method("POST"))
            .and(path("/v1/models/gemini-pro:generateContent"))
            .and(query_param("key", "test-gemini-key-1234567890"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "candidates": [{
                    "content": {
                        "role": "model",
                        "parts": [{
                            "text": "Hello! I'm Gemini, how can I assist you?"
                        }]
                    },
                    "finishReason": "STOP",
                    "index": 0
                }],
                "usageMetadata": {
                    "promptTokenCount": 8,
                    "candidatesTokenCount": 22,
                    "totalTokenCount": 30
                }
            })))
            .mount(server)
            .await;

        // Streaming chat completion - more permissive
        Mock::given(method("POST"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_string(Self::create_gemini_stream_response())
                    .insert_header("content-type", "application/json")
                    .insert_header("connection", "keep-alive"),
            )
            .mount(server)
            .await;

        // Models endpoint
        Mock::given(method("GET"))
            .and(path("/v1/models"))
            .and(query_param("key", "test-gemini-key-1234567890"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "models": [
                    {
                        "name": "models/gemini-pro",
                        "displayName": "Gemini Pro",
                        "description": "The best model for scaling across a wide range of tasks"
                    },
                    {
                        "name": "models/gemini-pro-vision",
                        "displayName": "Gemini Pro Vision",
                        "description": "The best image understanding model to handle a broad range of applications"
                    },
                    {
                        "name": "models/gemini-1.5-pro-latest",
                        "displayName": "Gemini 1.5 Pro Latest",
                        "description": "Latest Gemini 1.5 Pro model"
                    },
                    {
                        "name": "models/gemini-1.5-flash-latest",
                        "displayName": "Gemini 1.5 Flash Latest",
                        "description": "Latest Gemini 1.5 Flash model"
                    }
                ]
            })))
            .mount(server)
            .await;
    }

    /// Create realistic OpenAI streaming response
    fn create_openai_stream_response() -> String {
        vec![
            "data: {\"id\":\"chatcmpl-stream123\",\"object\":\"chat.completion.chunk\",\"created\":1234567890,\"model\":\"gpt-4\",\"choices\":[{\"index\":0,\"delta\":{\"role\":\"assistant\"},\"finish_reason\":null}]}",
            "data: {\"id\":\"chatcmpl-stream123\",\"object\":\"chat.completion.chunk\",\"created\":1234567890,\"model\":\"gpt-4\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\"Hello\"},\"finish_reason\":null}]}",
            "data: {\"id\":\"chatcmpl-stream123\",\"object\":\"chat.completion.chunk\",\"created\":1234567890,\"model\":\"gpt-4\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\" there!\"},\"finish_reason\":null}]}",
            "data: {\"id\":\"chatcmpl-stream123\",\"object\":\"chat.completion.chunk\",\"created\":1234567890,\"model\":\"gpt-4\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\" I'm\"},\"finish_reason\":null}]}",
            "data: {\"id\":\"chatcmpl-stream123\",\"object\":\"chat.completion.chunk\",\"created\":1234567890,\"model\":\"gpt-4\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\" an\"},\"finish_reason\":null}]}",
            "data: {\"id\":\"chatcmpl-stream123\",\"object\":\"chat.completion.chunk\",\"created\":1234567890,\"model\":\"gpt-4\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\" AI\"},\"finish_reason\":null}]}",
            "data: {\"id\":\"chatcmpl-stream123\",\"object\":\"chat.completion.chunk\",\"created\":1234567890,\"model\":\"gpt-4\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\" assistant\"},\"finish_reason\":null}]}",
            "data: {\"id\":\"chatcmpl-stream123\",\"object\":\"chat.completion.chunk\",\"created\":1234567890,\"model\":\"gpt-4\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\" designed\"},\"finish_reason\":null}]}",
            "data: {\"id\":\"chatcmpl-stream123\",\"object\":\"chat.completion.chunk\",\"created\":1234567890,\"model\":\"gpt-4\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\" to\"},\"finish_reason\":null}]}",
            "data: {\"id\":\"chatcmpl-stream123\",\"object\":\"chat.completion.chunk\",\"created\":1234567890,\"model\":\"gpt-4\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\" help\"},\"finish_reason\":null}]}",
            "data: {\"id\":\"chatcmpl-stream123\",\"object\":\"chat.completion.chunk\",\"created\":1234567890,\"model\":\"gpt-4\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\" with\"},\"finish_reason\":null}]}",
            "data: {\"id\":\"chatcmpl-stream123\",\"object\":\"chat.completion.chunk\",\"created\":1234567890,\"model\":\"gpt-4\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\" various\"},\"finish_reason\":null}]}",
            "data: {\"id\":\"chatcmpl-stream123\",\"object\":\"chat.completion.chunk\",\"created\":1234567890,\"model\":\"gpt-4\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\" tasks\"},\"finish_reason\":null}]}",
            "data: {\"id\":\"chatcmpl-stream123\",\"object\":\"chat.completion.chunk\",\"created\":1234567890,\"model\":\"gpt-4\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\" and\"},\"finish_reason\":null}]}",
            "data: {\"id\":\"chatcmpl-stream123\",\"object\":\"chat.completion.chunk\",\"created\":1234567890,\"model\":\"gpt-4\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\" answer\"},\"finish_reason\":null}]}",
            "data: {\"id\":\"chatcmpl-stream123\",\"object\":\"chat.completion.chunk\",\"created\":1234567890,\"model\":\"gpt-4\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\" questions.\"},\"finish_reason\":null}]}",
            "data: {\"id\":\"chatcmpl-stream123\",\"object\":\"chat.completion.chunk\",\"created\":1234567890,\"model\":\"gpt-4\",\"choices\":[{\"index\":0,\"delta\":{},\"finish_reason\":\"stop\"}]}",
            "data: [DONE]",
            ""
        ].join("\n\n")
    }

    /// Create realistic Anthropic streaming response
    fn create_anthropic_stream_response() -> String {
        let events = vec![
            json!({"type": "message_start", "message": {"id": "msg_stream123", "type": "message", "role": "assistant", "content": [], "model": "claude-3-sonnet", "usage": {"input_tokens": 15, "output_tokens": 0}}}),
            json!({"type": "content_block_start", "index": 0, "content_block": {"type": "text", "text": ""}}),
            json!({"type": "content_block_delta", "index": 0, "delta": {"type": "text_delta", "text": "Hello"}}),
            json!({"type": "content_block_delta", "index": 0, "delta": {"type": "text_delta", "text": " there!"}}),
            json!({"type": "content_block_delta", "index": 0, "delta": {"type": "text_delta", "text": " How"}}),
            json!({"type": "content_block_delta", "index": 0, "delta": {"type": "text_delta", "text": " can"}}),
            json!({"type": "content_block_delta", "index": 0, "delta": {"type": "text_delta", "text": " I"}}),
            json!({"type": "content_block_delta", "index": 0, "delta": {"type": "text_delta", "text": " help?"}}),
            json!({"type": "content_block_stop", "index": 0}),
            json!({"type": "message_delta", "delta": {"stop_reason": "end_turn", "usage": {"output_tokens": 25}}}),
            json!({"type": "message_stop"}),
        ];

        events
            .iter()
            .map(|event| format!("data: {}\n\n", serde_json::to_string(event).unwrap()))
            .collect::<Vec<_>>()
            .join("")
    }

    /// Create realistic Gemini streaming response
    fn create_gemini_stream_response() -> String {
        vec![
            json!({"candidates": [{"content": {"role": "model", "parts": [{"text": "Hello"}]}, "index": 0}]}),
            json!({"candidates": [{"content": {"role": "model", "parts": [{"text": " there!"}]}, "index": 0}]}),
            json!({"candidates": [{"content": {"role": "model", "parts": [{"text": " How"}]}, "index": 0}]}),
            json!({"candidates": [{"content": {"role": "model", "parts": [{"text": " can"}]}, "index": 0}]}),
            json!({"candidates": [{"content": {"role": "model", "parts": [{"text": " I"}]}, "index": 0}]}),
            json!({"candidates": [{"content": {"role": "model", "parts": [{"text": " help?"}]}, "index": 0}]}),
            json!({"candidates": [{"content": {"role": "model", "parts": [{"text": ""}]}, "finishReason": "STOP", "index": 0}], "usageMetadata": {"promptTokenCount": 8, "candidatesTokenCount": 22, "totalTokenCount": 30}}),
        ].iter()
            .map(|chunk| serde_json::to_string(chunk).unwrap())
            .collect::<Vec<_>>()
            .join("\n")
    }
}

/// Streaming response validation utilities
pub struct StreamingValidator;

impl StreamingValidator {
    /// Validate SSE format and event sequence
    pub fn validate_sse_response(response_body: &str) -> StreamingValidationResult {
        let mut result = StreamingValidationResult::new();
        let lines: Vec<&str> = response_body.lines().collect();

        // let mut current_event = String::new();
        let mut in_event = false;

        for line in lines {
            if line.starts_with("data: ") {
                let data = &line[6..]; // Remove "data: " prefix
                // let current_event = data.to_string();
                in_event = true;
                result.event_count += 1;

                // Try to parse as JSON
                if !data.is_empty() && data != "[DONE]" {
                    match serde_json::from_str::<Value>(data) {
                        Ok(event_json) => {
                            result.valid_json_events += 1;

                            if let Some(event_type) =
                                event_json.get("type").and_then(|t| t.as_str())
                            {
                                result.event_types.push(event_type.to_string());

                                match event_type {
                                    "message_start" => result.has_message_start = true,
                                    "content_block_start" => result.has_content_start = true,
                                    "content_block_delta" => {
                                        result.has_content_delta = true;
                                        if let Some(delta) = event_json.get("delta") {
                                            if let Some(text) =
                                                delta.get("text").and_then(|t| t.as_str())
                                            {
                                                result.content_chunks.push(text.to_string());
                                            }
                                        }
                                    }
                                    "content_block_stop" => result.has_content_stop = true,
                                    "message_delta" => result.has_message_delta = true,
                                    "message_stop" => result.has_message_stop = true,
                                    "error" => result.has_error = true,
                                    _ => {}
                                }
                            }
                        }
                        Err(_) => result.invalid_json_events += 1,
                    }
                }
            } else if line.is_empty() && in_event {
                in_event = false;
            }
        }

        result.full_content = result.content_chunks.join("");
        result.is_valid = result.validate();
        result
    }

    /// Validate streaming event sequence
    pub fn validate_event_sequence(event_types: &[String]) -> bool {
        if event_types.is_empty() {
            return false;
        }

        // Basic sequence validation
        let has_start = event_types.contains(&"message_start".to_string());
        let has_stop = event_types.contains(&"message_stop".to_string());
        let has_content = event_types.iter().any(|t| t.contains("content_block"));

        has_start && has_stop && has_content
    }
}

/// Result of streaming response validation
#[derive(Debug)]
pub struct StreamingValidationResult {
    pub event_count: usize,
    pub valid_json_events: usize,
    pub invalid_json_events: usize,
    pub event_types: Vec<String>,
    pub content_chunks: Vec<String>,
    pub full_content: String,
    pub has_message_start: bool,
    pub has_content_start: bool,
    pub has_content_delta: bool,
    pub has_content_stop: bool,
    pub has_message_delta: bool,
    pub has_message_stop: bool,
    pub has_error: bool,
    pub is_valid: bool,
}

impl StreamingValidationResult {
    fn new() -> Self {
        Self {
            event_count: 0,
            valid_json_events: 0,
            invalid_json_events: 0,
            event_types: Vec::new(),
            content_chunks: Vec::new(),
            full_content: String::new(),
            has_message_start: false,
            has_content_start: false,
            has_content_delta: false,
            has_content_stop: false,
            has_message_delta: false,
            has_message_stop: false,
            has_error: false,
            is_valid: false,
        }
    }

    fn validate(&self) -> bool {
        // Basic validation rules
        self.event_count > 0
            && self.valid_json_events > 0
            && self.has_message_start
            && self.has_message_stop
            && (self.has_content_delta || !self.full_content.is_empty())
    }
}

/// Test utilities for common operations
pub struct TestUtils;

impl TestUtils {
    /// Create a standard test request
    pub fn create_test_request(model: &str, content: &str, stream: bool) -> AnthropicRequest {
        AnthropicRequest {
            model: model.to_string(),
            messages: vec![Message::user(content.to_string())],
            max_tokens: 100,
            stream: Some(stream),
            temperature: Some(0.7),
            top_p: Some(0.9),
        }
    }

    /// Parse response body as JSON
    pub async fn parse_response_json(response: Response<Body>) -> Value {
        let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        serde_json::from_slice(&body_bytes).unwrap()
    }

    /// Parse response body as string
    pub async fn parse_response_string(response: Response<Body>) -> String {
        let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        String::from_utf8(body_bytes.to_vec()).unwrap()
    }

    /// Create HTTP request with JSON body
    pub fn create_json_request(method: &str, uri: &str, body: Value) -> Request<Body> {
        Request::builder()
            .method(method)
            .uri(uri)
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_string(&body).unwrap()))
            .unwrap()
    }

    /// Create empty HTTP request
    pub fn create_empty_request(method: &str, uri: &str) -> Request<Body> {
        Request::builder()
            .method(method)
            .uri(uri)
            .body(Body::empty())
            .unwrap()
    }

    /// Verify standard response format
    pub fn verify_standard_response(response_json: &Value, expected_model: &str) {
        assert_eq!(response_json["model"], expected_model);
        assert!(response_json["content"].is_array());
        assert!(response_json["usage"]["input_tokens"].is_number());
        assert!(response_json["usage"]["output_tokens"].is_number());
        assert!(response_json["id"].is_string());
    }

    /// Verify streaming response headers
    pub fn verify_streaming_headers(response: &Response<Body>) {
        assert_eq!(
            response.headers().get("content-type").unwrap(),
            "text/event-stream"
        );
        assert_eq!(response.headers().get("cache-control").unwrap(), "no-cache");
    }
}

/// Performance testing utilities
pub struct PerformanceTestUtils;

impl PerformanceTestUtils {
    /// Measure request latency
    pub async fn measure_latency<F, Fut>(operation: F) -> Duration
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = ()>,
    {
        let start = std::time::Instant::now();
        operation().await;
        start.elapsed()
    }

    /// Run concurrent requests and measure performance
    pub async fn run_concurrent_test<F, Fut>(
        operation_factory: F,
        concurrency: usize,
    ) -> Vec<Duration>
    where
        F: Fn() -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = ()> + Send + 'static,
    {
        let mut handles = Vec::new();

        for _ in 0..concurrency {
            let op = operation_factory();
            let handle = tokio::spawn(async move {
                let start = std::time::Instant::now();
                op.await;
                start.elapsed()
            });
            handles.push(handle);
        }

        let results = futures::future::join_all(handles).await;
        results.into_iter().map(|r| r.unwrap()).collect()
    }
}
