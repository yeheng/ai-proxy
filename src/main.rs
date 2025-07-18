use ai_proxy::{load_config, start_server, AppError};

/// 主函数 - AI代理服务的入口点
/// 
/// 负责加载配置并启动HTTP服务器
#[tokio::main]
async fn main() -> Result<(), AppError> {
    // 加载配置文件（config.toml）和环境变量配置
    let config = load_config()
        .map_err(|e| AppError::ConfigError(format!("加载配置失败: {}", e)))?;

    // 启动HTTP服务器，监听指定地址和端口
    start_server(config).await?;

    Ok(())
}
