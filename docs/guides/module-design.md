# Module Design Guide with Enhanced Error Handling

This guide provides a comprehensive overview of the AI Proxy module structure and implementation details, with enhanced error handling using `anyhow` + `thiserror`.

## Project Structure

```
src/
├── main.rs           # Application entry point
├── config.rs         # Configuration management
├── errors.rs         # Error handling with anyhow + thiserror
├── server.rs         # Web server and routing
└── providers/        # AI provider implementations
    ├── mod.rs        # Provider trait and exports
    ├── anthropic.rs  # Anthropic API format definitions
    ├── gemini.rs     # Google Gemini implementation
    └── openai.rs     # OpenAI implementation (future)
```

## Module Details

### 1. `main.rs` - Application Entry Point

**Responsibilities:**

- Initialize global services (logging, configuration)
- Create and configure Axum Web server
- Build shared application state `AppState`
- Define routes and start service

**Code Structure:**

```rust
// src/main.rs

mod config;
mod errors;
mod providers;
mod server;

use std::net::SocketAddr;
use std::sync::Arc;
use axum::{Router, routing::post};
use reqwest::Client;
use server::{chat_handler, AppState};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() {
    // 1. Initialize logging
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .init();

    // 2. Load configuration
    let app_config = config::load_config().expect("Failed to load configuration.");

    // 3. Create shared AppState
    let app_state = Arc::new(AppState {
        config: Arc::new(app_config),
        http_client: Client::new(), // Reusable HTTP client
    });

    // 4. Define routes
    let app = Router::new()
        .route("/v1/messages", post(chat_handler)) // Unified entry point
        .with_state(app_state);

    // 5. Start server
    let addr = SocketAddr::from((
        app_config.server.host.parse().unwrap(),
        app_config.server.port,
    ));
    tracing::info!("Listening on {}", addr);
    
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app.into_make_service()).await.unwrap();
}
```

### 2. `config.rs` - Configuration Management

**Responsibilities:**

- Define Rust structs matching `config.toml` structure
- Provide configuration loading function
- Support environment variable overrides

**Implementation:**

```rust
// src/config.rs

use serde::Deserialize;
use figment::{Figment, providers::{Format, Toml, Env}};
use std::collections::HashMap;

#[derive(Deserialize, Debug)]
pub struct Config {
    pub server: ServerConfig,
    pub providers: HashMap<String, ProviderDetail>, // Dynamic provider loading
}

#[derive(Deserialize, Debug)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
}

#[derive(Deserialize, Debug, Clone)]
pub struct ProviderDetail {
    pub api_key: String,
    pub api_base: String,
    pub models: Option<Vec<String>>, // Optional model aliases/versions
}

pub fn load_config() -> Result<Config, figment::Error> {
    Figment::new()
        .merge(Toml::file("config.toml")) // Load from file
        .merge(Env::prefixed("AI_PROXY_")) // Override with env vars
        .extract()
}
```

### 3. `errors.rs` - Error Handling with anyhow + thiserror

**Responsibilities:**

- Provide well-typed errors for API responses using `thiserror`
- Support context-rich error handling with `anyhow`
- Implement `IntoResponse` for HTTP error responses
- Enable ergonomic error propagation throughout the codebase

**Implementation:**

