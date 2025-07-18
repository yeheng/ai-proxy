use std::sync::Arc;
use tokio::sync::Mutex;
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

/// 应用程序状态 - 在所有请求处理器之间共享
/// 
/// 包含请求处理器所需的所有共享资源，
/// 包括配置、HTTP客户端和提供商注册表
#[derive(Clone)]
pub struct AppState {
    /// 应用程序配置（只读共享）
    pub config: Arc<Config>,
    /// HTTP客户端，用于与AI提供商通信
    pub http_client: Client,
    /// 提供商注册表，管理所有AI提供商
    pub provider_registry: Arc<Mutex<ProviderRegistry>>,
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
        let provider_registry = Arc::new(Mutex::new(
            ProviderRegistry::new(&config, http_client.clone())?
        ));

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
        .route("/v1/models/refresh", post(refresh_models_handler))
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
    tracing::info!("  POST /v1/models/refresh - Refresh models from providers");
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
    let provider = {
        let registry = state.provider_registry.lock().await;
        registry.get_provider_for_model(&request.model)?
    };

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

    let models = {
        let registry = state.provider_registry.lock().await;
        registry.list_all_models().await?
    };
    
    let response = json!({
        "object": "list",
        "data": models
    });

    tracing::info!("Models list request completed, {} models available", models.len());
    Ok(Json(response))
}

/// Handle model refresh requests
async fn refresh_models_handler(
    State(state): State<AppState>,
) -> AppResult<Json<Value>> {
    tracing::info!("Processing models refresh request");

    // Refresh models from all providers
    {
        let mut registry = state.provider_registry.lock().await;
        registry.refresh_models().await?;
    }

    // Get updated model statistics
    let stats = {
        let registry = state.provider_registry.lock().await;
        registry.get_model_stats()
    };

    let response = json!({
        "status": "success",
        "message": "Models refreshed successfully",
        "provider_stats": stats,
        "timestamp": chrono::Utc::now().to_rfc3339()
    });

    tracing::info!("Models refresh completed successfully");
    Ok(Json(response))
}

/// Handle system health check
async fn health_handler(
    State(state): State<AppState>,
) -> AppResult<Json<Value>> {
    let provider_count = {
        let registry = state.provider_registry.lock().await;
        registry.get_provider_ids().len()
    };
    
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

    let health_results = {
        let registry = state.provider_registry.lock().await;
        registry.health_check_all().await
    };
    
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
