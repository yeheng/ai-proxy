# AI Proxy 核心功能设计文档

## 概述

AI Proxy 系统采用分层架构和适配器模式，通过统一的 API 接口为多个 AI 提供商提供代理服务。系统基于 Rust 的异步生态系统构建，使用 Tokio 运行时和 Axum Web 框架，确保高性能和高并发处理能力。

核心设计原则：

- **统一接口**：所有提供商都通过相同的 API 格式访问
- **可扩展性**：新提供商可以通过实现标准接口轻松添加
- **高性能**：异步处理和连接池确保高吞吐量
- **容错性**：完善的错误处理和优雅降级机制

## 架构

### 系统架构图

```
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│                 │    │                 │    │                 │
│   Client App    │───▶│   AI Proxy      │───▶│   AI Provider   │
│                 │    │   (Gateway)     │    │   (Gemini,      │
│                 │    │                 │    │   OpenAI, etc.) │
└─────────────────┘    │                 │    └─────────────────┘
                       │                 │
                       │  ┌─────────────┐│
                       │  │ Config Mgr  ││
                       │  └─────────────┘│
                       │  ┌─────────────┐│
                       │  │ Provider    ││
                       │  │ Registry    ││
                       │  └─────────────┘│
                       │  ┌─────────────┐│
                       │  │ Error       ││
                       │  │ Handler     ││
                       │  └─────────────┘│
                       └─────────────────┘
```

### 分层架构

1. **表示层 (Presentation Layer)**
   - HTTP 路由和请求处理
   - 中间件（日志、认证、错误处理）
   - 响应格式化

2. **业务逻辑层 (Business Logic Layer)**
   - 提供商选择和路由
   - 请求/响应转换
   - 流式处理协调

3. **适配器层 (Adapter Layer)**
   - 提供商特定的 API 适配
   - 协议转换
   - 错误映射

4. **基础设施层 (Infrastructure Layer)**
   - HTTP 客户端管理
   - 配置管理
   - 日志和监控

## 组件和接口

### 核心组件

#### 1. AppState - 应用状态管理

```rust
pub struct AppState {
    pub config: Arc<Config>,
    pub http_client: Client,
    pub provider_registry: Arc<ProviderRegistry>,
}
```

**职责：**

- 维护全局应用状态
- 管理共享资源（HTTP 客户端、配置）
- 提供提供商注册表访问

#### 2. ProviderRegistry - 提供商注册表

```rust
pub struct ProviderRegistry {
    providers: HashMap<String, Arc<dyn AIProvider + Send + Sync>>,
    model_mapping: HashMap<String, String>, // model -> provider_id
}
```

**职责：**

- 动态注册和管理提供商实例
- 根据模型名称路由到正确的提供商
- 支持运行时提供商配置更新

#### 3. AIProvider Trait - 提供商接口

```rust
#[async_trait]
pub trait AIProvider: Send + Sync {
    async fn chat(&self, request: AnthropicRequest) -> Result<AnthropicResponse, AppError>;
    async fn chat_stream(&self, request: AnthropicRequest) -> Result<StreamResponse, AppError>;
    async fn list_models(&self) -> Result<Vec<ModelInfo>, AppError>;
    async fn health_check(&self) -> Result<HealthStatus, AppError>;
}
```

**职责：**

- 定义所有提供商必须实现的标准接口
- 支持同步和异步聊天请求
- 提供模型列表和健康检查功能

#### 4. RequestHandler - 请求处理器

```rust
pub struct RequestHandler {
    provider_registry: Arc<ProviderRegistry>,
}
```

**职责：**

- 处理入站 HTTP 请求
- 协调提供商选择和调用
- 管理流式和非流式响应

### 接口设计

#### HTTP API 接口

1. **聊天完成接口**
   - `POST /v1/messages` - 创建聊天完成
   - 支持流式和非流式响应
   - 统一的 Anthropic API 格式

2. **模型管理接口**
   - `GET /v1/models` - 获取可用模型列表
   - 返回所有已配置提供商的模型信息

3. **健康检查接口**
   - `GET /health` - 系统健康状态
   - `GET /health/providers` - 提供商健康状态

#### 内部接口

1. **提供商适配器接口**
   - 标准化的请求/响应转换
   - 错误处理和重试机制
   - 流式数据处理

2. **配置管理接口**
   - 配置加载和验证
   - 环境变量覆盖
   - 配置热重载支持

## 数据模型

### 请求/响应模型

#### AnthropicRequest - 统一请求格式

```rust
#[derive(Serialize, Deserialize, Debug)]
pub struct AnthropicRequest {
    pub model: String,
    pub messages: Vec<Message>,
    pub max_tokens: u32,
    pub stream: Option<bool>,
    pub temperature: Option<f32>,
    pub top_p: Option<f32>,
}
```

#### AnthropicResponse - 统一响应格式

```rust
#[derive(Serialize, Deserialize, Debug)]
pub struct AnthropicResponse {
    pub id: String,
    pub model: String,
    pub content: Vec<ContentBlock>,
    pub usage: Usage,
}
```

