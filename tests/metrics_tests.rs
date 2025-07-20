use ai_proxy::metrics::MetricsCollector;
use std::time::Duration;

#[test]
fn test_metrics_creation() {
    let metrics = MetricsCollector::new();

    // Test initial state
    let (total, success, errors) = metrics.get_basic_stats();
    assert_eq!(total, 0);
    assert_eq!(success, 0);
    assert_eq!(errors, 0);
}

#[test]
fn test_metrics_request_tracking() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let metrics = MetricsCollector::new();

        // Record a successful request
        let start_time = metrics.record_request_start();
        metrics
            .record_request_end(start_time, true, "openai", "gpt-4")
            .await;

        let (total, success, errors) = metrics.get_basic_stats();
        assert_eq!(total, 1);
        assert_eq!(success, 1);
        assert_eq!(errors, 0);

        // Record a failed request
        let start_time = metrics.record_request_start();
        metrics
            .record_request_end(start_time, false, "openai", "gpt-4")
            .await;

        let (total, success, errors) = metrics.get_basic_stats();
        assert_eq!(total, 2);
        assert_eq!(success, 1);
        assert_eq!(errors, 1);
    });
}

#[test]
fn test_metrics_concurrent_requests() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let metrics = MetricsCollector::new();

        assert_eq!(metrics.get_concurrent_requests(), 0);

        metrics.increment_concurrent_requests().await;
        assert_eq!(metrics.get_concurrent_requests(), 1);

        metrics.increment_concurrent_requests().await;
        assert_eq!(metrics.get_concurrent_requests(), 2);

        metrics.decrement_concurrent_requests().await;
        assert_eq!(metrics.get_concurrent_requests(), 1);
    });
}

#[test]
fn test_metrics_provider_specific() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let metrics = MetricsCollector::new();

        // Record requests for different providers
        let start_time = metrics.record_request_start();
        metrics
            .record_request_end(start_time, true, "openai", "gpt-4")
            .await;

        let start_time = metrics.record_request_start();
        metrics
            .record_request_end(start_time, true, "anthropic", "claude-3")
            .await;

        let start_time = metrics.record_request_start();
        metrics
            .record_request_end(start_time, true, "openai", "gpt-3.5")
            .await;

        // Get metrics summary to check provider-specific data
        let summary = metrics.get_metrics_summary().await;
        assert_eq!(summary.total_requests, 3);
        assert_eq!(summary.successful_requests, 3);
        assert!(summary.provider_metrics.contains_key("openai"));
        assert!(summary.provider_metrics.contains_key("anthropic"));

        let openai_metrics = &summary.provider_metrics["openai"];
        assert_eq!(openai_metrics.total_requests, 2);
        assert_eq!(openai_metrics.successful_requests, 2);

        let anthropic_metrics = &summary.provider_metrics["anthropic"];
        assert_eq!(anthropic_metrics.total_requests, 1);
        assert_eq!(anthropic_metrics.successful_requests, 1);
    });
}

#[test]
fn test_metrics_provider_errors() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let metrics = MetricsCollector::new();

        // Record successful and failed requests for different providers
        let start_time = metrics.record_request_start();
        metrics
            .record_request_end(start_time, false, "openai", "gpt-4")
            .await;

        let start_time = metrics.record_request_start();
        metrics
            .record_request_end(start_time, false, "openai", "gpt-4")
            .await;

        let start_time = metrics.record_request_start();
        metrics
            .record_request_end(start_time, false, "anthropic", "claude-3")
            .await;

        let summary = metrics.get_metrics_summary().await;
        assert_eq!(summary.total_requests, 3);
        assert_eq!(summary.failed_requests, 3);

        let openai_metrics = &summary.provider_metrics["openai"];
        assert_eq!(openai_metrics.failed_requests, 2);

        let anthropic_metrics = &summary.provider_metrics["anthropic"];
        assert_eq!(anthropic_metrics.failed_requests, 1);
    });
}

