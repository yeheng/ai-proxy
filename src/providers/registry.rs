use std::collections::HashMap;
use std::sync::Arc;
use reqwest::Client;

use crate::{
    config::Config,
    errors::AppError,
    providers::{AIProvider, ModelInfo, HealthStatus},
};
use super::{
    gemini::GeminiProvider,
    openai::OpenAIProvider,
    anthropic::AnthropicProvider,
};

/// Provider registry that manages all configured AI providers
/// 
/// The registry handles provider instantiation, model-to-provider mapping,
/// and provides a unified interface for accessing providers by model name.
pub struct ProviderRegistry {
    providers: HashMap<String, Arc<dyn AIProvider + Send + Sync>>,
    model_mapping: HashMap<String, String>, // model -> provider_id
}

impl ProviderRegistry {
    /// 从配置创建新的提供商注册表
    ///
    /// ## 功能说明
    /// 根据配置文件初始化所有AI提供商，建立模型到提供商的映射关系
    ///
    /// ## 内部实现逻辑
    /// 1. 遍历配置中的所有提供商设置
    /// 2. 根据提供商ID前缀识别提供商类型（gemini/openai/anthropic）
    /// 3. 为每个提供商创建对应的实现实例
    /// 4. 获取每个提供商支持的模型列表（配置或默认）
    /// 5. 建立模型名到提供商ID的映射关系
    /// 6. 验证至少配置了一个提供商
    ///
    /// ## 参数说明
    /// - `config`: 应用程序配置，包含所有提供商的详细设置
    /// - `http_client`: 共享的HTTP客户端，用于与AI提供商通信
    ///
    /// ## 执行例子
    /// ```rust
    /// let config = load_config()?;
    /// let http_client = Client::new();
    /// let registry = ProviderRegistry::new(&config, http_client)?;
    /// println!("Initialized {} providers", registry.providers.len());
    /// ```
    ///
    /// ## 返回值
    /// - `Ok(ProviderRegistry)`: 成功创建的提供商注册表
    /// - `Err(AppError)`: 创建失败，可能是未知提供商类型或无提供商配置
    pub fn new(config: &Config, http_client: Client) -> Result<Self, AppError> {
        let mut providers: HashMap<String, Arc<dyn AIProvider + Send + Sync>> = HashMap::new();
        let mut model_mapping: HashMap<String, String> = HashMap::new();

        // 根据配置初始化提供商
        for (provider_id, provider_config) in &config.providers {
            // 根据提供商ID前缀创建对应的提供商实例
            let provider: Arc<dyn AIProvider + Send + Sync> = match provider_id.as_str() {
                id if id.starts_with("gemini") => {
                    Arc::new(GeminiProvider::new(provider_config.clone(), http_client.clone()))
                }
                id if id.starts_with("openai") => {
                    Arc::new(OpenAIProvider::new(provider_config.clone(), http_client.clone()))
                }
                id if id.starts_with("anthropic") => {
                    Arc::new(AnthropicProvider::new(provider_config.clone(), http_client.clone()))
                }
                _ => {
                    return Err(AppError::ConfigError(
                        format!("Unknown provider type: {}", provider_id)
                    ));
                }
            };

            // 获取此提供商的模型列表并创建映射
            let models = provider_config.models.as_ref()
                .map(|m| m.clone())
                .unwrap_or_else(|| Self::get_default_models(provider_id));

            // 为每个模型创建到提供商的映射
            for model in models {
                model_mapping.insert(model, provider_id.clone());
            }

            providers.insert(provider_id.clone(), provider);
        }

        // 验证至少配置了一个提供商
        if providers.is_empty() {
            return Err(AppError::ConfigError(
                "No providers configured. At least one provider must be configured.".to_string()
            ));
        }

        Ok(Self {
            providers,
            model_mapping,
        })
    }

    /// Create an empty provider registry for testing purposes
    ///
    /// ## 功能说明
    /// 创建一个空的提供商注册表，主要用于测试错误处理场景
    ///
    /// ## 返回值
    /// 返回一个没有任何提供商的注册表实例
    pub fn new_empty() -> Self {
        Self {
            providers: HashMap::new(),
            model_mapping: HashMap::new(),
        }
    }

