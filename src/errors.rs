use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use thiserror::Error;

// Use anyhow::Result for internal error handling
// Use thiserror for well-typed errors that need to be handled specifically

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
        // Log the full error chain for debugging
        tracing::error!("Application error: {:?}", err);
        AppError::InternalServerError(err.to_string())
    }
}

/// Helper type for results that use anyhow for error handling
pub type AppResult<T> = Result<T, AppError>;

/// Helper type for results that use anyhow for internal operations
pub type AnyhowResult<T> = anyhow::Result<T>;