```rust
// src/errors.rs

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use thiserror::Error;
use serde_json::json;

/// Application-specific errors that need special handling
#[derive(Error, Debug)]
pub enum AppError {
    #[error("Bad request: {0}")]
    BadRequest(String),
    
    #[error("Provider not found: {0}")]
    ProviderNotFound(String),
    
    #[error("Provider error: {message}")]
    ProviderError {
        status: u16,
        message: String,
    },
    
    #[error("Internal server error: {0}")]
    InternalServerError(String),
    
    #[error("Configuration error: {0}")]
    ConfigError(String),
    
    #[error("Request validation failed: {0}")]
    ValidationError(String),
}

impl AppError {
    pub fn bad_request(msg: impl Into<String>) -> Self {
        Self::BadRequest(msg.into())
    }

    pub fn provider_not_found(msg: impl Into<String>) -> Self {
        Self::ProviderNotFound(msg.into())
    }

    pub fn provider_error(status: u16, message: impl Into<String>) -> Self {
        Self::ProviderError {
            status,
            message: message.into(),
        }
    }

    pub fn internal(msg: impl Into<String>) -> Self {
        Self::InternalServerError(msg.into())
    }
}

/// Convert AppError to HTTP response
impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, error_message) = match &self {
            AppError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg.clone()),
            AppError::ProviderNotFound(msg) => (StatusCode::NOT_FOUND, msg.clone()),
            AppError::ProviderError { status, message } => {
                (StatusCode::from_u16(*status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR), message.clone())
            }
            AppError::InternalServerError(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg.clone()),
            AppError::ConfigError(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg.clone()),
            AppError::ValidationError(msg) => (StatusCode::BAD_REQUEST, msg.clone()),
        };

        let error_type = match &self {
            AppError::BadRequest(_) => "invalid_request_error",
            AppError::ProviderNotFound(_) => "not_found",
            AppError::ProviderError { .. } => "provider_error",
            AppError::InternalServerError(_) => "internal_error",
            AppError::ConfigError(_) => "config_error",
            AppError::ValidationError(_) => "validation_error",
        };

        let body = Json(json!({
            "error": {
                "message": error_message,
                "type": error_type,
            }
        }));
        
        (status, body).into_response()
    }
}

/// Convert from anyhow::Error to AppError for error context
impl From<anyhow::Error> for AppError {
    fn from(err: anyhow::Error) -> Self {
        tracing::error!("Application error: {:?}", err);
        AppError::InternalServerError(err.to_string())
    }
}

/// Helper type for results that use anyhow for error handling
pub type AppResult<T> = Result<T, AppError>;

/// Helper type for results that use anyhow for internal operations
pub type AnyhowResult<T> = anyhow::Result<T>;
```

### Error Handling Strategy with anyhow + thiserror

**When to use which:**

- **`anyhow::Result<T>`**: Use for internal operations where error context is more important than specific error types
- **`AppError`**: Use for errors that need to be returned to API clients with specific HTTP status codes
- **`AppResult<T>`**: Use as return type for handler functions that need to return HTTP responses

**Usage Examples:**

```rust
// Internal operations - use anyhow::Result
use anyhow::{Context, Result as AnyhowResult};

async fn load_configuration() -> AnyhowResult<Config> {
    let config = Figment::new()
        .merge(Toml::file("config.toml"))
        .extract::<Config>()
        .context("Failed to load configuration")?;
    Ok(config)
}

// API handlers - use AppResult
use crate::errors::AppResult;

pub async fn chat_handler(
    State(state): State<Arc<AppState>>,
    Json(request): Json<AnthropicRequest>,
) -> AppResult<Response> {
    let provider = match_provider(&request.model, &state)?; // any error becomes AppError
    // ...
    Ok(response)
}

// Converting anyhow errors to AppError
let result = load_configuration()
    .map_err(|e| AppError::ConfigError(e.to_string()))?;
```

### 4. `server.rs` - Web Server Core Logic

**Responsibilities:**

- Define shared state `AppState`
- Implement core `chat_handler` for request processing
- Handle provider routing and selection
- Manage streaming vs non-streaming responses

**Implementation:**

