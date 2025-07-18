use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use thiserror::Error;

// 使用anyhow::Result进行内部错误处理
// 使用thiserror定义需要特殊处理的类型化错误

/// 应用程序特定的错误类型，需要特殊处理
/// 
/// 这些错误类型提供了详细的错误信息，并映射到适当的HTTP状态码
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
    
    #[error("Authentication failed: {0}")]
    AuthenticationError(String),
    
    #[error("Authorization failed: {0}")]
    AuthorizationError(String),
    
    #[error("Rate limit exceeded: {0}")]
    RateLimitError(String),
    
    #[error("Request timeout: {0}")]
    TimeoutError(String),
    
    #[error("Service unavailable: {0}")]
    ServiceUnavailable(String),
    
    #[error("Streaming error: {0}")]
    StreamingError(String),
    
    #[error("Model not supported: {0}")]
    ModelNotSupported(String),
    
    #[error("Quota exceeded: {0}")]
    QuotaExceeded(String),
    
    #[error("Network error: {0}")]
    NetworkError(String),
    
    #[error("Serialization error: {0}")]
    SerializationError(String),
}

impl AppError {
    /// 创建错误请求错误
    ///
    /// ## 功能说明
    /// 便捷方法，创建BadRequest类型的错误，用于客户端请求格式错误的情况
    ///
    /// ## 参数说明
    /// - `msg`: 错误消息，可以是任何能转换为String的类型
    ///
    /// ## 执行例子
    /// ```rust
    /// return Err(AppError::bad_request("Invalid JSON format"));
    /// ```
    pub fn bad_request(msg: impl Into<String>) -> Self {
        Self::BadRequest(msg.into())
    }

    /// 创建提供商未找到错误
    ///
    /// ## 功能说明
    /// 便捷方法，创建ProviderNotFound类型的错误，用于找不到支持指定模型的提供商
    ///
    /// ## 参数说明
    /// - `msg`: 错误消息，通常包含模型名称和可用模型列表
    ///
    /// ## 执行例子
    /// ```rust
    /// return Err(AppError::provider_not_found("No provider for model gpt-5"));
    /// ```
    pub fn provider_not_found(msg: impl Into<String>) -> Self {
        Self::ProviderNotFound(msg.into())
    }

    /// 创建提供商错误
    ///
    /// ## 功能说明
    /// 便捷方法，创建ProviderError类型的错误，用于AI提供商API返回错误的情况
    ///
    /// ## 参数说明
    /// - `status`: HTTP状态码
    /// - `message`: 错误消息，通常来自提供商的错误响应
    ///
    /// ## 执行例子
    /// ```rust
    /// return Err(AppError::provider_error(429, "Rate limit exceeded"));
    /// ```
    pub fn provider_error(status: u16, message: impl Into<String>) -> Self {
        Self::ProviderError {
            status,
            message: message.into(),
        }
    }

    /// 创建内部服务器错误
    ///
    /// ## 功能说明
    /// 便捷方法，创建InternalServerError类型的错误，用于系统内部错误
    ///
    /// ## 参数说明
    /// - `msg`: 错误消息，描述内部错误的具体情况
    ///
    /// ## 执行例子
    /// ```rust
    /// return Err(AppError::internal("Database connection failed"));
    /// ```
    pub fn internal(msg: impl Into<String>) -> Self {
        Self::InternalServerError(msg.into())
    }
}