#[test]
fn test_metrics_success_rate() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let metrics = MetricsCollector::new();

        // No requests yet
        let summary = metrics.get_metrics_summary().await;
        assert_eq!(summary.success_rate_percent, 0.0);

        // All successful requests
        let start_time = metrics.record_request_start();
        metrics
            .record_request_end(start_time, true, "openai", "gpt-4")
            .await;

        let start_time = metrics.record_request_start();
        metrics
            .record_request_end(start_time, true, "openai", "gpt-4")
            .await;

        let summary = metrics.get_metrics_summary().await;
        assert_eq!(summary.success_rate_percent, 100.0);

        // Some errors
        let start_time = metrics.record_request_start();
        metrics
            .record_request_end(start_time, false, "openai", "gpt-4")
            .await;

        let summary = metrics.get_metrics_summary().await;
        assert!((summary.success_rate_percent - 66.66666666666667).abs() < 0.001);
    });
}

#[test]
fn test_metrics_reset() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let metrics = MetricsCollector::new();

        // Add some data
        let start_time = metrics.record_request_start();
        metrics
            .record_request_end(start_time, true, "openai", "gpt-4")
            .await;

        let start_time = metrics.record_request_start();
        metrics
            .record_request_end(start_time, false, "openai", "gpt-4")
            .await;

        metrics.increment_concurrent_requests().await;

        // Verify data exists
        let (total, success, errors) = metrics.get_basic_stats();
        assert_eq!(total, 2);
        assert_eq!(success, 1);
        assert_eq!(errors, 1);
        assert_eq!(metrics.get_concurrent_requests(), 1);

        // Reset metrics
        metrics.reset_metrics().await;

        // Verify reset
        let (total, success, errors) = metrics.get_basic_stats();
        assert_eq!(total, 0);
        assert_eq!(success, 0);
        assert_eq!(errors, 0);
        assert_eq!(metrics.get_concurrent_requests(), 0);
    });
}

#[test]
fn test_metrics_thread_safety() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        use std::sync::Arc;
        use tokio::task;

        let metrics = Arc::new(MetricsCollector::new());
        let mut handles = vec![];

        // Spawn multiple async tasks to record requests
        for _ in 0..10 {
            let metrics_clone = Arc::clone(&metrics);
            let handle = task::spawn(async move {
                for _ in 0..100 {
                    let start_time = metrics_clone.record_request_start();
                    metrics_clone
                        .record_request_end(start_time, true, "test", "model")
                        .await;
                }
            });
            handles.push(handle);
        }

        // Wait for all tasks to complete
        for handle in handles {
            handle.await.unwrap();
        }

        // Verify final count
        let (total, success, _errors) = metrics.get_basic_stats();
        assert_eq!(total, 1000);
        assert_eq!(success, 1000);
    });
}

#[test]
fn test_metrics_response_time_statistics() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let metrics = MetricsCollector::new();

        // Record requests with different response times
        let start_time = std::time::Instant::now();
        std::thread::sleep(Duration::from_millis(50));
        metrics
            .record_request_end(start_time, true, "openai", "gpt-4")
            .await;

        let start_time = std::time::Instant::now();
        std::thread::sleep(Duration::from_millis(100));
        metrics
            .record_request_end(start_time, true, "openai", "gpt-4")
            .await;

        let summary = metrics.get_metrics_summary().await;
        assert!(summary.avg_latency_ms >= 0.0);
        assert!(summary.latency_stats.min_latency_ms > 0);
        assert!(summary.latency_stats.max_latency_ms > 0);
        assert_eq!(summary.latency_stats.request_count, 2);
    });
}

#[test]
fn test_metrics_summary() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let metrics = MetricsCollector::new();

        // Record some requests
        let start_time = metrics.record_request_start();
        metrics
            .record_request_end(start_time, true, "openai", "gpt-4")
            .await;

        let start_time = metrics.record_request_start();
        metrics
            .record_request_end(start_time, false, "anthropic", "claude-3")
            .await;

        let summary = metrics.get_metrics_summary().await;

        // Verify summary data
        assert_eq!(summary.total_requests, 2);
        assert_eq!(summary.successful_requests, 1);
        assert_eq!(summary.failed_requests, 1);
        assert_eq!(summary.success_rate_percent, 50.0);
        assert_eq!(summary.error_rate_percent, 50.0);
        assert!(!summary.timestamp.is_empty());

        // Verify provider metrics
        assert!(summary.provider_metrics.contains_key("openai"));
        assert!(summary.provider_metrics.contains_key("anthropic"));

        // Verify model metrics
        assert!(summary.model_metrics.contains_key("gpt-4"));
        assert!(summary.model_metrics.contains_key("claude-3"));
    });
}
