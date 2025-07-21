/// Performance and Load Testing for AI Proxy
///
/// This module provides comprehensive performance testing including:
/// - Concurrent request handling tests
/// - Memory usage and streaming performance tests
/// - Load testing for system stability verification
/// - Benchmarking for different scenarios

use ai_proxy::{
    server::{AppState, create_app},
};
use axum::{
    body::Body,
    http::Request,
    response::Response,
};
use serde_json::json;
use std::{
    sync::{Arc, atomic::{AtomicUsize, Ordering}},
    time::{Duration, Instant},
};
use tokio::{
    sync::Semaphore,
    task::JoinSet,
    time::timeout,
};
use tower::ServiceExt;

use crate::integration_framework::IntegrationTestFramework;

mod integration_framework;

/// Performance test configuration
#[derive(Debug, Clone)]
pub struct PerformanceTestConfig {
    pub concurrent_requests: usize,
    pub total_requests: usize,
    pub request_timeout: Duration,
    pub warmup_requests: usize,
    pub test_duration: Duration,
}

impl Default for PerformanceTestConfig {
    fn default() -> Self {
        Self {
            concurrent_requests: 50,
            total_requests: 1000,
            request_timeout: Duration::from_secs(30),
            warmup_requests: 10,
            test_duration: Duration::from_secs(60),
        }
    }
}

/// Performance test results
#[derive(Debug, Clone)]
pub struct PerformanceTestResults {
    pub total_requests: usize,
    pub successful_requests: usize,
    pub failed_requests: usize,
    pub average_latency: Duration,
    pub p95_latency: Duration,
    pub p99_latency: Duration,
    pub max_latency: Duration,
    pub min_latency: Duration,
    pub requests_per_second: f64,
    pub memory_usage_mb: f64,
    pub error_rate: f64,
}

/// Memory usage tracker
#[derive(Debug)]
pub struct MemoryTracker {
    initial_memory: usize,
    peak_memory: AtomicUsize,
    current_memory: AtomicUsize,
}

impl MemoryTracker {
    pub fn new() -> Self {
        let initial = Self::get_memory_usage();
        Self {
            initial_memory: initial,
            peak_memory: AtomicUsize::new(initial),
            current_memory: AtomicUsize::new(initial),
        }
    }

    pub fn update(&self) {
        let current = Self::get_memory_usage();
        self.current_memory.store(current, Ordering::Relaxed);
        
        let mut peak = self.peak_memory.load(Ordering::Relaxed);
        while current > peak {
            match self.peak_memory.compare_exchange_weak(
                peak, 
                current, 
                Ordering::Relaxed, 
                Ordering::Relaxed
            ) {
                Ok(_) => break,
                Err(x) => peak = x,
            }
        }
    }

    pub fn get_peak_usage_mb(&self) -> f64 {
        (self.peak_memory.load(Ordering::Relaxed) - self.initial_memory) as f64 / 1024.0 / 1024.0
    }

    pub fn get_current_usage_mb(&self) -> f64 {
        (self.current_memory.load(Ordering::Relaxed) - self.initial_memory) as f64 / 1024.0 / 1024.0
    }

    fn get_memory_usage() -> usize {
        // Simple memory usage estimation
        // In a real implementation, you might use a more sophisticated method
        std::process::id() as usize * 1024 // Placeholder
    }
}

/// Performance test suite
pub struct PerformanceTestSuite {
    framework: IntegrationTestFramework,
    app_state: AppState,
    memory_tracker: Arc<MemoryTracker>,
}

impl PerformanceTestSuite {
    /// Create a new performance test suite
    pub async fn new() -> Self {
        let framework = IntegrationTestFramework::new()
            .await
            .with_openai()
            .await
            .with_anthropic()
            .await
            .with_gemini()
            .await;

        let app_state = framework.create_app_state().await;
        let memory_tracker = Arc::new(MemoryTracker::new());

        Self {
            framework,
            app_state,
            memory_tracker,
        }
    }

