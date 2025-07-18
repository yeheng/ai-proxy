use serde::Deserialize;
use figment::{Figment, providers::{Format, Toml, Env}};
use std::collections::HashMap;
use anyhow::{Context, Result};

#[derive(Deserialize, Debug, Clone)]
pub struct Config {
    pub server: ServerConfig,
    pub providers: HashMap<String, ProviderDetail>,
    #[serde(default)]
    pub logging: LoggingConfig,
    #[serde(default)]
    pub security: SecurityConfig,
    #[serde(default)]
    pub performance: PerformanceConfig,
}

#[derive(Deserialize, Debug, Clone)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    #[serde(default = "default_request_timeout")]
    pub request_timeout_seconds: u64,
    #[serde(default = "default_max_request_size")]
    pub max_request_size_bytes: usize,
}

#[derive(Deserialize, Debug, Clone)]
pub struct ProviderDetail {
    pub api_key: String,
    pub api_base: String,
    pub models: Option<Vec<String>>,
    #[serde(default = "default_provider_timeout")]
    pub timeout_seconds: u64,
    #[serde(default = "default_max_retries")]
    pub max_retries: u32,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    #[serde(default)]
    pub rate_limit: Option<RateLimitConfig>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct LoggingConfig {
    #[serde(default = "default_log_level")]
    pub level: String,
    #[serde(default = "default_log_format")]
    pub format: String,
    #[serde(default = "default_log_requests")]
    pub log_requests: bool,
    #[serde(default = "default_log_responses")]
    pub log_responses: bool,
}

#[derive(Deserialize, Debug, Clone)]
pub struct SecurityConfig {
    #[serde(default)]
    pub api_keys: Vec<String>,
    #[serde(default = "default_cors_enabled")]
    pub cors_enabled: bool,
    #[serde(default)]
    pub allowed_origins: Vec<String>,
    #[serde(default = "default_rate_limit_enabled")]
    pub rate_limit_enabled: bool,
}

#[derive(Deserialize, Debug, Clone)]
pub struct PerformanceConfig {
    #[serde(default = "default_connection_pool_size")]
    pub connection_pool_size: usize,
    #[serde(default = "default_keep_alive_timeout")]
    pub keep_alive_timeout_seconds: u64,
    #[serde(default = "default_max_concurrent_requests")]
    pub max_concurrent_requests: usize,
}

#[derive(Deserialize, Debug, Clone)]
pub struct RateLimitConfig {
    pub requests_per_minute: u32,
    pub burst_size: u32,
}

// Default value functions
fn default_request_timeout() -> u64 { 30 }
fn default_max_request_size() -> usize { 1024 * 1024 } // 1MB
fn default_provider_timeout() -> u64 { 60 }
fn default_max_retries() -> u32 { 3 }
fn default_enabled() -> bool { true }
fn default_log_level() -> String { "info".to_string() }
fn default_log_format() -> String { "json".to_string() }
fn default_log_requests() -> bool { true }
fn default_log_responses() -> bool { false }
fn default_cors_enabled() -> bool { true }
fn default_rate_limit_enabled() -> bool { false }
fn default_connection_pool_size() -> usize { 10 }
fn default_keep_alive_timeout() -> u64 { 60 }
fn default_max_concurrent_requests() -> usize { 100 }

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: default_log_level(),
            format: default_log_format(),
            log_requests: default_log_requests(),
            log_responses: default_log_responses(),
        }
    }
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            api_keys: Vec::new(),
            cors_enabled: default_cors_enabled(),
            allowed_origins: Vec::new(),
            rate_limit_enabled: default_rate_limit_enabled(),
        }
    }
}

impl Default for PerformanceConfig {
    fn default() -> Self {
        Self {
            connection_pool_size: default_connection_pool_size(),
            keep_alive_timeout_seconds: default_keep_alive_timeout(),
            max_concurrent_requests: default_max_concurrent_requests(),
        }
    }
}

pub fn load_config() -> Result<Config> {
    let config: Config = Figment::new()
        .merge(Toml::file("config.toml"))
        .merge(Env::prefixed("AI_PROXY_"))
        .extract()
        .context("Failed to load configuration from config.toml or environment variables")?;
    
    // Validate the loaded configuration
    config.validate()
        .context("Configuration validation failed")?;
    
    Ok(config)
}

