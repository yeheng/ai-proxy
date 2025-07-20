use serde::Serialize;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;
use tokio::sync::RwLock;

/// 系统指标收集器
///
/// 负责收集和管理系统运行时的各种指标，包括请求计数、延迟、错误率、并发请求等
#[derive(Debug, Clone)]
pub struct MetricsCollector {
    /// 请求计数器
    request_count: Arc<AtomicU64>,
    /// 成功请求计数器
    success_count: Arc<AtomicU64>,
    /// 错误请求计数器
    error_count: Arc<AtomicU64>,
    /// 当前并发请求数
    concurrent_requests: Arc<AtomicU64>,
    /// 最大并发请求数
    max_concurrent_requests: Arc<AtomicU64>,
    /// 延迟统计信息
    latency_stats: Arc<RwLock<LatencyStats>>,
    /// 按提供商分组的指标
    provider_metrics: Arc<RwLock<HashMap<String, ProviderMetrics>>>,
    /// 按模型分组的指标
    model_metrics: Arc<RwLock<HashMap<String, ModelMetrics>>>,
    /// 系统启动时间
    start_time: Instant,
}

/// 延迟统计信息
#[derive(Debug, Clone, Serialize)]
pub struct LatencyStats {
    /// 总延迟时间（毫秒）
    pub total_latency_ms: u64,
    /// 最小延迟（毫秒）
    pub min_latency_ms: u64,
    /// 最大延迟（毫秒）
    pub max_latency_ms: u64,
    /// 请求数量
    pub request_count: u64,
}

/// 提供商指标
#[derive(Debug, Clone, Serialize)]
pub struct ProviderMetrics {
    /// 请求总数
    pub total_requests: u64,
    /// 成功请求数
    pub successful_requests: u64,
    /// 失败请求数
    pub failed_requests: u64,
    /// 平均延迟（毫秒）
    pub avg_latency_ms: f64,
    /// 最后一次请求时间
    pub last_request_time: Option<String>,
}

/// 模型指标
#[derive(Debug, Clone, Serialize)]
pub struct ModelMetrics {
    /// 请求总数
    pub total_requests: u64,
    /// 成功请求数
    pub successful_requests: u64,
    /// 失败请求数
    pub failed_requests: u64,
    /// 平均延迟（毫秒）
    pub avg_latency_ms: f64,
}

/// 系统指标摘要
#[derive(Debug, Serialize)]
pub struct MetricsSummary {
    /// 系统运行时间（秒）
    pub uptime_seconds: u64,
    /// 总请求数
    pub total_requests: u64,
    /// 成功请求数
    pub successful_requests: u64,
    /// 失败请求数
    pub failed_requests: u64,
    /// 成功率（百分比）
    pub success_rate_percent: f64,
    /// 错误率（百分比）
    pub error_rate_percent: f64,
    /// 平均延迟（毫秒）
    pub avg_latency_ms: f64,
    /// 当前并发请求数
    pub current_concurrent_requests: u64,
    /// 最大并发请求数
    pub max_concurrent_requests: u64,
    /// 延迟统计
    pub latency_stats: LatencyStats,
    /// 按提供商分组的指标
    pub provider_metrics: HashMap<String, ProviderMetrics>,
    /// 按模型分组的指标
    pub model_metrics: HashMap<String, ModelMetrics>,
    /// 指标收集时间戳
    pub timestamp: String,
}

impl Default for LatencyStats {
    fn default() -> Self {
        Self {
            total_latency_ms: 0,
            min_latency_ms: u64::MAX,
            max_latency_ms: 0,
            request_count: 0,
        }
    }
}

impl Default for ProviderMetrics {
    fn default() -> Self {
        Self {
            total_requests: 0,
            successful_requests: 0,
            failed_requests: 0,
            avg_latency_ms: 0.0,
            last_request_time: None,
        }
    }
}

impl Default for ModelMetrics {
    fn default() -> Self {
        Self {
            total_requests: 0,
            successful_requests: 0,
            failed_requests: 0,
            avg_latency_ms: 0.0,
        }
    }
}

impl MetricsCollector {
    /// 创建新的指标收集器
    ///
    /// ## 功能说明
    /// 初始化指标收集器，设置所有计数器为零，记录系统启动时间
    ///
    /// ## 执行例子
    /// ```rust
    /// let metrics = MetricsCollector::new();
    /// ```
    pub fn new() -> Self {
        Self {
            request_count: Arc::new(AtomicU64::new(0)),
            success_count: Arc::new(AtomicU64::new(0)),
            error_count: Arc::new(AtomicU64::new(0)),
            concurrent_requests: Arc::new(AtomicU64::new(0)),
            max_concurrent_requests: Arc::new(AtomicU64::new(0)),
            latency_stats: Arc::new(RwLock::new(LatencyStats::default())),
            provider_metrics: Arc::new(RwLock::new(HashMap::new())),
            model_metrics: Arc::new(RwLock::new(HashMap::new())),
            start_time: Instant::now(),
        }
    }

