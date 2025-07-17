use std::sync::Arc;
use axum::{
    extract::State,
    response::Json,
    routing::{get, post},
    Router,
};
use reqwest::Client;
use serde_json::{json, Value};
use tokio::net::TcpListener;
use tower::ServiceBuilder;
use tower_http::trace::TraceLayer;

use crate::{
    config::Config,
    errors::{AppError, AppResult},
    providers::{ProviderRegistry, anthropic::AnthropicRequest},
};

/// Application state shared across all request handlers
/// 
/// Contains all the shared resources needed by request handlers,
/// including configuration, HTTP client, and provider registry.
#[derive(Clone)]
pub struct AppState {
    pub config: Arc<Config>,
    pub http_client: Client,
    pub provider_registry: Arc<ProviderRegistry>,
}

impl AppState {
    /// Create new application state from configuration
    pub fn new(config: Config) -> AppResult<Self> {
        // Create HTTP client with connection pooling
        let http_client = Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .pool_max_idle_per_host(10)
            .pool_idle_timeout(std::time::Duration::from_secs(90))
            .build()
            .map_err(|e| AppError::ConfigError(format!("Failed to create HTTP client: {}", e)))?;

        // Create provider registry
        let provider_registry = Arc::new(
            ProviderRegistry::new(&config, http_client.clone())?
        );

        Ok(Self {
            config: Arc::new(config),
            http_client,
            provider_registry,
        })
    }
}

/// Create the main application router with all routes and middleware
pub fn create_app(state: AppState) -> Router {
    Router::new()
        // Chat completion endpoint
        .route("/v1/messages", post(chat_handler))
        // Model management endpoints
        .route("/v1/models", get(list_models_handler))
        // Health check endpoints
        .route("/health", get(health_handler))
        .route("/health/providers", get(health_providers_handler))
        // Add shared state
        .with_state(state)
        // Add middleware
        .layer(
            ServiceBuilder::new()
                .layer(TraceLayer::new_for_http())
        )
}

/// Start the HTTP server
pub async fn start_server(config: Config) -> AppResult<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_target(false)
        .compact()
        .init();

    // Create application state
    let app_state = AppState::new(config.clone())?;
    
    // Create router
    let app = create_app(app_state);

    // Create listener
    let addr = format!("{}:{}", config.server.host, config.server.port);
    let listener = TcpListener::bind(&addr).await
        .map_err(|e| AppError::ConfigError(format!("Failed to bind to {}: {}", addr, e)))?;

    tracing::info!("AI Proxy server starting on {}", addr);
    tracing::info!("Available endpoints:");
    tracing::info!("  POST /v1/messages - Chat completion");
    tracing::info!("  GET  /v1/models - List available models");
    tracing::info!("  GET  /health - System health check");
    tracing::info!("  GET  /health/providers - Provider health check");

    // Start server
    axum::serve(listener, app).await
        .map_err(|e| AppError::InternalServerError(format!("Server error: {}", e)))?;

    Ok(())
}

// Request Handlers

/// Handle chat completion requests
async fn chat_handler(
    State(state): State<AppState>,
    Json(request): Json<AnthropicRequest>,
) -> AppResult<Json<Value>> {
    tracing::info!("Processing chat request for model: {}", request.model);

    // Get provider for the requested model
    let provider = state.provider_registry.get_provider_for_model(&request.model)?;

    // Handle streaming vs non-streaming
    if request.stream.unwrap_or(false) {
        // TODO: Implement streaming response
        return Err(AppError::InternalServerError(
            "Streaming responses not yet implemented".to_string()
        ));
    }

    // Process non-streaming request
    let response = provider.chat(request).await?;
    
    tracing::info!("Chat request completed successfully");
    Ok(Json(serde_json::to_value(response).unwrap()))
}

/// Handle model listing requests
async fn list_models_handler(
    State(state): State<AppState>,
) -> AppResult<Json<Value>> {
    tracing::info!("Processing models list request");

    let models = state.provider_registry.list_all_models().await?;
    
    let response = json!({
        "object": "list",
        "data": models
    });

    tracing::info!("Models list request completed, {} models available", models.len());
    Ok(Json(response))
}

/// Handle system health check
async fn health_handler(
    State(state): State<AppState>,
) -> AppResult<Json<Value>> {
    let provider_count = state.provider_registry.get_provider_ids().len();
    
    let response = json!({
        "status": "healthy",
        "service": "ai-proxy",
        "version": env!("CARGO_PKG_VERSION"),
        "providers_configured": provider_count,
        "timestamp": chrono::Utc::now().to_rfc3339()
    });

    Ok(Json(response))
}

/// Handle provider health checks
async fn health_providers_handler(
    State(state): State<AppState>,
) -> AppResult<Json<Value>> {
    tracing::info!("Processing provider health check");

    let health_results = state.provider_registry.health_check_all().await;
    
    let overall_status = if health_results.values().all(|h| h.status == "healthy") {
        "healthy"
    } else {
        "degraded"
    };

    let response = json!({
        "status": overall_status,
        "providers": health_results,
        "timestamp": chrono::Utc::now().to_rfc3339()
    });

    tracing::info!("Provider health check completed");
    Ok(Json(response))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{ServerConfig, ProviderDetail};
    use std::collections::HashMap;

    fn create_test_config() -> Config {
        let mut providers = HashMap::new();
        providers.insert("test".to_string(), ProviderDetail {
            api_key: "test-key".to_string(),
            api_base: "https://api.test.com/".to_string(),
            models: Some(vec!["test-model".to_string()]),
        });

        Config {
            server: ServerConfig {
                host: "127.0.0.1".to_string(),
                port: 3000,
            },
            providers,
        }
    }

    fn create_valid_test_config() -> Config {
        let mut providers = HashMap::new();
        providers.insert("gemini".to_string(), ProviderDetail {
            api_key: "test-key".to_string(),
            api_base: "https://api.gemini.com/".to_string(),
            models: Some(vec!["gemini-pro".to_string()]),
        });

        Config {
            server: ServerConfig {
                host: "127.0.0.1".to_string(),
                port: 3000,
            },
            providers,
        }
    }

    #[tokio::test]
    async fn test_app_state_creation() {
        let config = create_test_config();
        let app_state = AppState::new(config);
        
        // Should fail because "test" is not a recognized provider type
        assert!(app_state.is_err());
    }

    #[tokio::test]
    async fn test_app_state_creation_valid() {
        let config = create_valid_test_config();
        let app_state = AppState::new(config);
        
        // Should succeed with valid provider
        assert!(app_state.is_ok());
    }

    #[tokio::test]
    async fn test_router_creation() {
        let config = create_valid_test_config();
        let app_state = AppState::new(config).unwrap();
        
        // This should not panic - router creation should work
        let _router = create_app(app_state);
    }
}