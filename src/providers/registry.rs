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
    /// Create a new provider registry from configuration
    pub fn new(config: &Config, http_client: Client) -> Result<Self, AppError> {
        let mut providers: HashMap<String, Arc<dyn AIProvider + Send + Sync>> = HashMap::new();
        let mut model_mapping: HashMap<String, String> = HashMap::new();

        // Initialize providers based on configuration
        for (provider_id, provider_config) in &config.providers {
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

            // Get models for this provider and create mappings
            let models = provider_config.models.as_ref()
                .map(|m| m.clone())
                .unwrap_or_else(|| Self::get_default_models(provider_id));

            for model in models {
                model_mapping.insert(model, provider_id.clone());
            }

            providers.insert(provider_id.clone(), provider);
        }

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

    /// Get provider by model name
    pub fn get_provider_for_model(&self, model: &str) -> Result<Arc<dyn AIProvider + Send + Sync>, AppError> {
        // First try exact match
        if let Some(provider_id) = self.model_mapping.get(model) {
            return self.providers.get(provider_id)
                .cloned()
                .ok_or_else(|| AppError::InternalServerError(
                    format!("Provider {} not found in registry", provider_id)
                ));
        }

        // Try prefix matching for provider selection
        for (provider_id, provider) in &self.providers {
            if model.starts_with(provider_id) {
                return Ok(provider.clone());
            }
        }

        // If no provider found, return error with available models
        let available_models: Vec<String> = self.model_mapping.keys().cloned().collect();
        Err(AppError::ProviderNotFound(
            format!("No provider found for model '{}'. Available models: {}", 
                model, available_models.join(", "))
        ))
    }

    /// Get all available models from all providers
    pub async fn list_all_models(&self) -> Result<Vec<ModelInfo>, AppError> {
        let mut all_models = Vec::new();

        for provider in self.providers.values() {
            match provider.list_models().await {
                Ok(mut models) => all_models.append(&mut models),
                Err(e) => {
                    tracing::warn!("Failed to get models from provider: {}", e);
                    // Continue with other providers instead of failing completely
                }
            }
        }

        Ok(all_models)
    }

    /// Check health of all providers
    pub async fn health_check_all(&self) -> HashMap<String, HealthStatus> {
        let mut results = HashMap::new();

        for (provider_id, provider) in &self.providers {
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

    /// Get all configured provider IDs
    pub fn get_provider_ids(&self) -> Vec<String> {
        self.providers.keys().cloned().collect()
    }

    /// Get model mapping for debugging/monitoring
    pub fn get_model_mapping(&self) -> &HashMap<String, String> {
        &self.model_mapping
    }

    /// Refresh models from all providers and update the model mapping
    ///
    /// This method fetches the latest model lists from all configured providers
    /// and updates the internal model mapping. This is useful for getting the
    /// most up-to-date model information without restarting the service.
    pub async fn refresh_models(&mut self) -> Result<(), AppError> {
        let mut new_model_mapping: HashMap<String, String> = HashMap::new();

        for (provider_id, provider) in &self.providers {
            match provider.list_models().await {
                Ok(models) => {
                    for model in models {
                        new_model_mapping.insert(model.id, provider_id.clone());
                    }
                    tracing::info!("Refreshed models for provider: {}", provider_id);
                }
                Err(e) => {
                    tracing::warn!("Failed to refresh models for provider {}: {}", provider_id, e);
                    // Continue with other providers instead of failing completely
                }
            }
        }

        // Update the model mapping
        self.model_mapping = new_model_mapping;
        tracing::info!("Model mapping refreshed successfully");

        Ok(())
    }

    /// Get statistics about the current model mapping
    pub fn get_model_stats(&self) -> HashMap<String, usize> {
        let mut stats = HashMap::new();

        for provider_id in self.model_mapping.values() {
            *stats.entry(provider_id.clone()).or_insert(0) += 1;
        }

        stats
    }

    /// Get default models for a provider type
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