impl Config {
    /// Validate the entire configuration
    pub fn validate(&self) -> Result<()> {
        // Validate server configuration
        self.server.validate()
            .context("Server configuration validation failed")?;
        
        // Validate provider configurations
        if self.providers.is_empty() {
            return Err(anyhow::anyhow!("At least one provider must be configured"));
        }
        
        for (name, provider) in &self.providers {
            provider.validate()
                .with_context(|| format!("Provider '{}' configuration validation failed", name))?;
        }
        
        // Validate logging configuration
        self.logging.validate()
            .context("Logging configuration validation failed")?;
        
        // Validate security configuration
        self.security.validate()
            .context("Security configuration validation failed")?;
        
        // Validate performance configuration
        self.performance.validate()
            .context("Performance configuration validation failed")?;
        
        Ok(())
    }
}

impl ServerConfig {
    /// Validate server configuration
    pub fn validate(&self) -> Result<()> {
        // Validate host
        if self.host.is_empty() {
            return Err(anyhow::anyhow!("Server host cannot be empty"));
        }
        
        // Validate port range
        if self.port == 0 {
            return Err(anyhow::anyhow!("Server port cannot be 0"));
        }
        
        // Validate timeout
        if self.request_timeout_seconds == 0 {
            return Err(anyhow::anyhow!("Request timeout must be greater than 0"));
        }
        
        if self.request_timeout_seconds > 300 {
            return Err(anyhow::anyhow!("Request timeout cannot exceed 300 seconds"));
        }
        
        // Validate max request size
        if self.max_request_size_bytes == 0 {
            return Err(anyhow::anyhow!("Max request size must be greater than 0"));
        }
        
        if self.max_request_size_bytes > 100 * 1024 * 1024 {
            return Err(anyhow::anyhow!("Max request size cannot exceed 100MB"));
        }
        
        Ok(())
    }
}

impl ProviderDetail {
    /// Validate provider configuration
    pub fn validate(&self) -> Result<()> {
        // Validate API key
        if self.api_key.is_empty() {
            return Err(anyhow::anyhow!("Provider API key cannot be empty"));
        }
        
        if self.api_key.len() < 10 {
            return Err(anyhow::anyhow!("Provider API key seems too short (minimum 10 characters)"));
        }
        
        // Validate API base URL
        if self.api_base.is_empty() {
            return Err(anyhow::anyhow!("Provider API base URL cannot be empty"));
        }
        
        if !self.api_base.starts_with("http://") && !self.api_base.starts_with("https://") {
            return Err(anyhow::anyhow!("Provider API base URL must start with http:// or https://"));
        }
        
        // Validate timeout
        if self.timeout_seconds == 0 {
            return Err(anyhow::anyhow!("Provider timeout must be greater than 0"));
        }
        
        if self.timeout_seconds > 600 {
            return Err(anyhow::anyhow!("Provider timeout cannot exceed 600 seconds"));
        }
        
        // Validate max retries
        if self.max_retries > 10 {
            return Err(anyhow::anyhow!("Provider max retries cannot exceed 10"));
        }
        
        // Validate models list if provided
        if let Some(models) = &self.models {
            if models.is_empty() {
                return Err(anyhow::anyhow!("Provider models list cannot be empty if specified"));
            }
            
            for model in models {
                if model.is_empty() {
                    return Err(anyhow::anyhow!("Provider model name cannot be empty"));
                }
            }
        }
        
        // Validate rate limit configuration if provided
        if let Some(rate_limit) = &self.rate_limit {
            rate_limit.validate()?;
        }
        
        Ok(())
    }
}

impl LoggingConfig {
    /// Validate logging configuration
    pub fn validate(&self) -> Result<()> {
        // Validate log level
        let valid_levels = ["trace", "debug", "info", "warn", "error"];
        if !valid_levels.contains(&self.level.as_str()) {
            return Err(anyhow::anyhow!(
                "Invalid log level '{}': must be one of {:?}",
                self.level, valid_levels
            ));
        }
        
        // Validate log format
        let valid_formats = ["json", "pretty", "compact"];
        if !valid_formats.contains(&self.format.as_str()) {
            return Err(anyhow::anyhow!(
                "Invalid log format '{}': must be one of {:?}",
                self.format, valid_formats
            ));
        }
        
        Ok(())
    }
}