/// Convert AppError to HTTP response
impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, error_message, error_code) = match &self {
            AppError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg.clone(), None),
            AppError::ProviderNotFound(msg) => (StatusCode::NOT_FOUND, msg.clone(), None),
            AppError::ProviderError { status, message } => {
                (StatusCode::from_u16(*status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR), message.clone(), Some(*status))
            }
            AppError::InternalServerError(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg.clone(), None),
            AppError::ConfigError(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg.clone(), None),
            AppError::ValidationError(msg) => (StatusCode::BAD_REQUEST, msg.clone(), None),
            AppError::AuthenticationError(msg) => (StatusCode::UNAUTHORIZED, msg.clone(), None),
            AppError::AuthorizationError(msg) => (StatusCode::FORBIDDEN, msg.clone(), None),
            AppError::RateLimitError(msg) => (StatusCode::TOO_MANY_REQUESTS, msg.clone(), None),
            AppError::TimeoutError(msg) => (StatusCode::REQUEST_TIMEOUT, msg.clone(), None),
            AppError::ServiceUnavailable(msg) => (StatusCode::SERVICE_UNAVAILABLE, msg.clone(), None),
            AppError::StreamingError(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg.clone(), None),
            AppError::ModelNotSupported(msg) => (StatusCode::BAD_REQUEST, msg.clone(), None),
            AppError::QuotaExceeded(msg) => (StatusCode::TOO_MANY_REQUESTS, msg.clone(), None),
            AppError::NetworkError(msg) => (StatusCode::BAD_GATEWAY, msg.clone(), None),
            AppError::SerializationError(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg.clone(), None),
        };

        let error_type = match &self {
            AppError::BadRequest(_) => "invalid_request_error",
            AppError::ProviderNotFound(_) => "not_found_error",
            AppError::ProviderError { .. } => "provider_error",
            AppError::InternalServerError(_) => "internal_server_error",
            AppError::ConfigError(_) => "configuration_error",
            AppError::ValidationError(_) => "validation_error",
            AppError::AuthenticationError(_) => "authentication_error",
            AppError::AuthorizationError(_) => "authorization_error",
            AppError::RateLimitError(_) => "rate_limit_error",
            AppError::TimeoutError(_) => "timeout_error",
            AppError::ServiceUnavailable(_) => "service_unavailable_error",
            AppError::StreamingError(_) => "streaming_error",
            AppError::ModelNotSupported(_) => "model_not_supported_error",
            AppError::QuotaExceeded(_) => "quota_exceeded_error",
            AppError::NetworkError(_) => "network_error",
            AppError::SerializationError(_) => "serialization_error",
        };

        // Create error response with additional context
        let mut error_json = json!({
            "error": {
                "message": error_message,
                "type": error_type,
                "code": status.as_u16(),
            }
        });

        // Add provider-specific error code if available
        if let Some(provider_status) = error_code {
            error_json["error"]["provider_code"] = json!(provider_status);
        }

        // Add timestamp for debugging
        error_json["error"]["timestamp"] = json!(chrono::Utc::now().to_rfc3339());

        let body = Json(error_json);
        
        (status, body).into_response()
    }
}

/// Convert from anyhow::Error to AppError for error context
impl From<anyhow::Error> for AppError {
    fn from(err: anyhow::Error) -> Self {
        // Log the full error chain for debugging (requirement 5.4)
        tracing::error!("Internal application error: {:?}", err);
        
        // Don't expose sensitive internal error details to clients (requirement 5.4)
        AppError::InternalServerError("An internal server error occurred".to_string())
    }
}

/// Convert from reqwest::Error to AppError for HTTP client errors
impl From<reqwest::Error> for AppError {
    fn from(err: reqwest::Error) -> Self {
        tracing::error!("HTTP client error: {:?}", err);
        
        if err.is_timeout() {
            AppError::TimeoutError("Request to provider timed out".to_string())
        } else if err.is_connect() {
            AppError::NetworkError("Failed to connect to provider".to_string())
        } else if let Some(status) = err.status() {
            AppError::provider_error(status.as_u16(), "Provider API error")
        } else {
            AppError::NetworkError("Network error occurred".to_string())
        }
    }
}

/// Convert from serde_json::Error to AppError for serialization errors
impl From<serde_json::Error> for AppError {
    fn from(err: serde_json::Error) -> Self {
        tracing::error!("JSON serialization error: {:?}", err);
        AppError::SerializationError("Failed to process JSON data".to_string())
    }
}

/// Helper type for results that use anyhow for error handling
pub type AppResult<T> = Result<T, AppError>;

/// Helper type for results that use anyhow for internal operations
pub type AnyhowResult<T> = anyhow::Result<T>;
