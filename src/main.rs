use ai_proxy::{start_server, AppError, Config};
use clap::{Arg, Command};
use std::path::PathBuf;
use tokio::signal;
use tracing_subscriber::{
    fmt::{self, format::FmtSpan},
    layer::SubscriberExt,
    util::SubscriberInitExt,
    EnvFilter,
};

/// 命令行参数结构体
#[derive(Debug)]
struct Args {
    /// 配置文件路径
    config_path: Option<PathBuf>,
    /// 服务器主机地址
    host: Option<String>,
    /// 服务器端口
    port: Option<u16>,
    /// 日志级别
    log_level: Option<String>,
    /// 是否验证配置后退出
    validate_config: bool,
    /// 是否显示版本信息
    version: bool,
}

/// 主函数 - AI代理服务的入口点
/// 
/// 负责解析命令行参数、初始化日志系统、加载配置并启动HTTP服务器
/// 支持优雅关闭和信号处理
#[tokio::main]
async fn main() -> Result<(), AppError> {
    // 解析命令行参数
    let args = parse_args();

    // 如果请求显示版本信息，显示后退出
    if args.version {
        print_version_info();
        return Ok(());
    }

    // 初始化结构化日志系统
    init_tracing(args.log_level.as_deref())?;

    tracing::info!(
        version = env!("CARGO_PKG_VERSION"),
        "AI Proxy service starting up"
    );

    // 加载配置文件和环境变量配置
    let mut config = load_config_with_args(&args)
        .map_err(|e| AppError::ConfigError(format!("加载配置失败: {}", e)))?;

    // 应用命令行参数覆盖配置
    apply_args_to_config(&mut config, &args);

    tracing::info!(
        host = %config.server.host,
        port = %config.server.port,
        providers_count = config.providers.len(),
        config_file = ?args.config_path.as_ref().unwrap_or(&PathBuf::from("config.toml")),
        "Configuration loaded successfully"
    );

    // 如果只是验证配置，验证后退出
    if args.validate_config {
        tracing::info!("Configuration validation successful");
        println!("✓ Configuration is valid");
        return Ok(());
    }

    // 设置优雅关闭处理
    let shutdown_signal = setup_shutdown_signal();

    // 启动HTTP服务器，支持优雅关闭
    tracing::info!("Starting HTTP server with graceful shutdown support");
    
    tokio::select! {
        result = start_server(config) => {
            match result {
                Ok(_) => tracing::info!("Server stopped normally"),
                Err(e) => {
                    tracing::error!(error = %e, "Server stopped with error");
                    return Err(e);
                }
            }
        }
        _ = shutdown_signal => {
            tracing::info!("Shutdown signal received, stopping server gracefully");
        }
    }

    tracing::info!("AI Proxy service shutdown completed");
    Ok(())
}

/// 解析命令行参数
/// 
/// 使用clap库解析命令行参数，支持配置文件路径、服务器设置、日志级别等选项
fn parse_args() -> Args {
    let matches = Command::new("ai-proxy")
        .version(env!("CARGO_PKG_VERSION"))
        .author("AI Proxy Team")
        .about("High-performance AI provider proxy gateway")
        .long_about("AI Proxy is a Rust-based API gateway that unifies multiple AI providers (Gemini, OpenAI, Anthropic, etc.) into a single, consistent interface.")
        .arg(
            Arg::new("config")
                .short('c')
                .long("config")
                .value_name("FILE")
                .help("Configuration file path")
                .long_help("Path to the TOML configuration file. Defaults to 'config.toml' in the current directory.")
        )
        .arg(
            Arg::new("host")
                .short('H')
                .long("host")
                .value_name("HOST")
                .help("Server host address")
                .long_help("Override the server host address from configuration file")
        )
        .arg(
            Arg::new("port")
                .short('p')
                .long("port")
                .value_name("PORT")
                .help("Server port number")
                .long_help("Override the server port number from configuration file")
                .value_parser(clap::value_parser!(u16))
        )
        .arg(
            Arg::new("log-level")
                .short('l')
                .long("log-level")
                .value_name("LEVEL")
                .help("Log level")
                .long_help("Set the logging level (trace, debug, info, warn, error)")
                .value_parser(["trace", "debug", "info", "warn", "error"])
        )
        .arg(
            Arg::new("validate")
                .long("validate-config")
                .help("Validate configuration and exit")
                .long_help("Load and validate the configuration file, then exit without starting the server")
                .action(clap::ArgAction::SetTrue)
        )
        .arg(
            Arg::new("version")
                .short('V')
                .long("version")
                .help("Show version information")
                .long_help("Display detailed version information and exit")
                .action(clap::ArgAction::SetTrue)
        )
        .get_matches();

    Args {
        config_path: matches.get_one::<String>("config").map(PathBuf::from),
        host: matches.get_one::<String>("host").cloned(),
        port: matches.get_one::<u16>("port").copied(),
        log_level: matches.get_one::<String>("log-level").cloned(),
        validate_config: matches.get_flag("validate"),
        version: matches.get_flag("version"),
    }
}