```rust
// src/server.rs

use std::sync::Arc;
use axum::{
    extract::State,
    response::{Response, IntoResponse},
    Json,
};
use axum::body::Body;
use reqwest::Client;

use crate::{
    config::{Config, ProviderDetail},
    errors::AppError,
    providers::{AIProvider, anthropic::AnthropicRequest},
};

// Shared application state
pub struct AppState {
    pub config: Arc<Config>,
    pub http_client: Client,
}

// Core request handler
pub async fn chat_handler(
    State(state): State<Arc<AppState>>,
    Json(request): Json<AnthropicRequest>,
) -> Result<Response, AppError> {
    tracing::info!("Received chat request for model: {}", &request.model);

    // 1. Select provider based on model name
    let provider = match_provider(&request.model, &state)?;
    
    // 2. Handle streaming vs non-streaming
    if request.stream.unwrap_or(false) {
        let stream = provider.chat_stream(request).await?;
        let body = Body::from_stream(stream);
        Ok(Response::builder()
            .header("Content-Type", "text/event-stream")
            .header("Cache-Control", "no-cache")
            .header("Connection", "keep-alive")
            .body(body)
            .unwrap())
    } else {
        let response = provider.chat(request).await?;
        Ok(Json(response).into_response())
    }
}

// Provider factory function
fn match_provider(
    model_name: &str,
    state: &Arc<AppState>,
) -> Result<Box<dyn AIProvider + Send + Sync>, AppError> {
    // Simple prefix-based matching
    if model_name.starts_with("gemini-") {
        let gemini_config = state.config.providers.get("gemini")
            .ok_or_else(|| AppError::ProviderNotFound("Gemini config not found".to_string()))?;
        
        Ok(Box::new(providers::gemini::GeminiProvider::new(
            gemini_config.clone(), 
            state.http_client.clone()
        )))
    } else if model_name.starts_with("gpt-") {
        let openai_config = state.config.providers.get("openai")
            .ok_or_else(|| AppError::ProviderNotFound("OpenAI config not found".to_string()))?;
        
        Ok(Box::new(providers::openai::OpenAIProvider::new(
            openai_config.clone(),
            state.http_client.clone()
        )))
    } else if model_name.starts_with("claude-") {
        let anthropic_config = state.config.providers.get("anthropic")
            .ok_or_else(|| AppError::ProviderNotFound("Anthropic config not found".to_string()))?;
        
        Ok(Box::new(providers::anthropic::AnthropicProvider::new(
            anthropic_config.clone(),
            state.http_client.clone()
        )))
    } else {
        Err(AppError::ProviderNotFound(format!(
            "No provider configured for model: {}",
            model_name
        )))
    }
}
```

### 5. `providers/mod.rs` - Provider Interface

**Responsibilities:**

- Define `AIProvider` trait that all providers must implement
- Export provider implementations
- Define common types for streaming responses

**Implementation:**

```rust
// src/providers/mod.rs

pub mod anthropic;
pub mod gemini;
pub mod openai;

use async_trait::async_trait;
use futures::stream::BoxStream;
use crate::errors::AppError;
use self::anthropic::{AnthropicRequest, AnthropicResponse};

// Streaming response type alias
pub type StreamResponse = BoxStream<'static, Result<String, AppError>>;

// Core AI Provider trait
#[async_trait]
pub trait AIProvider: Send + Sync {
    // Handle non-streaming requests
    async fn chat(&self, request: AnthropicRequest) -> Result<AnthropicResponse, AppError>;
    
    // Handle streaming requests
    async fn chat_stream(&self, request: AnthropicRequest) -> Result<StreamResponse, AppError>;
}
```

### 6. `providers/anthropic.rs` - API Format Definitions

**Responsibilities:**

- Define common API format based on Anthropic API
- Provide request/response structures used by all providers
- Ensure consistency across different AI providers

**Implementation:**

```rust
// src/providers/anthropic.rs

use serde::{Deserialize, Serialize};

// Request structures
#[derive(Serialize, Deserialize, Debug)]
pub struct AnthropicRequest {
    pub model: String,
    pub messages: Vec<Message>,
    pub max_tokens: u32,
    pub stream: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Message {
    pub role: String, // "user" or "assistant"
    pub content: String,
}

// Response structures
#[derive(Serialize, Deserialize, Debug)]
pub struct AnthropicResponse {
    pub id: String,
    pub model: String,
    pub content: Vec<ContentBlock>,
    pub usage: Usage,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ContentBlock {
    #[serde(rename = "type")]
    pub type_field: String, // "text"
    pub text: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Usage {
    pub input_tokens: u32,
    pub output_tokens: u32,
}

// Streaming event structures
#[derive(Serialize, Debug)]
#[serde(tag = "type")]
pub enum AnthropicStreamEvent {
    #[serde(rename = "message_start")]
    MessageStart { message: AnthropicResponse },
    #[serde(rename = "content_block_delta")]
    ContentBlockDelta { index: u32, delta: TextDelta },
    #[serde(rename = "message_delta")]
    MessageDelta { delta: StopReasonDelta },
    #[serde(rename = "message_stop")]
    MessageStop,
}

#[derive(Serialize, Debug)]
pub struct TextDelta {
    #[serde(rename = "type")]
    pub type_field: String,
    pub text: String,
}

#[derive(Serialize, Debug)]
pub struct StopReasonDelta {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_reason: Option<String>,
}
```

### 7. `providers/gemini.rs` - Provider Implementation Example

**Responsibilities:**

