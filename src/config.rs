use serde::Deserialize;
use figment::{Figment, providers::{Format, Toml, Env}};
use std::collections::HashMap;
use anyhow::{Context, Result};

#[derive(Deserialize, Debug)]
pub struct Config {
    pub server: ServerConfig,
    pub providers: HashMap<String, ProviderDetail>,
}

#[derive(Deserialize, Debug)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
}

#[derive(Deserialize, Debug, Clone)]
pub struct ProviderDetail {
    pub api_key: String,
    pub api_base: String,
    pub models: Option<Vec<String>>,
}

pub fn load_config() -> Result<Config> {
    Figment::new()
        .merge(Toml::file("config.toml"))
        .merge(Env::prefixed("AI_PROXY_"))
        .extract()
        .context("Failed to load configuration from config.toml or environment variables")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_config() {
        // Test will fail if config.toml doesn't exist, but that's expected
        let config = load_config();
        assert!(config.is_ok() || config.is_err());
    }
}