/// 显示版本信息
/// 
/// 显示详细的版本信息，包括构建信息和系统信息
fn print_version_info() {
    println!("AI Proxy v{}", env!("CARGO_PKG_VERSION"));
    println!("A high-performance AI provider proxy gateway written in Rust");
    println!();
    println!("Build Information:");
    println!("  Version: {}", env!("CARGO_PKG_VERSION"));
    println!("  Profile: {}", if cfg!(debug_assertions) { "debug" } else { "release" });
    println!("  Architecture: {}", std::env::consts::ARCH);
    println!("  OS: {}", std::env::consts::OS);
    println!();
    println!("Features:");
    println!("  • Multiple AI provider support (OpenAI, Anthropic, Gemini)");
    println!("  • Unified API interface with Anthropic format");
    println!("  • Real-time streaming responses");
    println!("  • High-performance async processing");
    println!("  • Comprehensive monitoring and logging");
    println!("  • Docker and Kubernetes ready");
}

/// 根据命令行参数加载配置
/// 
/// 支持自定义配置文件路径，如果未指定则使用默认的config.toml
fn load_config_with_args(args: &Args) -> anyhow::Result<Config> {
    use figment::{Figment, providers::{Format, Toml, Env}};

    let config_path = args.config_path
        .as_ref()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|| "config.toml".to_string());

    // 创建配置加载器，按优先级合并配置源
    let config: Config = Figment::new()
        .merge(Toml::file(&config_path))  // 配置文件
        .merge(Env::prefixed("AI_PROXY_"))  // 环境变量覆盖
        .extract()
        .map_err(|e| anyhow::anyhow!("Failed to load configuration from {} or environment variables: {}", config_path, e))?;

    // 验证加载的配置是否有效
    config.validate()
        .map_err(|e| anyhow::anyhow!("Configuration validation failed: {}", e))?;

    Ok(config)
}

/// 应用命令行参数到配置
/// 
/// 命令行参数具有最高优先级，会覆盖配置文件和环境变量中的相同设置
fn apply_args_to_config(config: &mut Config, args: &Args) {
    if let Some(host) = &args.host {
        tracing::info!(old_host = %config.server.host, new_host = %host, "Overriding server host from command line");
        config.server.host = host.clone();
    }

    if let Some(port) = args.port {
        tracing::info!(old_port = config.server.port, new_port = port, "Overriding server port from command line");
        config.server.port = port;
    }
}

/// 设置优雅关闭信号处理
/// 
/// 监听SIGINT (Ctrl+C) 和SIGTERM信号，支持优雅关闭
async fn setup_shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("Failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("Failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {
            tracing::info!("Received SIGINT (Ctrl+C)");
        },
        _ = terminate => {
            tracing::info!("Received SIGTERM");
        },
    }
}

/// 初始化结构化日志系统
/// 
/// 配置tracing和tracing-subscriber，支持：
/// - 结构化JSON日志输出
/// - 环境变量和命令行参数控制日志级别
/// - 请求ID传播和追踪
/// - 详细的请求/响应日志记录
fn init_tracing(log_level_override: Option<&str>) -> Result<(), AppError> {
    // 确定日志级别优先级：命令行参数 > 环境变量 > 默认值
    let env_filter = if let Some(level) = log_level_override {
        EnvFilter::new(format!("ai_proxy={},tower_http=debug", level))
    } else {
        EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| EnvFilter::new("ai_proxy=info,tower_http=debug"))
    };

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