- Implement `AIProvider` trait for Google Gemini
- Handle Gemini-specific API calls
- Convert between Anthropic format and Gemini format
- Implement both streaming and non-streaming

**Implementation Structure:**

```rust
// src/providers/gemini.rs

use async_trait::async_trait;
use futures::{StreamExt, stream};
use reqwest::Client;
use serde::{Deserialize, Serialize};

use crate::{
    config::ProviderDetail,
    errors::AppError,
    providers::{AIProvider, StreamResponse, anthropic::*},
};

// Gemini-specific data models
#[derive(Serialize, Debug)]
struct GeminiRequest {
    contents: Vec<GeminiContent>,
    generation_config: GenerationConfig,
}

#[derive(Deserialize, Debug)]
struct GeminiResponse {
    candidates: Vec<GeminiCandidate>,
    usage_metadata: UsageMetadata,
}

pub struct GeminiProvider {
    config: ProviderDetail,
    client: Client,
}

impl GeminiProvider {
    pub fn new(config: ProviderDetail, client: Client) -> Self {
        Self { config, client }
    }
    
    // Conversion helpers
    fn convert_request(&self, request: &AnthropicRequest) -> Result<GeminiRequest, AppError> {
        // Implementation details...
    }
    
    fn convert_response(&self, gemini_res: GeminiResponse) -> Result<AnthropicResponse, AppError> {
        // Implementation details...
    }
}

#[async_trait]
impl AIProvider for GeminiProvider {
    async fn chat(&self, request: AnthropicRequest) -> Result<AnthropicResponse, AppError> {
        // 1. Convert request format
        let gemini_req = self.convert_request(&request)?;
        let url = format!("{}{}:generateContent?key={}", 
            self.config.api_base, request.model, self.config.api_key);

        // 2. Send request to Gemini API
        let response = self.client.post(&url)
            .json(&gemini_req)
            .send()
            .await?;
        
        // 3. Handle API errors
        if !response.status().is_success() {
            let error_body = response.text().await.unwrap_or_default();
            return Err(AppError::ProviderError { 
                status: response.status().as_u16(), 
                message: error_body 
            });
        }
        
        // 4. Parse and convert response
        let gemini_res = response.json::<GeminiResponse>().await?;
        self.convert_response(gemini_res)
    }

    async fn chat_stream(&self, request: AnthropicRequest) -> Result<StreamResponse, AppError> {
        // Complex streaming implementation
        // 1. Convert request
        // 2. Build streaming URL
        // 3. Handle SSE parsing
        // 4. Convert chunks to Anthropic format
        todo!()
    }
}
```

## Development Guidelines

### Adding New Providers

1. **Create provider module** in `src/providers/[provider].rs`
2. **Implement AIProvider trait** with both `chat` and `chat_stream`
3. **Add configuration** to `config.toml` schema
4. **Update provider matching** in `server.rs`
5. **Add tests** for the new provider

### Testing Strategy

#### Unit Tests

- Test provider-specific request/response conversion
- Test error handling edge cases
- Test configuration loading

#### Integration Tests

- Test full request/response flow
- Test streaming functionality
- Test provider switching

#### Load Tests

- Test concurrent request handling
- Test memory usage under load
- Test provider rate limit handling

## Best Practices

### Code Organization

- Keep modules small and focused
- Use clear naming conventions
- Document public APIs
- Handle errors gracefully

### Performance

- Use connection pooling for HTTP clients
- Implement proper streaming for large responses
- Cache provider configurations
- Monitor memory usage

### Security

- Never log API keys or sensitive data
- Validate all input parameters
- Use HTTPS for all external requests
- Implement rate limiting

### Async/Await

- Use `Send + Sync` bounds for trait objects
- Handle cancellation properly
- Avoid blocking operations
- Use appropriate timeouts

### Dependencies

Add to `Cargo.toml`:
```toml
[dependencies]
# ... existing dependencies ...
anyhow = "1.0"
thiserror = "2.0"
```

### Benefits of Enhanced Error Handling

1. **Better Error Context**: `anyhow` provides rich error context with `.context()`
2. **Type Safety**: `thiserror` ensures well-typed errors for API responses
3. **Ergonomic**: Simplified error handling with `?` operator
4. **Maintainable**: Clear separation between internal and external errors
5. **Extensible**: Easy to add new error types without breaking changes