### 配置模型

#### Config - 系统配置

```rust
#[derive(Deserialize, Debug)]
pub struct Config {
    pub server: ServerConfig,
    pub providers: HashMap<String, ProviderDetail>,
    pub logging: LoggingConfig,
    pub monitoring: MonitoringConfig,
}
```

### 提供商特定模型

每个提供商都有自己的内部数据模型，通过适配器转换为统一格式：

1. **GeminiModels** - Google Gemini API 模型
2. **OpenAIModels** - OpenAI API 模型  
3. **AnthropicModels** - Anthropic API 模型（原生格式）

## 错误处理

### 错误分类

#### AppError - 应用级错误

```rust
#[derive(Error, Debug)]
pub enum AppError {
    #[error("Bad request: {0}")]
    BadRequest(String),
    
    #[error("Provider not found: {0}")]
    ProviderNotFound(String),
    
    #[error("Provider error: {message}")]
    ProviderError { status: u16, message: String },
    
    #[error("Internal server error: {0}")]
    InternalServerError(String),
    
    #[error("Configuration error: {0}")]
    ConfigError(String),
    
    #[error("Request validation failed: {0}")]
    ValidationError(String),
}
```

### 错误处理策略

1. **分层错误处理**
   - 内部操作使用 `anyhow::Result` 提供丰富的错误上下文
   - API 响应使用 `AppError` 确保类型安全和一致性

2. **错误转换**
   - 提供商特定错误自动转换为统一格式
   - 敏感信息过滤，避免泄露内部实现细节

3. **错误恢复**
   - 优雅降级机制
   - 重试逻辑（针对临时性错误）
   - 断路器模式（防止级联失败）

## 测试策略

### 测试层次

#### 1. 单元测试

- **配置加载测试**：验证配置解析和验证逻辑
- **错误处理测试**：确保错误正确分类和转换
- **数据转换测试**：验证请求/响应格式转换

#### 2. 集成测试

- **提供商适配器测试**：使用 mock 服务器测试 API 集成
- **端到端流程测试**：完整的请求处理流程
- **流式响应测试**：验证 SSE 流式处理

#### 3. 性能测试

- **并发处理测试**：验证高并发场景下的系统稳定性
- **内存使用测试**：确保流式处理不会导致内存泄漏
- **延迟测试**：测量请求处理延迟

### 测试工具和框架

1. **Mock 服务器**：使用 `wiremock-rs` 模拟外部 API
2. **负载测试**：使用 `criterion` 进行性能基准测试
3. **集成测试**：使用 `tokio-test` 进行异步测试

### 测试数据管理

- **测试配置**：独立的测试配置文件
- **Mock 响应**：预定义的 API 响应模板
- **测试用例**：覆盖正常和异常场景的测试数据集

## 性能考虑

### 异步处理

1. **非阻塞 I/O**
   - 所有网络操作使用异步 I/O
   - 避免阻塞操作影响整体性能

2. **连接池管理**
   - HTTP 客户端连接复用
   - 合理的连接池大小配置

3. **流式处理**
   - 使用 `futures::Stream` 处理大型响应
   - 避免将完整响应加载到内存

### 内存管理

1. **零拷贝优化**
   - 尽可能避免不必要的数据复制
   - 使用引用和借用减少内存分配

2. **缓存策略**
   - 配置信息缓存
   - 提供商实例复用

### 并发控制

1. **背压处理**
   - 合理的请求队列大小
   - 优雅的过载保护机制

2. **资源限制**
   - 每个提供商的并发请求限制
   - 全局资源使用监控

## 安全考虑

### API 密钥管理

1. **安全存储**
   - 配置文件中的 API 密钥加密存储
   - 环境变量优先级高于配置文件

2. **密钥轮换**
   - 支持运行时密钥更新
   - 密钥失效检测和告警

### 输入验证

1. **请求验证**
   - 严格的输入参数验证
   - SQL 注入和 XSS 防护

2. **速率限制**
   - 基于 IP 和 API 密钥的速率限制
   - 防止 DDoS 攻击

### 数据保护

1. **日志安全**
   - 敏感信息过滤
   - 日志访问控制

2. **传输安全**
   - 强制 HTTPS 通信
   - TLS 证书验证

## 部署和运维

### 容器化部署

1. **Docker 镜像**
   - 多阶段构建优化镜像大小
   - 安全基础镜像选择

2. **Kubernetes 部署**
   - 水平扩展支持
   - 健康检查和自动恢复

### 监控和告警

1. **指标收集**
   - Prometheus 指标导出
   - 关键业务指标监控

2. **日志聚合**
   - 结构化日志输出
   - 集中式日志收集和分析

3. **分布式追踪**
   - OpenTelemetry 集成
   - 请求链路追踪

### 配置管理

1. **环境配置**
   - 开发、测试、生产环境隔离
   - 配置版本控制

2. **动态配置**
   - 配置热重载
   - 配置变更审计