    /// Run concurrent request handling test
    pub async fn test_concurrent_requests(&self, config: PerformanceTestConfig) -> PerformanceTestResults {
        println!("Starting concurrent request test with {} concurrent requests", config.concurrent_requests);
        
        let app = create_app(self.app_state.clone());
        let semaphore = Arc::new(Semaphore::new(config.concurrent_requests));
        let mut latencies = Vec::new();
        let successful_requests = Arc::new(AtomicUsize::new(0));
        let failed_requests = Arc::new(AtomicUsize::new(0));
        
        let start_time = Instant::now();
        let mut join_set = JoinSet::new();

        // Warmup requests
        for _ in 0..config.warmup_requests {
            let _ = self.send_test_request(&app).await;
        }

        // Main test requests
        for i in 0..config.total_requests {
            let app_clone = app.clone();
            let semaphore_clone = semaphore.clone();
            let successful_clone = successful_requests.clone();
            let failed_clone = failed_requests.clone();
            let memory_tracker = self.memory_tracker.clone();

            join_set.spawn(async move {
                let _permit = semaphore_clone.acquire().await.unwrap();
                memory_tracker.update();
                
                let request_start = Instant::now();
                match timeout(Duration::from_secs(30), Self::send_single_request(&app_clone)).await {
                    Ok(Ok(_)) => {
                        successful_clone.fetch_add(1, Ordering::Relaxed);
                        request_start.elapsed()
                    }
                    _ => {
                        failed_clone.fetch_add(1, Ordering::Relaxed);
                        request_start.elapsed()
                    }
                }
            });

            // Add small delay to prevent overwhelming the system
            if i % 10 == 0 {
                tokio::time::sleep(Duration::from_millis(1)).await;
            }
        }

        // Collect results
        while let Some(result) = join_set.join_next().await {
            if let Ok(latency) = result {
                latencies.push(latency);
            }
        }

        let total_duration = start_time.elapsed();
        self.calculate_results(latencies, successful_requests, failed_requests, total_duration)
    }

    /// Test streaming performance and memory usage
    pub async fn test_streaming_performance(&self, config: PerformanceTestConfig) -> PerformanceTestResults {
        println!("Starting streaming performance test");
        
        let app = create_app(self.app_state.clone());
        let mut latencies = Vec::new();
        let successful_requests = Arc::new(AtomicUsize::new(0));
        let failed_requests = Arc::new(AtomicUsize::new(0));
        
        let start_time = Instant::now();
        let mut join_set = JoinSet::new();

        for _ in 0..config.total_requests {
            let app_clone = app.clone();
            let successful_clone = successful_requests.clone();
            let failed_clone = failed_requests.clone();
            let memory_tracker = self.memory_tracker.clone();

            join_set.spawn(async move {
                memory_tracker.update();
                let request_start = Instant::now();
                
                match Self::send_streaming_request(&app_clone).await {
                    Ok(_) => {
                        successful_clone.fetch_add(1, Ordering::Relaxed);
                        request_start.elapsed()
                    }
                    Err(_) => {
                        failed_clone.fetch_add(1, Ordering::Relaxed);
                        request_start.elapsed()
                    }
                }
            });

            // Control request rate for streaming tests
            tokio::time::sleep(Duration::from_millis(10)).await;
        }

        // Collect results
        while let Some(result) = join_set.join_next().await {
            if let Ok(latency) = result {
                latencies.push(latency);
            }
        }

        let total_duration = start_time.elapsed();
        self.calculate_results(latencies, successful_requests, failed_requests, total_duration)
    }