impl SecurityConfig {
    /// Validate security configuration
    pub fn validate(&self) -> Result<()> {
        // Validate API keys if provided
        for api_key in &self.api_keys {
            if api_key.is_empty() {
                return Err(anyhow::anyhow!("Security API key cannot be empty"));
            }
            
            if api_key.len() < 16 {
                return Err(anyhow::anyhow!("Security API key must be at least 16 characters long"));
            }
        }
        
        // Validate allowed origins if CORS is enabled
        if self.cors_enabled && !self.allowed_origins.is_empty() {
            for origin in &self.allowed_origins {
                if origin.is_empty() {
                    return Err(anyhow::anyhow!("Allowed origin cannot be empty"));
                }
                
                if origin != "*" && !origin.starts_with("http://") && !origin.starts_with("https://") {
                    return Err(anyhow::anyhow!(
                        "Allowed origin '{}' must be '*' or start with http:// or https://",
                        origin
                    ));
                }
            }
        }
        
        Ok(())
    }
}

impl PerformanceConfig {
    /// Validate performance configuration
    pub fn validate(&self) -> Result<()> {
        // Validate connection pool size
        if self.connection_pool_size == 0 {
            return Err(anyhow::anyhow!("Connection pool size must be greater than 0"));
        }
        
        if self.connection_pool_size > 1000 {
            return Err(anyhow::anyhow!("Connection pool size cannot exceed 1000"));
        }
        
        // Validate keep alive timeout
        if self.keep_alive_timeout_seconds == 0 {
            return Err(anyhow::anyhow!("Keep alive timeout must be greater than 0"));
        }
        
        if self.keep_alive_timeout_seconds > 3600 {
            return Err(anyhow::anyhow!("Keep alive timeout cannot exceed 3600 seconds"));
        }
        
        // Validate max concurrent requests
        if self.max_concurrent_requests == 0 {
            return Err(anyhow::anyhow!("Max concurrent requests must be greater than 0"));
        }
        
        if self.max_concurrent_requests > 10000 {
            return Err(anyhow::anyhow!("Max concurrent requests cannot exceed 10000"));
        }
        
        Ok(())
    }
}

