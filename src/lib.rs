/// AI代理服务的主库模块
/// 
/// 提供统一的AI服务代理功能，支持多个AI提供商（OpenAI、Anthropic、Gemini等）
/// 通过标准化的API接口提供聊天完成、模型管理等功能

pub mod config;      // 配置管理模块
pub mod errors;      // 错误处理模块
pub mod providers;   // AI提供商模块
pub mod server;      // HTTP服务器模块

// 重新导出常用类型，方便外部使用
pub use config::{Config, load_config};
pub use errors::{AppError, AppResult};
pub use server::{AppState, start_server};