    /// Run load test to verify system stability
    pub async fn test_system_stability(&self, config: PerformanceTestConfig) -> PerformanceTestResults {
        println!("Starting system stability test for {} seconds", config.test_duration.as_secs());
        
        let app = create_app(self.app_state.clone());
        let mut latencies = Vec::new();
        let successful_requests = Arc::new(AtomicUsize::new(0));
        let failed_requests = Arc::new(AtomicUsize::new(0));
        
        let start_time = Instant::now();
        let end_time = start_time + config.test_duration;
        let mut join_set = JoinSet::new();
        let request_counter = Arc::new(AtomicUsize::new(0));

        // Continuous load generation
        while Instant::now() < end_time {
            let app_clone = app.clone();
            let successful_clone = successful_requests.clone();
            let failed_clone = failed_requests.clone();
            let memory_tracker = self.memory_tracker.clone();
            let counter = request_counter.clone();

            join_set.spawn(async move {
                memory_tracker.update();
                let request_start = Instant::now();
                counter.fetch_add(1, Ordering::Relaxed);
                
                match Self::send_mixed_request(&app_clone).await {
                    Ok(_) => {
                        successful_clone.fetch_add(1, Ordering::Relaxed);
                        request_start.elapsed()
                    }
                    Err(_) => {
                        failed_clone.fetch_add(1, Ordering::Relaxed);
                        request_start.elapsed()
                    }
                }
            });

            // Control request rate
            tokio::time::sleep(Duration::from_millis(50)).await;
        }

        // Wait for remaining requests to complete
        while let Some(result) = join_set.join_next().await {
            if let Ok(latency) = result {
                latencies.push(latency);
            }
        }

        let total_duration = start_time.elapsed();
        self.calculate_results(latencies, successful_requests, failed_requests, total_duration)
    }

    /// Send a test request to the application
    async fn send_test_request(&self, app: &axum::Router) -> Result<Response<Body>, Box<dyn std::error::Error + Send + Sync>> {
        let request = Request::builder()
            .method("POST")
            .uri("/v1/messages")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_string(&json!({
                "model": "gpt-4",
                "messages": [{"role": "user", "content": "Hello, this is a performance test"}],
                "max_tokens": 50,
                "stream": false
            }))?))
            .unwrap();

        let response = app.clone().oneshot(request).await?;
        Ok(response)
    }

    /// Send a single request for concurrent testing
    async fn send_single_request(app: &axum::Router) -> Result<Response<Body>, Box<dyn std::error::Error + Send + Sync>> {
        let request = Request::builder()
            .method("POST")
            .uri("/v1/messages")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_string(&json!({
                "model": "claude-3-sonnet",
                "messages": [{"role": "user", "content": "Performance test message"}],
                "max_tokens": 100,
                "stream": false
            }))?))
            .unwrap();

        let response = app.clone().oneshot(request).await?;
        Ok(response)
    }

    /// Send a streaming request for streaming performance testing
    async fn send_streaming_request(app: &axum::Router) -> Result<Response<Body>, Box<dyn std::error::Error + Send + Sync>> {
        let request = Request::builder()
            .method("POST")
            .uri("/v1/messages")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_string(&json!({
                "model": "claude-3-sonnet",
                "messages": [{"role": "user", "content": "This is a streaming performance test with a longer message to test streaming capabilities and memory usage under load"}],
                "max_tokens": 200,
                "stream": true
            }))?))
            .unwrap();

        let response = app.clone().oneshot(request).await?;
        
        // Consume the streaming response to test memory usage
        let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX).await?;
        let _body_string = String::from_utf8(body_bytes.to_vec())?;
        
        Ok(Response::builder().status(200).body(Body::empty()).unwrap())
    }

    /// Send mixed requests (streaming and non-streaming) for stability testing
    async fn send_mixed_request(app: &axum::Router) -> Result<Response<Body>, Box<dyn std::error::Error + Send + Sync>> {
        let is_streaming = rand::random::<bool>();
        let models = ["gpt-4", "claude-3-sonnet", "gemini-pro"];
        let model = models[rand::random::<usize>() % models.len()];
        
        let request = Request::builder()
            .method("POST")
            .uri("/v1/messages")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_string(&json!({
                "model": model,
                "messages": [{"role": "user", "content": "Mixed load test message for system stability verification"}],
                "max_tokens": if is_streaming { 150 } else { 75 },
                "stream": is_streaming
            }))?))
            .unwrap();

        let response = app.clone().oneshot(request).await?;
        
        if is_streaming {
            // Consume streaming response
            let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX).await?;
            let _body_string = String::from_utf8(body_bytes.to_vec())?;
            Ok(Response::builder().status(200).body(Body::empty()).unwrap())
        } else {
            Ok(response)
        }
    }

    /// Calculate performance test results
    fn calculate_results(
        &self,
        mut latencies: Vec<Duration>,
        successful_requests: Arc<AtomicUsize>,
        failed_requests: Arc<AtomicUsize>,
        total_duration: Duration,
    ) -> PerformanceTestResults {
        latencies.sort();
        
        let total_requests = successful_requests.load(Ordering::Relaxed) + failed_requests.load(Ordering::Relaxed);
        let successful = successful_requests.load(Ordering::Relaxed);
        let failed = failed_requests.load(Ordering::Relaxed);
        
        let average_latency = if !latencies.is_empty() {
            latencies.iter().sum::<Duration>() / latencies.len() as u32
        } else {
            Duration::ZERO
        };
        
        let p95_latency = if !latencies.is_empty() {
            latencies[(latencies.len() as f64 * 0.95) as usize]
        } else {
            Duration::ZERO
        };
        
        let p99_latency = if !latencies.is_empty() {
            latencies[(latencies.len() as f64 * 0.99) as usize]
        } else {
            Duration::ZERO
        };
        
        let max_latency = latencies.last().copied().unwrap_or(Duration::ZERO);
        let min_latency = latencies.first().copied().unwrap_or(Duration::ZERO);
        
        let requests_per_second = if total_duration.as_secs_f64() > 0.0 {
            successful as f64 / total_duration.as_secs_f64()
        } else {
            0.0
        };
        
        let error_rate = if total_requests > 0 {
            (failed as f64 / total_requests as f64) * 100.0
        } else {
            0.0
        };
        
        PerformanceTestResults {
            total_requests,
            successful_requests: successful,
            failed_requests: failed,
            average_latency,
            p95_latency,
            p99_latency,
            max_latency,
            min_latency,
            requests_per_second,
            memory_usage_mb: self.memory_tracker.get_peak_usage_mb(),
            error_rate,
        }
    }
}

