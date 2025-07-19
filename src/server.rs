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
    metrics::MetricsCollector,
};

/// 应用程序状态 - 在所有请求处理器之间共享
/// 
/// 包含请求处理器所需的所有共享资源，
/// 包括配置、HTTP客户端、提供商注册表和指标收集器
#[derive(Clone)]
pub struct AppState {
    /// 应用程序配置（只读共享）
    pub config: Arc<Config>,
    /// HTTP客户端，用于与AI提供商通信
    pub http_client: Client,
    /// 提供商注册表，管理所有AI提供商
    pub provider_registry: Arc<Mutex<ProviderRegistry>>,
    /// 指标收集器，用于系统监控
    pub metrics: Arc<MetricsCollector>,
}

impl AppState {
    /// 从配置创建新的应用程序状态
    ///
    /// ## 功能说明
    /// 根据提供的配置创建应用程序的共享状态，包括HTTP客户端和提供商注册表
    ///
    /// ## 内部实现逻辑
    /// 1. 创建配置了连接池的HTTP客户端
    /// 2. 设置30秒请求超时和连接池参数
    /// 3. 使用HTTP客户端创建提供商注册表
    /// 4. 将所有组件包装在Arc中以支持多线程共享
    /// 5. 返回完整的应用程序状态对象
    ///
    /// ## 参数说明
    /// - `config`: 应用程序配置对象，包含服务器和提供商设置
    ///
    /// ## 执行例子
    /// ```rust
    /// let config = load_config()?;
    /// let app_state = AppState::new(config)?;
    /// println!("Application state created successfully");
    /// ```
    ///
    /// ## 返回值
    /// - `Ok(AppState)`: 成功创建的应用程序状态
    /// - `Err(AppError)`: 创建失败，可能是HTTP客户端或提供商注册表创建失败
    pub fn new(config: Config) -> AppResult<Self> {
        // 创建带连接池的HTTP客户端
        let http_client = Client::builder()
            .timeout(std::time::Duration::from_secs(30))  // 30秒超时
            .pool_max_idle_per_host(10)  // 每个主机最多10个空闲连接
            .pool_idle_timeout(std::time::Duration::from_secs(90))  // 90秒空闲超时
            .build()
            .map_err(|e| AppError::ConfigError(format!("Failed to create HTTP client: {}", e)))?;

        // 创建提供商注册表
        let provider_registry = Arc::new(Mutex::new(
            ProviderRegistry::new(&config, http_client.clone())?
        ));

        Ok(Self {
            config: Arc::new(config),  // 配置的只读共享
            http_client,  // HTTP客户端
            provider_registry,  // 提供商注册表的线程安全共享
            metrics: Arc::new(MetricsCollector::new()),  // 指标收集器
        })
    }
}

/// 创建主应用程序路由器，包含所有路由和中间件
///
/// ## 功能说明
/// 构建完整的HTTP路由器，配置所有API端点和中间件层
///
/// ## 内部实现逻辑
/// 1. 创建新的Axum路由器
/// 2. 配置聊天完成API端点（POST /v1/messages）
/// 3. 配置模型管理端点（GET /v1/models, POST /v1/models/refresh）
/// 4. 配置健康检查端点（GET /health, GET /health/providers）
/// 5. 添加应用程序状态到路由器
/// 6. 添加HTTP追踪中间件用于请求日志
///
/// ## 参数说明
/// - `state`: 应用程序状态，包含配置和提供商注册表
///
/// ## 路由配置
/// - `POST /v1/messages`: 聊天完成请求
/// - `GET /v1/models`: 获取可用模型列表
/// - `POST /v1/models/refresh`: 刷新模型列表
/// - `GET /health`: 系统健康检查
/// - `GET /health/providers`: 提供商健康检查
///
/// ## 执行例子
/// ```rust
/// let app_state = AppState::new(config)?;
/// let app = create_app(app_state);
/// // app现在可以用于启动HTTP服务器
/// ```
pub fn create_app(state: AppState) -> Router {
    Router::new()
        // 聊天完成端点
        .route("/v1/messages", post(chat_handler))
        // 模型管理端点
        .route("/v1/models", get(list_models_handler))
        .route("/v1/models/refresh", post(refresh_models_handler))
        // 健康检查端点
        .route("/health", get(health_handler))
        .route("/health/providers", get(health_providers_handler))
        // 指标端点
        .route("/metrics", get(metrics_handler))
        // 添加共享状态
        .with_state(state)
        // 添加中间件层
        .layer(
            ServiceBuilder::new()
                .layer(TraceLayer::new_for_http())  // HTTP请求追踪
        )
}