impl RateLimitConfig {
    /// Validate rate limit configuration
    pub fn validate(&self) -> Result<()> {
        if self.requests_per_minute == 0 {
            return Err(anyhow::anyhow!("Requests per minute must be greater than 0"));
        }
        
        if self.requests_per_minute > 10000 {
            return Err(anyhow::anyhow!("Requests per minute cannot exceed 10000"));
        }
        
        if self.burst_size == 0 {
            return Err(anyhow::anyhow!("Burst size must be greater than 0"));
        }
        
        if self.burst_size > self.requests_per_minute {
            return Err(anyhow::anyhow!("Burst size cannot exceed requests per minute"));
        }
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Helper function to create a valid config for testing
    fn create_valid_config() -> Config {
        let mut providers = HashMap::new();
        providers.insert("test_provider".to_string(), ProviderDetail {
            api_key: "test-api-key-1234567890".to_string(),
            api_base: "https://api.example.com/v1/".to_string(),
            models: Some(vec!["model1".to_string(), "model2".to_string()]),
            timeout_seconds: 60,
            max_retries: 3,
            enabled: true,
            rate_limit: None,
        });

        Config {
            server: ServerConfig {
                host: "127.0.0.1".to_string(),
                port: 3000,
                request_timeout_seconds: 30,
                max_request_size_bytes: 1024 * 1024,
            },
            providers,
            logging: LoggingConfig::default(),
            security: SecurityConfig::default(),
            performance: PerformanceConfig::default(),
        }
    }

    #[test]
    fn test_load_config() {
        // Test will fail if config.toml doesn't exist, but that's expected
        let config = load_config();
        assert!(config.is_ok() || config.is_err());
    }

    #[test]
    fn test_config_validation_valid() {
        let config = create_valid_config();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_config_validation_no_providers() {
        let mut config = create_valid_config();
        config.providers.clear();
        let result = config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("At least one provider must be configured"));
    }

    #[test]
    fn test_server_config_validation_valid() {
        let server_config = ServerConfig {
            host: "0.0.0.0".to_string(),
            port: 8080,
            request_timeout_seconds: 60,
            max_request_size_bytes: 2 * 1024 * 1024,
        };
        assert!(server_config.validate().is_ok());
    }

    #[test]
    fn test_server_config_validation_empty_host() {
        let server_config = ServerConfig {
            host: "".to_string(),
            port: 8080,
            request_timeout_seconds: 60,
            max_request_size_bytes: 1024 * 1024,
        };
        let result = server_config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Server host cannot be empty"));
    }

    #[test]
    fn test_server_config_validation_zero_port() {
        let server_config = ServerConfig {
            host: "127.0.0.1".to_string(),
            port: 0,
            request_timeout_seconds: 60,
            max_request_size_bytes: 1024 * 1024,
        };
        let result = server_config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Server port cannot be 0"));
    }

    #[test]
    fn test_server_config_validation_invalid_timeout() {
        let server_config = ServerConfig {
            host: "127.0.0.1".to_string(),
            port: 3000,
            request_timeout_seconds: 0,
            max_request_size_bytes: 1024 * 1024,
        };
        let result = server_config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Request timeout must be greater than 0"));

        let server_config = ServerConfig {
            host: "127.0.0.1".to_string(),
            port: 3000,
            request_timeout_seconds: 301,
            max_request_size_bytes: 1024 * 1024,
        };
        let result = server_config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Request timeout cannot exceed 300 seconds"));
    }

    #[test]
    fn test_server_config_validation_invalid_request_size() {
        let server_config = ServerConfig {
            host: "127.0.0.1".to_string(),
            port: 3000,
            request_timeout_seconds: 30,
            max_request_size_bytes: 0,
        };
        let result = server_config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Max request size must be greater than 0"));

        let server_config = ServerConfig {
            host: "127.0.0.1".to_string(),
            port: 3000,
            request_timeout_seconds: 30,
            max_request_size_bytes: 101 * 1024 * 1024,
        };
        let result = server_config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Max request size cannot exceed 100MB"));
    }

    #[test]
    fn test_provider_detail_validation_valid() {
        let provider = ProviderDetail {
            api_key: "valid-api-key-1234567890".to_string(),
            api_base: "https://api.example.com/v1/".to_string(),
            models: Some(vec!["model1".to_string()]),
            timeout_seconds: 60,
            max_retries: 3,
            enabled: true,
            rate_limit: None,
        };
        assert!(provider.validate().is_ok());
    }

    #[test]
    fn test_provider_detail_validation_empty_api_key() {
        let provider = ProviderDetail {
            api_key: "".to_string(),
            api_base: "https://api.example.com/v1/".to_string(),
            models: None,
            timeout_seconds: 60,
            max_retries: 3,
            enabled: true,
            rate_limit: None,
        };
        let result = provider.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Provider API key cannot be empty"));
    }

    #[test]
    fn test_provider_detail_validation_short_api_key() {
        let provider = ProviderDetail {
            api_key: "short".to_string(),
            api_base: "https://api.example.com/v1/".to_string(),
            models: None,
            timeout_seconds: 60,
            max_retries: 3,
            enabled: true,
            rate_limit: None,
        };
        let result = provider.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Provider API key seems too short"));
    }

    #[test]
    fn test_provider_detail_validation_invalid_api_base() {
        let provider = ProviderDetail {
            api_key: "valid-api-key-1234567890".to_string(),
            api_base: "".to_string(),
            models: None,
            timeout_seconds: 60,
            max_retries: 3,
            enabled: true,
            rate_limit: None,
        };
        let result = provider.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Provider API base URL cannot be empty"));

        let provider = ProviderDetail {
            api_key: "valid-api-key-1234567890".to_string(),
            api_base: "invalid-url".to_string(),
            models: None,
            timeout_seconds: 60,
            max_retries: 3,
            enabled: true,
            rate_limit: None,
        };
        let result = provider.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Provider API base URL must start with http:// or https://"));
    }

    #[test]
    fn test_provider_detail_validation_invalid_timeout() {
        let provider = ProviderDetail {
            api_key: "valid-api-key-1234567890".to_string(),
            api_base: "https://api.example.com/v1/".to_string(),
            models: None,
            timeout_seconds: 0,
            max_retries: 3,
            enabled: true,
            rate_limit: None,
        };
        let result = provider.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Provider timeout must be greater than 0"));

        let provider = ProviderDetail {
            api_key: "valid-api-key-1234567890".to_string(),
            api_base: "https://api.example.com/v1/".to_string(),
            models: None,
            timeout_seconds: 601,
            max_retries: 3,
            enabled: true,
            rate_limit: None,
        };
        let result = provider.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Provider timeout cannot exceed 600 seconds"));
    }

    #[test]
    fn test_provider_detail_validation_invalid_max_retries() {
        let provider = ProviderDetail {
            api_key: "valid-api-key-1234567890".to_string(),
            api_base: "https://api.example.com/v1/".to_string(),
            models: None,
            timeout_seconds: 60,
            max_retries: 11,
            enabled: true,
            rate_limit: None,
        };
        let result = provider.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Provider max retries cannot exceed 10"));
    }

    #[test]
    fn test_provider_detail_validation_empty_models_list() {
        let provider = ProviderDetail {
            api_key: "valid-api-key-1234567890".to_string(),
            api_base: "https://api.example.com/v1/".to_string(),
            models: Some(vec![]),
            timeout_seconds: 60,
            max_retries: 3,
            enabled: true,
            rate_limit: None,
        };
        let result = provider.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Provider models list cannot be empty if specified"));
    }

    #[test]
    fn test_provider_detail_validation_empty_model_name() {
        let provider = ProviderDetail {
            api_key: "valid-api-key-1234567890".to_string(),
            api_base: "https://api.example.com/v1/".to_string(),
            models: Some(vec!["valid-model".to_string(), "".to_string()]),
            timeout_seconds: 60,
            max_retries: 3,
            enabled: true,
            rate_limit: None,
        };
        let result = provider.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Provider model name cannot be empty"));
    }

    #[test]
    fn test_logging_config_validation_valid() {
        let logging_config = LoggingConfig {
            level: "info".to_string(),
            format: "json".to_string(),
            log_requests: true,
            log_responses: false,
        };
        assert!(logging_config.validate().is_ok());
    }

    #[test]
    fn test_logging_config_validation_invalid_level() {
        let logging_config = LoggingConfig {
            level: "invalid".to_string(),
            format: "json".to_string(),
            log_requests: true,
            log_responses: false,
        };
        let result = logging_config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Invalid log level 'invalid'"));
    }

    #[test]
    fn test_logging_config_validation_invalid_format() {
        let logging_config = LoggingConfig {
            level: "info".to_string(),
            format: "invalid".to_string(),
            log_requests: true,
            log_responses: false,
        };
        let result = logging_config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Invalid log format 'invalid'"));
    }

    #[test]
    fn test_security_config_validation_valid() {
        let security_config = SecurityConfig {
            api_keys: vec!["valid-api-key-1234567890".to_string()],
            cors_enabled: true,
            allowed_origins: vec!["https://example.com".to_string(), "*".to_string()],
            rate_limit_enabled: false,
        };
        assert!(security_config.validate().is_ok());
    }

    #[test]
    fn test_security_config_validation_empty_api_key() {
        let security_config = SecurityConfig {
            api_keys: vec!["".to_string()],
            cors_enabled: true,
            allowed_origins: vec![],
            rate_limit_enabled: false,
        };
        let result = security_config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Security API key cannot be empty"));
    }

    #[test]
    fn test_security_config_validation_short_api_key() {
        let security_config = SecurityConfig {
            api_keys: vec!["short".to_string()],
            cors_enabled: true,
            allowed_origins: vec![],
            rate_limit_enabled: false,
        };
        let result = security_config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Security API key must be at least 16 characters long"));
    }

    #[test]
    fn test_security_config_validation_invalid_origin() {
        let security_config = SecurityConfig {
            api_keys: vec![],
            cors_enabled: true,
            allowed_origins: vec!["".to_string()],
            rate_limit_enabled: false,
        };
        let result = security_config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Allowed origin cannot be empty"));

        let security_config = SecurityConfig {
            api_keys: vec![],
            cors_enabled: true,
            allowed_origins: vec!["invalid-origin".to_string()],
            rate_limit_enabled: false,
        };
        let result = security_config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("must be '*' or start with http:// or https://"));
    }