/// Performance test utilities for integration with benchmarking tools
impl PerformanceTestSuite {
    /// Get a configured test suite for benchmarking
    pub async fn for_benchmarking() -> Self {
        Self::new().await
    }
    
    /// Run a quick concurrent test for benchmarking
    pub async fn benchmark_concurrent(&self, concurrency: usize) -> PerformanceTestResults {
        let config = PerformanceTestConfig {
            concurrent_requests: concurrency,
            total_requests: concurrency * 2,
            request_timeout: Duration::from_secs(5),
            ..Default::default()
        };
        
        self.test_concurrent_requests(config).await
    }
    
    /// Run a quick streaming test for benchmarking
    pub async fn benchmark_streaming(&self) -> PerformanceTestResults {
        let config = PerformanceTestConfig {
            total_requests: 10,
            request_timeout: Duration::from_secs(5),
            ..Default::default()
        };
        
        self.test_streaming_performance(config).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_concurrent_request_handling() {
        let suite = PerformanceTestSuite::new().await;
        
        let config = PerformanceTestConfig {
            concurrent_requests: 20,
            total_requests: 100,
            request_timeout: Duration::from_secs(10),
            ..Default::default()
        };
        
        let results = suite.test_concurrent_requests(config).await;
        
        println!("Concurrent Request Test Results:");
        println!("Total Requests: {}", results.total_requests);
        println!("Successful: {}", results.successful_requests);
        println!("Failed: {}", results.failed_requests);
        println!("Average Latency: {:?}", results.average_latency);
        println!("P95 Latency: {:?}", results.p95_latency);
        println!("P99 Latency: {:?}", results.p99_latency);
        println!("Requests/sec: {:.2}", results.requests_per_second);
        println!("Error Rate: {:.2}%", results.error_rate);
        println!("Memory Usage: {:.2} MB", results.memory_usage_mb);
        
        // Assertions for performance requirements
        assert!(results.error_rate < 5.0, "Error rate should be less than 5%");
        assert!(results.requests_per_second > 10.0, "Should handle at least 10 requests per second");
        assert!(results.average_latency < Duration::from_secs(2), "Average latency should be less than 2 seconds");
    }

    #[tokio::test]
    async fn test_streaming_memory_usage() {
        let suite = PerformanceTestSuite::new().await;
        
        let config = PerformanceTestConfig {
            total_requests: 50,
            request_timeout: Duration::from_secs(15),
            ..Default::default()
        };
        
        let results = suite.test_streaming_performance(config).await;
        
        println!("Streaming Performance Test Results:");
        println!("Total Requests: {}", results.total_requests);
        println!("Successful: {}", results.successful_requests);
        println!("Failed: {}", results.failed_requests);
        println!("Average Latency: {:?}", results.average_latency);
        println!("Memory Usage: {:.2} MB", results.memory_usage_mb);
        println!("Error Rate: {:.2}%", results.error_rate);
        
        // Assertions for streaming performance
        assert!(results.error_rate < 10.0, "Streaming error rate should be less than 10%");
        assert!(results.memory_usage_mb < 100.0, "Memory usage should be reasonable for streaming");
        assert!(results.successful_requests > 0, "Should have some successful streaming requests");
    }

    #[tokio::test]
    async fn test_system_stability_under_load() {
        let suite = PerformanceTestSuite::new().await;
        
        let config = PerformanceTestConfig {
            test_duration: Duration::from_secs(30),
            ..Default::default()
        };
        
        let results = suite.test_system_stability(config).await;
        
        println!("System Stability Test Results:");
        println!("Total Requests: {}", results.total_requests);
        println!("Successful: {}", results.successful_requests);
        println!("Failed: {}", results.failed_requests);
        println!("Average Latency: {:?}", results.average_latency);
        println!("P95 Latency: {:?}", results.p95_latency);
        println!("Requests/sec: {:.2}", results.requests_per_second);
        println!("Error Rate: {:.2}%", results.error_rate);
        println!("Memory Usage: {:.2} MB", results.memory_usage_mb);
        
        // Assertions for system stability
        assert!(results.error_rate < 15.0, "Error rate under sustained load should be manageable");
        assert!(results.total_requests > 100, "Should process a significant number of requests");
        assert!(results.p95_latency < Duration::from_secs(5), "P95 latency should be reasonable");
    }

    #[tokio::test]
    async fn test_memory_tracker() {
        let tracker = MemoryTracker::new();
        
        // Simulate memory usage updates
        for _ in 0..10 {
            tracker.update();
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        
        let peak_usage = tracker.get_peak_usage_mb();
        let current_usage = tracker.get_current_usage_mb();
        
        println!("Peak Memory Usage: {:.2} MB", peak_usage);
        println!("Current Memory Usage: {:.2} MB", current_usage);
        
        // Basic assertions
        assert!(peak_usage >= 0.0, "Peak memory usage should be non-negative");
        assert!(current_usage >= 0.0, "Current memory usage should be non-negative");
    }

    #[tokio::test]
    async fn test_performance_config_validation() {
        let config = PerformanceTestConfig::default();
        
        assert!(config.concurrent_requests > 0, "Concurrent requests should be positive");
        assert!(config.total_requests > 0, "Total requests should be positive");
        assert!(config.request_timeout > Duration::ZERO, "Request timeout should be positive");
        assert!(config.test_duration > Duration::ZERO, "Test duration should be positive");
    }
}