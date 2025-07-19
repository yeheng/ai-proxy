use ai_proxy::{load_config, start_server, AppError};
use tracing_subscriber::{
    fmt::{self, format::FmtSpan},
    layer::SubscriberExt,
    util::SubscriberInitExt,
    EnvFilter,
};

/// 主函数 - AI代理服务的入口点
/// 
/// 负责初始化日志系统、加载配置并启动HTTP服务器
#[tokio::main]
async fn main() -> Result<(), AppError> {
    // 初始化结构化日志系统
    init_tracing()?;

    tracing::info!("AI Proxy service starting up");

    // 加载配置文件（config.toml）和环境变量配置
    let config = load_config()
        .map_err(|e| AppError::ConfigError(format!("加载配置失败: {}", e)))?;

    tracing::info!(
        host = %config.server.host,
        port = %config.server.port,
        providers_count = config.providers.len(),
        "Configuration loaded successfully"
    );

    // 启动HTTP服务器，监听指定地址和端口
    start_server(config).await?;

    Ok(())
}

/// 初始化结构化日志系统
/// 
/// 配置tracing和tracing-subscriber，支持：
/// - 结构化JSON日志输出
/// - 环境变量控制日志级别
/// - 请求ID传播和追踪
/// - 详细的请求/响应日志记录
fn init_tracing() -> Result<(), AppError> {
    // 从环境变量获取日志级别，默认为info
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("ai_proxy=info,tower_http=debug"));

    // 配置日志格式
    let fmt_layer = fmt::layer()
        .with_target(true)  // 显示模块路径
        .with_thread_ids(true)  // 显示线程ID
        .with_thread_names(true)  // 显示线程名称
        .with_file(true)  // 显示文件名
        .with_line_number(true)  // 显示行号
        .with_span_events(FmtSpan::NEW | FmtSpan::CLOSE)  // 显示span事件
        .json();  // 使用JSON格式

    // 初始化全局subscriber
    tracing_subscriber::registry()
        .with(env_filter)
        .with(fmt_layer)
        .try_init()
        .map_err(|e| AppError::ConfigError(format!("Failed to initialize tracing: {}", e)))?;

    tracing::info!("Structured logging system initialized");
    Ok(())
}
