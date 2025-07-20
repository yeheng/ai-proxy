use serde::{Deserialize, Serialize};
use figment::{Figment, providers::{Format, Toml, Env}};
use std::collections::HashMap;
use anyhow::{Context, Result};

/// 主配置结构体
/// 
/// 包含AI代理服务的所有配置信息，从配置文件和环境变量加载
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Config {
    /// 服务器配置
    pub server: ServerConfig,
    /// AI提供商配置映射（提供商ID -> 提供商详情）
    pub providers: HashMap<String, ProviderDetail>,
    /// 日志配置（可选，有默认值）
    #[serde(default)]
    pub logging: LoggingConfig,
    /// 安全配置（可选，有默认值）
    #[serde(default)]
    pub security: SecurityConfig,
    /// 性能配置（可选，有默认值）
    #[serde(default)]
    pub performance: PerformanceConfig,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    #[serde(default = "default_request_timeout")]
    pub request_timeout_seconds: u64,
    #[serde(default = "default_max_request_size")]
    pub max_request_size_bytes: usize,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
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

#[derive(Deserialize, Serialize, Debug, Clone)]
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

#[derive(Deserialize, Serialize, Debug, Clone)]
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

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct PerformanceConfig {
    #[serde(default = "default_connection_pool_size")]
    pub connection_pool_size: usize,
    #[serde(default = "default_keep_alive_timeout")]
    pub keep_alive_timeout_seconds: u64,
    #[serde(default = "default_max_concurrent_requests")]
    pub max_concurrent_requests: usize,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
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

/// 加载配置文件和环境变量
///
/// ## 功能说明
/// 从config.toml文件和环境变量（前缀AI_PROXY_）加载配置，环境变量会覆盖配置文件中的相同设置
///
/// ## 内部实现逻辑
/// 1. 使用Figment库创建配置加载器
/// 2. 首先加载config.toml文件中的配置
/// 3. 然后加载以AI_PROXY_开头的环境变量，覆盖文件配置
/// 4. 将配置反序列化为Config结构体
/// 5. 调用validate()方法验证配置的有效性
/// 6. 返回验证通过的配置对象
///
/// ## 执行例子
/// ```rust
/// // 加载配置
/// let config = load_config()?;
/// println!("Server will run on {}:{}", config.server.host, config.server.port);
/// ```
///
/// ## 错误处理
/// - 配置文件格式错误时返回解析错误
/// - 配置验证失败时返回验证错误
/// - 必需字段缺失时返回配置错误
pub fn load_config() -> Result<Config> {
    // 创建配置加载器，按优先级合并配置源
    let config: Config = Figment::new()
        .merge(Toml::file("config.toml"))  // 基础配置文件
        .merge(Env::prefixed("AI_PROXY_"))  // 环境变量覆盖
        .extract()
        .context("Failed to load configuration from config.toml or environment variables")?;

    // 验证加载的配置是否有效
    config.validate()
        .context("Configuration validation failed")?;

    Ok(config)
}

impl Config {
    /// 验证整个配置的有效性
    ///
    /// ## 功能说明
    /// 对配置对象的所有子配置进行全面验证，确保配置参数的合法性和一致性
    ///
    /// ## 内部实现逻辑
    /// 1. 验证服务器配置（主机、端口、超时等）
    /// 2. 检查至少配置了一个AI提供商
    /// 3. 逐个验证每个提供商的配置
    /// 4. 验证日志配置的有效性
    /// 5. 验证安全配置的有效性
    /// 6. 验证性能配置的有效性
    ///
    /// ## 执行例子
    /// ```rust
    /// let config = load_config()?;
    /// config.validate()?; // 验证配置有效性
    /// println!("Configuration is valid");
    /// ```
    ///
    /// ## 返回值
    /// - `Ok(())`: 配置验证通过
    /// - `Err(anyhow::Error)`: 配置验证失败，包含详细错误信息
    pub fn validate(&self) -> Result<()> {
        // 验证服务器配置
        self.server.validate()
            .context("Server configuration validation failed")?;

        // 检查提供商配置不能为空
        if self.providers.is_empty() {
            return Err(anyhow::anyhow!("At least one provider must be configured"));
        }

        // 逐个验证每个提供商配置
        for (name, provider) in &self.providers {
            provider.validate()
                .with_context(|| format!("Provider '{}' configuration validation failed", name))?;
        }

        // 验证日志配置
        self.logging.validate()
            .context("Logging configuration validation failed")?;

        // 验证安全配置
        self.security.validate()
            .context("Security configuration validation failed")?;

        // 验证性能配置
        self.performance.validate()
            .context("Performance configuration validation failed")?;

        Ok(())
    }
}

impl ServerConfig {
    /// 验证服务器配置参数
    ///
    /// ## 功能说明
    /// 验证HTTP服务器相关配置的有效性，包括主机地址、端口、超时时间和请求大小限制
    ///
    /// ## 内部实现逻辑
    /// 1. 检查主机地址不能为空
    /// 2. 验证端口号不能为0（系统保留）
    /// 3. 验证请求超时时间在合理范围内（1-300秒）
    /// 4. 验证最大请求大小在合理范围内（1字节-100MB）
    ///
    /// ## 参数验证规则
    /// - `host`: 不能为空字符串
    /// - `port`: 必须大于0
    /// - `request_timeout_seconds`: 1-300秒之间
    /// - `max_request_size_bytes`: 1字节-100MB之间
    ///
    /// ## 执行例子
    /// ```rust
    /// let server_config = ServerConfig {
    ///     host: "0.0.0.0".to_string(),
    ///     port: 8080,
    ///     request_timeout_seconds: 30,
    ///     max_request_size_bytes: 10 * 1024 * 1024, // 10MB
    /// };
    /// server_config.validate()?;
    /// ```
    pub fn validate(&self) -> Result<()> {
        // 验证主机地址
        if self.host.is_empty() {
            return Err(anyhow::anyhow!("Server host cannot be empty"));
        }

        // 验证端口范围
        if self.port == 0 {
            return Err(anyhow::anyhow!("Server port cannot be 0"));
        }

        // 验证超时时间下限
        if self.request_timeout_seconds == 0 {
            return Err(anyhow::anyhow!("Request timeout must be greater than 0"));
        }

        // 验证超时时间上限
        if self.request_timeout_seconds > 300 {
            return Err(anyhow::anyhow!("Request timeout cannot exceed 300 seconds"));
        }

        // 验证最大请求大小下限
        if self.max_request_size_bytes == 0 {
            return Err(anyhow::anyhow!("Max request size must be greater than 0"));
        }

        // 验证最大请求大小上限（100MB）
        if self.max_request_size_bytes > 100 * 1024 * 1024 {
            return Err(anyhow::anyhow!("Max request size cannot exceed 100MB"));
        }

        Ok(())
    }
}

impl ProviderDetail {
    /// 验证AI提供商配置参数
    ///
    /// ## 功能说明
    /// 验证单个AI提供商的配置参数，包括API密钥、基础URL、超时设置、重试次数等
    ///
    /// ## 内部实现逻辑
    /// 1. 验证API密钥的存在性和长度（至少10个字符）
    /// 2. 验证API基础URL的格式和协议（必须是http://或https://）
    /// 3. 验证超时时间在合理范围内（1-600秒）
    /// 4. 验证最大重试次数不超过10次
    /// 5. 如果配置了模型列表，验证模型名称的有效性
    /// 6. 如果配置了速率限制，验证速率限制参数
    ///
    /// ## 参数验证规则
    /// - `api_key`: 不能为空，至少10个字符
    /// - `api_base`: 必须以http://或https://开头
    /// - `timeout_seconds`: 1-600秒之间
    /// - `max_retries`: 0-10次之间
    /// - `models`: 如果提供，不能为空列表，模型名不能为空
    ///
    /// ## 执行例子
    /// ```rust
    /// let provider = ProviderDetail {
    ///     api_key: "sk-1234567890abcdef".to_string(),
    ///     api_base: "https://api.openai.com/v1/".to_string(),
    ///     timeout_seconds: 30,
    ///     max_retries: 3,
    ///     enabled: true,
    ///     models: Some(vec!["gpt-4".to_string()]),
    ///     rate_limit: None,
    /// };
    /// provider.validate()?;
    /// ```
    pub fn validate(&self) -> Result<()> {
        // 验证API密钥存在性
        if self.api_key.is_empty() {
            return Err(anyhow::anyhow!("Provider API key cannot be empty"));
        }

        // 验证API密钥长度（安全性考虑）
        if self.api_key.len() < 10 {
            return Err(anyhow::anyhow!("Provider API key seems too short (minimum 10 characters)"));
        }

        // 验证API基础URL存在性
        if self.api_base.is_empty() {
            return Err(anyhow::anyhow!("Provider API base URL cannot be empty"));
        }

        // 验证API基础URL协议
        if !self.api_base.starts_with("http://") && !self.api_base.starts_with("https://") {
            return Err(anyhow::anyhow!("Provider API base URL must start with http:// or https://"));
        }

        // 验证超时时间下限
        if self.timeout_seconds == 0 {
            return Err(anyhow::anyhow!("Provider timeout must be greater than 0"));
        }

        // 验证超时时间上限（10分钟）
        if self.timeout_seconds > 600 {
            return Err(anyhow::anyhow!("Provider timeout cannot exceed 600 seconds"));
        }

        // 验证最大重试次数
        if self.max_retries > 10 {
            return Err(anyhow::anyhow!("Provider max retries cannot exceed 10"));
        }

        // 如果提供了模型列表，验证模型列表
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

        // 如果提供了速率限制配置，验证速率限制配置
        if let Some(rate_limit) = &self.rate_limit {
            rate_limit.validate()?;
        }

        Ok(())
    }
}

impl LoggingConfig {
    /// 验证日志配置参数
    ///
    /// ## 功能说明
    /// 验证日志系统的配置参数，确保日志级别和格式的有效性
    ///
    /// ## 内部实现逻辑
    /// 1. 检查日志级别是否在支持的级别列表中
    /// 2. 检查日志格式是否在支持的格式列表中
    /// 3. 所有验证通过后返回成功
    ///
    /// ## 参数验证规则
    /// - `level`: 必须是 "trace", "debug", "info", "warn", "error" 之一
    /// - `format`: 必须是 "json", "pretty", "compact" 之一
    ///
    /// ## 执行例子
    /// ```rust
    /// let logging_config = LoggingConfig {
    ///     level: "info".to_string(),
    ///     format: "json".to_string(),
    /// };
    /// logging_config.validate()?;
    /// ```
    pub fn validate(&self) -> Result<()> {
        // 验证日志级别
        let valid_levels = ["trace", "debug", "info", "warn", "error"];
        if !valid_levels.contains(&self.level.as_str()) {
            return Err(anyhow::anyhow!(
                "Invalid log level '{}': must be one of {:?}",
                self.level, valid_levels
            ));
        }

        // 验证日志格式
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
    /// 验证安全配置参数
    ///
    /// ## 功能说明
    /// 验证安全相关配置，包括API密钥和CORS设置的有效性
    ///
    /// ## 内部实现逻辑
    /// 1. 验证所有配置的API密钥长度和格式
    /// 2. 如果启用了CORS，验证允许的源地址格式
    /// 3. 确保安全配置符合最佳实践
    ///
    /// ## 参数验证规则
    /// - `api_keys`: 每个密钥不能为空，至少16个字符
    /// - `allowed_origins`: 如果CORS启用，源地址必须是"*"或有效的URL
    /// - `cors_enabled`: 布尔值，控制是否启用CORS
    ///
    /// ## 执行例子
    /// ```rust
    /// let security_config = SecurityConfig {
    ///     api_keys: vec!["secure-api-key-123456".to_string()],
    ///     cors_enabled: true,
    ///     allowed_origins: vec!["https://example.com".to_string()],
    /// };
    /// security_config.validate()?;
    /// ```
    pub fn validate(&self) -> Result<()> {
        // 验证API密钥（如果配置了）
        for api_key in &self.api_keys {
            // 检查密钥不能为空
            if api_key.is_empty() {
                return Err(anyhow::anyhow!("Security API key cannot be empty"));
            }

            // 检查密钥长度（安全性要求）
            if api_key.len() < 16 {
                return Err(anyhow::anyhow!("Security API key must be at least 16 characters long"));
            }
        }

        // 验证CORS允许的源地址（如果CORS启用）
        if self.cors_enabled && !self.allowed_origins.is_empty() {
            for origin in &self.allowed_origins {
                // 检查源地址不能为空
                if origin.is_empty() {
                    return Err(anyhow::anyhow!("Allowed origin cannot be empty"));
                }

                // 检查源地址格式（必须是通配符或有效URL）
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
    /// 验证性能配置参数
    ///
    /// ## 功能说明
    /// 验证系统性能相关配置，包括连接池大小、保活超时和并发请求限制
    ///
    /// ## 内部实现逻辑
    /// 1. 验证连接池大小在合理范围内（1-1000）
    /// 2. 验证保活超时时间在合理范围内（1-3600秒）
    /// 3. 验证最大并发请求数在合理范围内（1-10000）
    /// 4. 确保所有性能参数都有合理的上下限
    ///
    /// ## 参数验证规则
    /// - `connection_pool_size`: 1-1000之间
    /// - `keep_alive_timeout_seconds`: 1-3600秒之间
    /// - `max_concurrent_requests`: 1-10000之间
    ///
    /// ## 执行例子
    /// ```rust
    /// let perf_config = PerformanceConfig {
    ///     connection_pool_size: 100,
    ///     keep_alive_timeout_seconds: 300,
    ///     max_concurrent_requests: 1000,
    /// };
    /// perf_config.validate()?;
    /// ```
    pub fn validate(&self) -> Result<()> {
        // 验证连接池大小下限
        if self.connection_pool_size == 0 {
            return Err(anyhow::anyhow!("Connection pool size must be greater than 0"));
        }

        // 验证连接池大小上限
        if self.connection_pool_size > 1000 {
            return Err(anyhow::anyhow!("Connection pool size cannot exceed 1000"));
        }

        // 验证保活超时下限
        if self.keep_alive_timeout_seconds == 0 {
            return Err(anyhow::anyhow!("Keep alive timeout must be greater than 0"));
        }

        // 验证保活超时上限（1小时）
        if self.keep_alive_timeout_seconds > 3600 {
            return Err(anyhow::anyhow!("Keep alive timeout cannot exceed 3600 seconds"));
        }

        // 验证最大并发请求数下限
        if self.max_concurrent_requests == 0 {
            return Err(anyhow::anyhow!("Max concurrent requests must be greater than 0"));
        }

        // 验证最大并发请求数上限
        if self.max_concurrent_requests > 10000 {
            return Err(anyhow::anyhow!("Max concurrent requests cannot exceed 10000"));
        }

        Ok(())
    }
}

impl RateLimitConfig {
    /// 验证速率限制配置参数
    ///
    /// ## 功能说明
    /// 验证API速率限制配置的有效性，确保限流参数的合理性和一致性
    ///
    /// ## 内部实现逻辑
    /// 1. 验证每分钟请求数大于0且不超过10000
    /// 2. 验证突发大小大于0
    /// 3. 验证突发大小不超过每分钟请求数（逻辑一致性）
    /// 4. 确保速率限制配置符合令牌桶算法要求
    ///
    /// ## 参数验证规则
    /// - `requests_per_minute`: 1-10000之间
    /// - `burst_size`: 1到requests_per_minute之间
    /// - 突发大小不能超过每分钟请求数（防止配置冲突）
    ///
    /// ## 执行例子
    /// ```rust
    /// let rate_limit = RateLimitConfig {
    ///     requests_per_minute: 100,
    ///     burst_size: 20,
    /// };
    /// rate_limit.validate()?;
    /// ```
    pub fn validate(&self) -> Result<()> {
        // 验证每分钟请求数下限
        if self.requests_per_minute == 0 {
            return Err(anyhow::anyhow!("Requests per minute must be greater than 0"));
        }

        // 验证每分钟请求数上限
        if self.requests_per_minute > 10000 {
            return Err(anyhow::anyhow!("Requests per minute cannot exceed 10000"));
        }

        // 验证突发大小下限
        if self.burst_size == 0 {
            return Err(anyhow::anyhow!("Burst size must be greater than 0"));
        }

        // 验证突发大小与每分钟请求数的逻辑一致性
        if self.burst_size > self.requests_per_minute {
            return Err(anyhow::anyhow!("Burst size cannot exceed requests per minute"));
        }

        Ok(())
    }
}