    /// 根据模型名称获取对应的提供商
    ///
    /// ## 功能说明
    /// 通过模型名称查找并返回能够处理该模型的AI提供商实例
    ///
    /// ## 内部实现逻辑
    /// 1. 首先尝试精确匹配：在模型映射表中查找模型名
    /// 2. 如果精确匹配失败，尝试前缀匹配：检查模型名是否以提供商ID开头
    /// 3. 如果都失败，返回错误并列出所有可用模型
    /// 4. 返回找到的提供商的Arc引用
    ///
    /// ## 参数说明
    /// - `model`: 要查找的模型名称，如"gpt-4"、"claude-3-sonnet"等
    ///
    /// ## 匹配策略
    /// 1. **精确匹配**: 直接在model_mapping中查找
    /// 2. **前缀匹配**: 检查模型名是否以提供商ID开头（如"openai-gpt-4"匹配"openai"提供商）
    ///
    /// ## 执行例子
    /// ```rust
    /// let provider = registry.get_provider_for_model("gpt-4")?;
    /// let response = provider.chat(request).await?;
    /// ```
    ///
    /// ## 返回值
    /// - `Ok(Arc<dyn AIProvider>)`: 找到的提供商实例
    /// - `Err(AppError::ProviderNotFound)`: 未找到支持该模型的提供商
    /// - `Err(AppError::InternalServerError)`: 内部状态不一致错误
    pub fn get_provider_for_model(&self, model: &str) -> Result<Arc<dyn AIProvider + Send + Sync>, AppError> {
        // 首先尝试精确匹配
        if let Some(provider_id) = self.model_mapping.get(model) {
            return self.providers.get(provider_id)
                .cloned()
                .ok_or_else(|| AppError::InternalServerError(
                    format!("Provider {} not found in registry", provider_id)
                ));
        }

        // 尝试前缀匹配进行提供商选择
        for (provider_id, provider) in &self.providers {
            if model.starts_with(provider_id) {
                return Ok(provider.clone());
            }
        }

        // 如果未找到提供商，返回错误并列出可用模型
        let available_models: Vec<String> = self.model_mapping.keys().cloned().collect();
        Err(AppError::ProviderNotFound(
            format!("No provider found for model '{}'. Available models: {}",
                model, available_models.join(", "))
        ))
    }

    /// 获取所有提供商的可用模型列表
    ///
    /// ## 功能说明
    /// 异步获取所有已配置提供商的模型列表，合并为统一的模型信息列表
    ///
    /// ## 内部实现逻辑
    /// 1. 遍历所有已注册的提供商
    /// 2. 异步调用每个提供商的list_models方法
    /// 3. 将成功获取的模型列表合并到结果中
    /// 4. 对于失败的提供商，记录警告但继续处理其他提供商
    /// 5. 返回合并后的完整模型列表
    ///
    /// ## 容错机制
    /// - 单个提供商失败不会影响整体结果
    /// - 失败的提供商会记录警告日志
    /// - 至少返回成功提供商的模型列表
    ///
    /// ## 执行例子
    /// ```rust
    /// let models = registry.list_all_models().await?;
    /// for model in models {
    ///     println!("Model: {} ({})", model.id, model.owned_by);
    /// }
    /// ```
    ///
    /// ## 返回值
    /// - `Ok(Vec<ModelInfo>)`: 所有可用模型的信息列表
    /// - `Err(AppError)`: 极少情况下的系统错误
    pub async fn list_all_models(&self) -> Result<Vec<ModelInfo>, AppError> {
        let mut all_models = Vec::new();

        // 遍历所有提供商获取模型列表
        for provider in self.providers.values() {
            match provider.list_models().await {
                Ok(mut models) => {
                    // 成功获取模型，添加到结果列表
                    all_models.append(&mut models)
                },
                Err(e) => {
                    // 单个提供商失败，记录警告但继续处理
                    tracing::warn!("Failed to get models from provider: {}", e);
                    // 继续处理其他提供商而不是完全失败
                }
            }
        }

        Ok(all_models)
    }