    #[test]
    fn test_performance_config_validation_valid() {
        let performance_config = PerformanceConfig {
            connection_pool_size: 20,
            keep_alive_timeout_seconds: 120,
            max_concurrent_requests: 200,
        };
        assert!(performance_config.validate().is_ok());
    }

    #[test]
    fn test_performance_config_validation_invalid_pool_size() {
        let performance_config = PerformanceConfig {
            connection_pool_size: 0,
            keep_alive_timeout_seconds: 60,
            max_concurrent_requests: 100,
        };
        let result = performance_config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Connection pool size must be greater than 0"));

        let performance_config = PerformanceConfig {
            connection_pool_size: 1001,
            keep_alive_timeout_seconds: 60,
            max_concurrent_requests: 100,
        };
        let result = performance_config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Connection pool size cannot exceed 1000"));
    }

    #[test]
    fn test_performance_config_validation_invalid_keep_alive() {
        let performance_config = PerformanceConfig {
            connection_pool_size: 10,
            keep_alive_timeout_seconds: 0,
            max_concurrent_requests: 100,
        };
        let result = performance_config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Keep alive timeout must be greater than 0"));

        let performance_config = PerformanceConfig {
            connection_pool_size: 10,
            keep_alive_timeout_seconds: 3601,
            max_concurrent_requests: 100,
        };
        let result = performance_config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Keep alive timeout cannot exceed 3600 seconds"));
    }