/// 启动HTTP服务器
///
/// ## 功能说明
/// 启动AI代理HTTP服务器，监听指定地址和端口，处理客户端请求
///
/// ## 内部实现逻辑
/// 1. 初始化tracing日志系统
/// 2. 从配置创建应用程序状态
/// 3. 创建包含所有路由的应用程序路由器
/// 4. 绑定TCP监听器到指定地址
/// 5. 记录服务器启动信息和可用端点
/// 6. 启动Axum服务器并等待请求
///
/// ## 参数说明
/// - `config`: 服务器配置，包含主机地址、端口等信息
///
/// ## 执行例子
/// ```rust
/// let config = load_config()?;
/// start_server(config).await?;
/// // 服务器将持续运行直到收到停止信号
/// ```
///
/// ## 返回值
/// - `Ok(())`: 服务器正常关闭
/// - `Err(AppError)`: 服务器启动或运行失败
pub async fn start_server(config: Config) -> AppResult<()> {
    // 初始化日志追踪系统
    tracing_subscriber::fmt()
        .with_target(false)  // 不显示目标模块
        .compact()  // 紧凑格式
        .init();

    // 创建应用程序状态
    let app_state = AppState::new(config.clone())?;

    // 创建路由器
    let app = create_app(app_state);

    // 创建TCP监听器
    let addr = format!("{}:{}", config.server.host, config.server.port);
    let listener = TcpListener::bind(&addr).await
        .map_err(|e| AppError::ConfigError(format!("Failed to bind to {}: {}", addr, e)))?;

    // 记录服务器启动信息
    tracing::info!("AI Proxy server starting on {}", addr);
    tracing::info!("Available endpoints:");
    tracing::info!("  POST /v1/messages - Chat completion");
    tracing::info!("  GET  /v1/models - List available models");
    tracing::info!("  POST /v1/models/refresh - Refresh models from providers");
    tracing::info!("  GET  /health - System health check");
    tracing::info!("  GET  /health/providers - Provider health check");
    tracing::info!("  GET  /metrics - System metrics and statistics");

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
) -> AppResult<axum::response::Response> {
    use axum::response::{Response, IntoResponse};
    use axum::body::Body;
    
    // Record request start time for metrics
    let start_time = state.metrics.record_request_start();
    
    tracing::info!("Processing chat request for model: {}", request.model);

    // Extract provider name from model for metrics
    let provider_name = if request.model.starts_with("gpt") || request.model.starts_with("openai") {
        "openai"
    } else if request.model.starts_with("gemini") {
        "gemini"
    } else if request.model.starts_with("claude") || request.model.starts_with("anthropic") {
        "anthropic"
    } else {
        "unknown"
    };

    // Get provider for the requested model
    let provider_result = {
        let registry = state.provider_registry.lock().await;
        registry.get_provider_for_model(&request.model)
    };

    let provider = match provider_result {
        Ok(p) => p,
        Err(e) => {
            // Record failed request
            state.metrics.record_request_end(start_time, false, provider_name, &request.model).await;
            return Err(e);
        }
    };

    // Handle streaming vs non-streaming
    let result = if request.stream.unwrap_or(false) {
        tracing::info!("Processing streaming chat request");
        
        // Get streaming response
        match provider.chat_stream(request.clone()).await {
            Ok(stream) => {
                // Convert stream to HTTP response body
                let body = Body::from_stream(stream);
                
                // Create SSE response
                let response = Response::builder()
                    .status(200)
                    .header("Content-Type", "text/event-stream")
                    .header("Cache-Control", "no-cache")
                    .header("Connection", "keep-alive")
                    .header("Access-Control-Allow-Origin", "*")
                    .header("Access-Control-Allow-Headers", "Content-Type")
                    .body(body)
                    .map_err(|e| AppError::InternalServerError(format!("Failed to create streaming response: {}", e)))?;
                    
                tracing::info!("Streaming chat request initialized successfully");
                Ok(response)
            }
            Err(e) => Err(e)
        }
    } else {
        // Process non-streaming request
        match provider.chat(request.clone()).await {
            Ok(response) => {
                tracing::info!("Chat request completed successfully");
                Ok(Json(serde_json::to_value(response).unwrap()).into_response())
            }
            Err(e) => Err(e)
        }
    };

    // Record request completion
    let success = result.is_ok();
    state.metrics.record_request_end(start_time, success, provider_name, &request.model).await;

    result
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

/// Handle metrics endpoint
async fn metrics_handler(
    State(state): State<AppState>,
) -> AppResult<Json<Value>> {
    tracing::info!("Processing metrics request");

    let metrics_summary = state.metrics.get_metrics_summary().await;
    
    let response = json!({
        "metrics": metrics_summary
    });

    tracing::info!("Metrics request completed");
    Ok(Json(response))
}