    /// 增加并发请求计数
    ///
    /// ## 功能说明
    /// 增加当前并发请求数，并更新最大并发请求数记录
    ///
    /// ## 执行例子
    /// ```rust
    /// metrics.increment_concurrent_requests().await;
    /// ```
    pub async fn increment_concurrent_requests(&self) {
        let current = self.concurrent_requests.fetch_add(1, Ordering::Relaxed) + 1;

        // 更新最大并发请求数
        let mut max = self.max_concurrent_requests.load(Ordering::Relaxed);
        while current > max {
            match self.max_concurrent_requests.compare_exchange_weak(
                max,
                current,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => break,
                Err(x) => max = x,
            }
        }
    }

    /// 减少并发请求计数
    ///
    /// ## 功能说明
    /// 减少当前并发请求数，通常在请求完成时调用
    ///
    /// ## 执行例子
    /// ```rust
    /// metrics.decrement_concurrent_requests().await;
    /// ```
    pub async fn decrement_concurrent_requests(&self) {
        self.concurrent_requests.fetch_sub(1, Ordering::Relaxed);
    }

    /// 获取当前并发请求数
    ///
    /// ## 功能说明
    /// 返回当前正在处理的并发请求数量
    ///
    /// ## 执行例子
    /// ```rust
    /// let concurrent = metrics.get_concurrent_requests();
    /// ```
    ///
    /// ## 返回值
    /// - `u64`: 当前并发请求数
    pub fn get_concurrent_requests(&self) -> u64 {
        self.concurrent_requests.load(Ordering::Relaxed)
    }

    /// 记录请求开始
    ///
    /// ## 功能说明
    /// 增加总请求计数，返回请求开始时间用于后续延迟计算
    ///
    /// ## 执行例子
    /// ```rust
    /// let start_time = metrics.record_request_start();
    /// // ... 处理请求 ...
    /// metrics.record_request_end(start_time, true, "openai", "gpt-4").await;
    /// ```
    ///
    /// ## 返回值
    /// - `Instant`: 请求开始时间，用于延迟计算
    pub fn record_request_start(&self) -> Instant {
        self.request_count.fetch_add(1, Ordering::Relaxed);
        Instant::now()
    }

    /// 记录请求结束
    ///
    /// ## 功能说明
    /// 记录请求完成，更新成功/失败计数、延迟统计和提供商/模型指标
    ///
    /// ## 参数说明
    /// - `start_time`: 请求开始时间，用于计算延迟
    /// - `success`: 请求是否成功
    /// - `provider`: 处理请求的提供商ID
    /// - `model`: 使用的模型名称
    ///
    /// ## 执行例子
    /// ```rust
    /// let start_time = metrics.record_request_start();
    /// // ... 处理请求 ...
    /// metrics.record_request_end(start_time, true, "openai", "gpt-4").await;
    /// ```
    pub async fn record_request_end(
        &self,
        start_time: Instant,
        success: bool,
        provider: &str,
        model: &str,
    ) {
        let latency = start_time.elapsed();
        let latency_ms = latency.as_millis() as u64;

        // 更新成功/失败计数
        if success {
            self.success_count.fetch_add(1, Ordering::Relaxed);
        } else {
            self.error_count.fetch_add(1, Ordering::Relaxed);
        }

        // 更新延迟统计
        {
            let mut stats = self.latency_stats.write().await;
            stats.total_latency_ms += latency_ms;
            stats.request_count += 1;
            stats.min_latency_ms = stats.min_latency_ms.min(latency_ms);
            stats.max_latency_ms = stats.max_latency_ms.max(latency_ms);
        }

        // 更新提供商指标
        {
            let mut provider_metrics = self.provider_metrics.write().await;
            let metrics = provider_metrics.entry(provider.to_string()).or_default();
            metrics.total_requests += 1;
            if success {
                metrics.successful_requests += 1;
            } else {
                metrics.failed_requests += 1;
            }

            // 更新平均延迟
            let total_latency =
                (metrics.avg_latency_ms * (metrics.total_requests - 1) as f64) + latency_ms as f64;
            metrics.avg_latency_ms = total_latency / metrics.total_requests as f64;
            metrics.last_request_time = Some(chrono::Utc::now().to_rfc3339());
        }

        // 更新模型指标
        {
            let mut model_metrics = self.model_metrics.write().await;
            let metrics = model_metrics.entry(model.to_string()).or_default();
            metrics.total_requests += 1;
            if success {
                metrics.successful_requests += 1;
            } else {
                metrics.failed_requests += 1;
            }

            // 更新平均延迟
            let total_latency =
                (metrics.avg_latency_ms * (metrics.total_requests - 1) as f64) + latency_ms as f64;
            metrics.avg_latency_ms = total_latency / metrics.total_requests as f64;
        }
    }