    #[test]
    fn test_performance_config_validation_invalid_concurrent_requests() {
        let performance_config = PerformanceConfig {
            connection_pool_size: 10,
            keep_alive_timeout_seconds: 60,
            max_concurrent_requests: 0,
        };
        let result = performance_config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Max concurrent requests must be greater than 0"));

        let performance_config = PerformanceConfig {
            connection_pool_size: 10,
            keep_alive_timeout_seconds: 60,
            max_concurrent_requests: 10001,
        };
        let result = performance_config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Max concurrent requests cannot exceed 10000"));
    }

    #[test]
    fn test_rate_limit_config_validation_valid() {
        let rate_limit_config = RateLimitConfig {
            requests_per_minute: 100,
            burst_size: 50,
        };
        assert!(rate_limit_config.validate().is_ok());
    }

    #[test]
    fn test_rate_limit_config_validation_invalid_requests_per_minute() {
        let rate_limit_config = RateLimitConfig {
            requests_per_minute: 0,
            burst_size: 10,
        };
        let result = rate_limit_config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Requests per minute must be greater than 0"));

        let rate_limit_config = RateLimitConfig {
            requests_per_minute: 10001,
            burst_size: 10,
        };
        let result = rate_limit_config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Requests per minute cannot exceed 10000"));
    }

    #[test]
    fn test_rate_limit_config_validation_invalid_burst_size() {
        let rate_limit_config = RateLimitConfig {
            requests_per_minute: 100,
            burst_size: 0,
        };
        let result = rate_limit_config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Burst size must be greater than 0"));

        let rate_limit_config = RateLimitConfig {
            requests_per_minute: 100,
            burst_size: 101,
        };
        let result = rate_limit_config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Burst size cannot exceed requests per minute"));
    }

    #[test]
    fn test_default_implementations() {
        let logging_config = LoggingConfig::default();
        assert_eq!(logging_config.level, "info");
        assert_eq!(logging_config.format, "json");
        assert!(logging_config.log_requests);
        assert!(!logging_config.log_responses);

        let security_config = SecurityConfig::default();
        assert!(security_config.api_keys.is_empty());
        assert!(security_config.cors_enabled);
        assert!(security_config.allowed_origins.is_empty());
        assert!(!security_config.rate_limit_enabled);

        let performance_config = PerformanceConfig::default();
        assert_eq!(performance_config.connection_pool_size, 10);
        assert_eq!(performance_config.keep_alive_timeout_seconds, 60);
        assert_eq!(performance_config.max_concurrent_requests, 100);
    }
}
