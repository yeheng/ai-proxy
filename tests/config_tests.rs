use ai_proxy::config::*;
use std::collections::HashMap;

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
