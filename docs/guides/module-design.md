# Module Design Guide

This guide provides a comprehensive overview of the AI Proxy module structure and implementation details.

## Project Structure

```
src/
├── main.rs           # Application entry point
├── config.rs         # Configuration management
├── errors.rs         # Error handling
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

### 3. `errors.rs` - Unified Error Handling

**Responsibilities:**

- Define global `AppError` enum for all error types
- Implement `IntoResponse` for HTTP error responses
- Provide error conversion traits

**Implementation:**

```rust
// src/errors.rs

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;

#[derive(Debug)]
pub enum AppError {
    // Client errors
    BadRequest(String),
    // Proxy internal errors
    InternalServerError(String),
    ProviderNotFound(String),
    // Backend API errors
    ProviderError {
        status: u16,
        message: String,
    },
    // Dependency errors
    ReqwestError(reqwest::Error),
    JsonError(serde_json::Error),
}

// Convert AppError to HTTP response
impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, error_message) = match self {
            AppError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg),
            AppError::ProviderError { status, message } => {
                (StatusCode::from_u16(status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR), message)
            }
            AppError::InternalServerError(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
            AppError::ProviderNotFound(msg) => (StatusCode::NOT_FOUND, msg),
            // Convert external errors to internal server errors
            AppError::ReqwestError(e) => {
                tracing::error!("Reqwest error: {:?}", e);
                (StatusCode::INTERNAL_SERVER_ERROR, "Internal network error".to_string())
            }
            AppError::JsonError(e) => {
                tracing::error!("JSON error: {:?}", e);
                (StatusCode::INTERNAL_SERVER_ERROR, "Serialization error".to_string())
            }
        };

        let body = Json(json!({ "error": { "message": error_message } }));
        (status, body).into_response()
    }
}

// From traits for ? operator
impl From<reqwest::Error> for AppError {
    fn from(err: reqwest::Error) -> Self { AppError::ReqwestError(err) }
}

impl From<serde_json::Error> for AppError {
    fn from(err: serde_json::Error) -> Self { AppError::JsonError(err) }
}
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
