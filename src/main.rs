use ai_proxy::{load_config, start_server, AppError};

#[tokio::main]
async fn main() -> Result<(), AppError> {
    // Load configuration
    let config = load_config()
        .map_err(|e| AppError::ConfigError(format!("Failed to load configuration: {}", e)))?;

    // Start the server
    start_server(config).await?;

    Ok(())
}