    /// 获取系统指标摘要
    ///
    /// ## 功能说明
    /// 收集并返回系统的完整指标摘要，包括请求统计、延迟信息、提供商和模型指标
    ///
    /// ## 执行例子
    /// ```rust
    /// let summary = metrics.get_metrics_summary().await;
    /// println!("Success rate: {:.2}%", summary.success_rate_percent);
    /// ```
    ///
    /// ## 返回值
    /// - `MetricsSummary`: 包含所有系统指标的摘要对象
    pub async fn get_metrics_summary(&self) -> MetricsSummary {
        let total_requests = self.request_count.load(Ordering::Relaxed);
        let successful_requests = self.success_count.load(Ordering::Relaxed);
        let failed_requests = self.error_count.load(Ordering::Relaxed);

        let success_rate_percent = if total_requests > 0 {
            (successful_requests as f64 / total_requests as f64) * 100.0
        } else {
            0.0
        };

        let error_rate_percent = if total_requests > 0 {
            (failed_requests as f64 / total_requests as f64) * 100.0
        } else {
            0.0
        };

        let latency_stats = self.latency_stats.read().await.clone();
        let avg_latency_ms = if latency_stats.request_count > 0 {
            latency_stats.total_latency_ms as f64 / latency_stats.request_count as f64
        } else {
            0.0
        };

        let provider_metrics = self.provider_metrics.read().await.clone();
        let model_metrics = self.model_metrics.read().await.clone();

        MetricsSummary {
            uptime_seconds: self.start_time.elapsed().as_secs(),
            total_requests,
            successful_requests,
            failed_requests,
            success_rate_percent,
            error_rate_percent,
            avg_latency_ms,
            current_concurrent_requests: self.concurrent_requests.load(Ordering::Relaxed),
            max_concurrent_requests: self.max_concurrent_requests.load(Ordering::Relaxed),
            latency_stats,
            provider_metrics,
            model_metrics,
            timestamp: chrono::Utc::now().to_rfc3339(),
        }
    }

    /// 重置所有指标
    ///
    /// ## 功能说明
    /// 将所有指标重置为初始状态，通常用于测试或系统重启后的指标清理
    ///
    /// ## 执行例子
    /// ```rust
    /// metrics.reset_metrics().await;
    /// ```
    pub async fn reset_metrics(&self) {
        self.request_count.store(0, Ordering::Relaxed);
        self.success_count.store(0, Ordering::Relaxed);
        self.error_count.store(0, Ordering::Relaxed);
        self.concurrent_requests.store(0, Ordering::Relaxed);
        self.max_concurrent_requests.store(0, Ordering::Relaxed);

        *self.latency_stats.write().await = LatencyStats::default();
        self.provider_metrics.write().await.clear();
        self.model_metrics.write().await.clear();
    }

    /// 获取基本指标（用于快速检查）
    ///
    /// ## 功能说明
    /// 返回基本的请求统计信息，不包括详细的提供商和模型指标
    ///
    /// ## 执行例子
    /// ```rust
    /// let (total, success, errors) = metrics.get_basic_stats();
    /// println!("Requests: {}, Success: {}, Errors: {}", total, success, errors);
    /// ```
    ///
    /// ## 返回值
    /// - `(u64, u64, u64)`: (总请求数, 成功请求数, 失败请求数)
    pub fn get_basic_stats(&self) -> (u64, u64, u64) {
        (
            self.request_count.load(Ordering::Relaxed),
            self.success_count.load(Ordering::Relaxed),
            self.error_count.load(Ordering::Relaxed),
        )
    }
}

/// 指标中间件，用于自动收集HTTP请求指标
///
/// ## 功能说明
/// 这是一个Axum中间件，自动为所有HTTP请求收集指标信息
///
/// ## 使用方法
/// ```rust
/// let app = Router::new()
///     .route("/api", get(handler))
///     .layer(MetricsMiddleware::new(metrics_collector));
/// ```
pub struct MetricsMiddleware {
    metrics: Arc<MetricsCollector>,
}

impl MetricsMiddleware {
    /// 创建新的指标中间件
    pub fn new(metrics: Arc<MetricsCollector>) -> Self {
        Self { metrics }
    }

    /// 获取指标收集器的引用
    pub fn metrics(&self) -> &Arc<MetricsCollector> {
        &self.metrics
    }
}

impl Clone for MetricsMiddleware {
    fn clone(&self) -> Self {
        Self {
            metrics: Arc::clone(&self.metrics),
        }
    }
}
