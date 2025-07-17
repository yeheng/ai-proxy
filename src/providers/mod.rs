pub mod anthropic;
pub mod gemini;
pub mod openai;
pub mod registry;

use async_trait::async_trait;
use futures::stream::BoxStream;
use crate::errors::AppError;
use self::anthropic::{AnthropicRequest, AnthropicResponse};

// Re-export registry for easier access
pub use registry::ProviderRegistry;

/// Streaming response type alias for provider implementations
pub type StreamResponse = BoxStream<'static, Result<String, AppError>>;

/// Model information structure
#[derive(Debug, Clone, serde::Serialize)]
pub struct ModelInfo {
    pub id: String,
    pub object: String,
    pub created: u64,
    pub owned_by: String,
}

/// Health status for provider monitoring
#[derive(Debug, Clone, serde::Serialize)]
pub struct HealthStatus {
    pub status: String,
    pub provider: String,
    pub latency_ms: Option<u64>,
    pub error: Option<String>,
}

/// Core AI Provider trait that all providers must implement
/// 
/// This trait defines the standard interface for all AI providers,
/// ensuring consistent behavior across different AI services.
#[async_trait]
pub trait AIProvider: Send + Sync {
    /// Handle non-streaming chat requests
    /// 
    /// Takes a standardized AnthropicRequest and returns a standardized response.
    /// Each provider implementation handles the conversion to/from their specific API format.
    async fn chat(&self, request: AnthropicRequest) -> Result<AnthropicResponse, AppError>;
    
    /// Handle streaming chat requests
    /// 
    /// Returns a stream of Server-Sent Events formatted strings.
    /// The stream should emit events in Anthropic's streaming format.
    async fn chat_stream(&self, request: AnthropicRequest) -> Result<StreamResponse, AppError>;
    
    /// List available models for this provider
    /// 
    /// Returns a list of models that this provider supports.
    async fn list_models(&self) -> Result<Vec<ModelInfo>, AppError>;
    
    /// Check provider health and connectivity
    /// 
    /// Performs a lightweight check to verify the provider is accessible.
    async fn health_check(&self) -> Result<HealthStatus, AppError>;
}