    /// 检查所有提供商的健康状态
    ///
    /// ## 功能说明
    /// 异步检查所有已配置提供商的健康状态，返回每个提供商的详细状态信息
    ///
    /// ## 内部实现逻辑
    /// 1. 遍历所有已注册的提供商
    /// 2. 异步调用每个提供商的health_check方法
    /// 3. 对于成功的健康检查，直接使用返回的状态
    /// 4. 对于失败的健康检查，创建错误状态对象
    /// 5. 将所有结果收集到HashMap中返回
    ///
    /// ## 健康检查内容
    /// - 提供商API的连通性
    /// - 响应延迟测量
    /// - 认证状态验证
    /// - 服务可用性确认
    ///
    /// ## 执行例子
    /// ```rust
    /// let health_results = registry.health_check_all().await;
    /// for (provider_id, status) in health_results {
    ///     println!("{}: {} ({}ms)", provider_id, status.status,
    ///              status.latency_ms.unwrap_or(0));
    /// }
    /// ```
    ///
    /// ## 返回值
    /// - `HashMap<String, HealthStatus>`: 提供商ID到健康状态的映射
    ///   - 键：提供商ID
    ///   - 值：包含状态、延迟、错误信息的HealthStatus对象
    pub async fn health_check_all(&self) -> HashMap<String, HealthStatus> {
        let mut results = HashMap::new();

        // 遍历所有提供商进行健康检查
        for (provider_id, provider) in &self.providers {
            // 执行健康检查，失败时创建错误状态
            let health = provider.health_check().await.unwrap_or_else(|e| HealthStatus {
                status: "error".to_string(),
                provider: provider_id.clone(),
                latency_ms: None,
                error: Some(e.to_string()),
            });
            results.insert(provider_id.clone(), health);
        }

        results
    }

    /// 获取所有已配置的提供商ID列表
    ///
    /// ## 功能说明
    /// 返回当前注册表中所有提供商的ID列表，用于监控和调试
    ///
    /// ## 执行例子
    /// ```rust
    /// let provider_ids = registry.get_provider_ids();
    /// println!("Configured providers: {:?}", provider_ids);
    /// ```
    ///
    /// ## 返回值
    /// - `Vec<String>`: 所有提供商ID的列表
    pub fn get_provider_ids(&self) -> Vec<String> {
        self.providers.keys().cloned().collect()
    }

    /// 获取模型映射表用于调试和监控
    ///
    /// ## 功能说明
    /// 返回模型名到提供商ID的映射关系，用于调试和监控系统状态
    ///
    /// ## 执行例子
    /// ```rust
    /// let mapping = registry.get_model_mapping();
    /// for (model, provider) in mapping {
    ///     println!("Model {} -> Provider {}", model, provider);
    /// }
    /// ```
    ///
    /// ## 返回值
    /// - `&HashMap<String, String>`: 模型名到提供商ID的映射引用
    pub fn get_model_mapping(&self) -> &HashMap<String, String> {
        &self.model_mapping
    }

    /// 根据模型名称获取对应的提供商（返回Option）
    ///
    /// ## 功能说明
    /// 通过模型名称查找并返回能够处理该模型的AI提供商实例，返回Option类型便于测试
    ///
    /// ## 内部实现逻辑
    /// 调用get_provider_for_model方法，将Result转换为Option
    ///
    /// ## 参数说明
    /// - `model`: 要查找的模型名称，如"gpt-4"、"claude-3-sonnet"等
    ///
    /// ## 执行例子
    /// ```rust
    /// if let Some(provider) = registry.get_provider("gpt-4") {
    ///     // 使用provider
    /// }
    /// ```
    ///
    /// ## 返回值
    /// - `Some(Arc<dyn AIProvider>)`: 找到的提供商实例
    /// - `None`: 未找到支持该模型的提供商
    pub fn get_provider(&self, model: &str) -> Option<Arc<dyn AIProvider + Send + Sync>> {
        self.get_provider_for_model(model).ok()
    }