## 高级设计与优化

本节探讨了在现有坚实基础上可以进一步增强系统健壮性、可维护性和可扩展性的高级设计模式和优化策略。

### 1. 配置热重载 (Configuration Hot-Reloading)

**挑战**: 对于需要7x24小时运行的代理服务，在不中断服务的情况下更新配置（如添加新模型、更改API密钥或调整速率限制）至关重要。

**建议方案**: 引入配置热重载机制。可以利用 `tokio::sync::watch` 通道来分发更新后的配置。应用启动时，一个独立的任务会监视配置文件的变化。当检测到变更时，它会重新加载配置并将其发送到 `watch` 通道。应用中需要访问配置的各个部分（如 `AppState` 或特定的服务）则持有该通道的接收端。这样，它们就可以在不重启整个应用的情况下，对配置变更做出反应。

### 2. 中间件与请求生命周期 (Middleware and Request Lifecycle)

**挑战**: 认证、日志、速率限制和指标收集等横切关注点（Cross-Cutting Concerns）需要在核心业务逻辑之外统一处理。

**建议方案**: 在 `server.rs` 中明确定义一个中间件层。Axum 基于 `tower::Layer` 和 `ServiceBuilder` 提供了强大的中间件支持。可以设计一系列可重用的中间件来处理请求生命周期中的通用任务：

- **日志中间件**: 记录每个请求的详细信息（如请求ID、路径、来源IP、处理延迟、响应状态码）。
- **认证中间件**: 在请求到达核心 `chat_handler` 之前，验证 `Authorization` 头中的凭据。
- **速率限制中间件**: 基于客户端IP或API密钥实施灵活的速率限制策略，以防止滥用。
- **指标中间件**: 收集并导出 Prometheus 指标，例如请求总数（`http_requests_total`）、错误率和延迟直方图（`http_request_duration_seconds`）。

### 3. 动态提供者路由 (Dynamic Provider Routing)

**挑战**: `server.rs` 中的 `match_provider` 函数使用硬编码的 `if-else` 逻辑来选择提供者。随着支持的提供者增多，这种方式会变得笨重且难以维护。

**建议方案**: 实现一个更具扩展性的动态提供者路由机制。可以在 `AppState` 中维护一个 `HashMap<String, Arc<dyn AIProvider>>`。应用启动时，根据配置文件动态地初始化所有提供者，并将其注册到这个 `HashMap` 中。`match_provider` 函数的逻辑可以简化为直接从 `HashMap` 中根据模型名称查找对应的提供者实例。这种方法不仅简化了代码，还使得在运行时（结合配置热重载）添加或移除提供者成为可能。

### 4. 深化测试策略 (Advanced Testing Strategies)

**挑战**: 集成测试依赖于外部AI提供者的API，这会导致测试不稳定、缓慢且成本高昂。

**建议方案**: 推荐使用 `wiremock-rs` 或 `mockall` 等库来模拟外部API。`wiremock-rs` 可以启动一个本地HTTP服务器，用于模拟真实的API行为，允许你精确控制响应内容、状态码和延迟。这使得集成测试可以：

- **完全独立**: 不再需要网络连接或真实的API密钥。
- **确定性高**: 确保测试在任何环境下都产生相同的结果。
- **覆盖全面**: 轻松模拟各种成功和失败的场景，包括API错误、网络超时、格式错误等，从而验证应用的错误处理逻辑。

### 5. 增强可观测性 (Enhanced Observability)

**挑战**: 单纯的日志记录不足以全面了解系统在高负载下的行为和性能瓶颈。

**建议方案**: 构建一个由日志、指标和分布式追踪组成的三位一体可观测性体系。

- **指标 (Metrics)**: 集成 `metrics` 和 `metrics-exporter-prometheus` 库，在应用中暴露一个 `/metrics` 端点。通过这个端点，可以监控关键业务指标，如请求总数、错误率、各提供者的使用分布、处理延迟等。
- **分布式追踪 (Distributed Tracing)**: 集成 `tracing-opentelemetry`，将 `tracing` 的 spans 导出到兼容 OpenTelemetry 的后端（如 Jaeger 或 Zipkin）。这对于理解一个复杂请求在系统内部的完整调用链非常有价值，尤其是在排查性能瓶颈和分布式环境中的错误时。