    /// 刷新所有提供商的模型列表并更新模型映射
    ///
    /// ## 功能说明
    /// 从所有已配置的提供商获取最新的模型列表，更新内部模型映射表。
    /// 这对于在不重启服务的情况下获取最新模型信息非常有用。
    ///
    /// ## 内部实现逻辑
    /// 1. 创建新的模型映射表
    /// 2. 遍历所有提供商，异步获取最新模型列表
    /// 3. 将成功获取的模型添加到新映射表中
    /// 4. 对失败的提供商记录警告但继续处理
    /// 5. 用新映射表替换旧的模型映射
    /// 6. 记录刷新完成的日志
    ///
    /// ## 使用场景
    /// - 提供商新增了模型
    /// - 某些模型被废弃或下线
    /// - 定期更新模型列表以保持同步
    ///
    /// ## 执行例子
    /// ```rust
    /// registry.refresh_models().await?;
    /// println!("Models refreshed successfully");
    /// ```
    ///
    /// ## 返回值
    /// - `Ok(())`: 模型刷新成功完成
    /// - `Err(AppError)`: 系统级错误（极少发生）
    pub async fn refresh_models(&mut self) -> Result<(), AppError> {
        let mut new_model_mapping: HashMap<String, String> = HashMap::new();

        // 遍历所有提供商获取最新模型列表
        for (provider_id, provider) in &self.providers {
            match provider.list_models().await {
                Ok(models) => {
                    // 成功获取模型，更新映射表
                    for model in models {
                        new_model_mapping.insert(model.id, provider_id.clone());
                    }
                    tracing::info!("Refreshed models for provider: {}", provider_id);
                }
                Err(e) => {
                    // 单个提供商失败，记录警告但继续处理
                    tracing::warn!("Failed to refresh models for provider {}: {}", provider_id, e);
                    // 继续处理其他提供商而不是完全失败
                }
            }
        }

        // 更新模型映射表
        self.model_mapping = new_model_mapping;
        tracing::info!("Model mapping refreshed successfully");

        Ok(())
    }

    /// 获取当前模型映射的统计信息
    ///
    /// ## 功能说明
    /// 统计每个提供商支持的模型数量，用于监控和调试
    ///
    /// ## 内部实现逻辑
    /// 1. 创建统计结果HashMap
    /// 2. 遍历模型映射表中的所有提供商ID
    /// 3. 为每个提供商计数其支持的模型数量
    /// 4. 返回提供商ID到模型数量的映射
    ///
    /// ## 执行例子
    /// ```rust
    /// let stats = registry.get_model_stats();
    /// for (provider, count) in stats {
    ///     println!("Provider {} supports {} models", provider, count);
    /// }
    /// ```
    ///
    /// ## 返回值
    /// - `HashMap<String, usize>`: 提供商ID到其支持模型数量的映射
    pub fn get_model_stats(&self) -> HashMap<String, usize> {
        let mut stats = HashMap::new();

        // 统计每个提供商的模型数量
        for provider_id in self.model_mapping.values() {
            *stats.entry(provider_id.clone()).or_insert(0) += 1;
        }

        stats
    }

    /// 获取默认模型列表
    fn get_default_models(provider_id: &str) -> Vec<String> {
        match provider_id {
            id if id.starts_with("gemini") => vec![
                "gemini-1.5-pro-latest".to_string(),
                "gemini-1.5-flash-latest".to_string(),
                "gemini-pro".to_string(),
            ],
            id if id.starts_with("openai") => vec![
                "gpt-4".to_string(),
                "gpt-4-turbo-preview".to_string(),
                "gpt-3.5-turbo".to_string(),
                "gpt-3.5-turbo-16k".to_string(),
            ],
            id if id.starts_with("anthropic") => vec![
                "claude-3-opus-20240229".to_string(),
                "claude-3-sonnet-20240229".to_string(),
                "claude-3-haiku-20240307".to_string(),
            ],
            _ => vec![],
        }
    }
}
