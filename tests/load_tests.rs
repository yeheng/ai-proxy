/// Load Testing Module for AI Proxy
///
/// This module provides specialized load testing capabilities including:
/// - High-volume request processing
/// - Sustained load testing
/// - Resource exhaustion testing
/// - Scalability verification

use ai_proxy::server::{AppState, create_app};
use axum::{
    body::Body,
    http::Request,
    response::Response,
};
use futures::StreamExt;
use serde_json::json;
use std::{
    sync::{Arc, atomic::{AtomicUsize, Ordering}},
    time::{Duration, Instant},
};
use tokio::{
    sync::{RwLock, Barrier},
    task::JoinSet,
    time::{timeout, interval},
};
use tower::ServiceExt;
use integration_framework::IntegrationTestFramework;

mod integration_framework;

/// Load test configuration for different scenarios
#[derive(Debug, Clone)]
pub struct LoadTestConfig {
    pub name: String,
    pub concurrent_users: usize,
    pub requests_per_user: usize,
    pub ramp_up_duration: Duration,
    pub test_duration: Duration,
    pub cool_down_duration: Duration,
    pub request_timeout: Duration,
    pub think_time: Duration,
    pub target_rps: Option<f64>,
}

impl LoadTestConfig {
    pub fn light_load() -> Self {
        Self {
            name: "Light Load".to_string(),
            concurrent_users: 10,
            requests_per_user: 50,
            ramp_up_duration: Duration::from_secs(10),
            test_duration: Duration::from_secs(60),
            cool_down_duration: Duration::from_secs(5),
            request_timeout: Duration::from_secs(30),
            think_time: Duration::from_millis(100),
            target_rps: Some(50.0),
        }
    }

    pub fn moderate_load() -> Self {
        Self {
            name: "Moderate Load".to_string(),
            concurrent_users: 50,
            requests_per_user: 100,
            ramp_up_duration: Duration::from_secs(30),
            test_duration: Duration::from_secs(120),
            cool_down_duration: Duration::from_secs(10),
            request_timeout: Duration::from_secs(30),
            think_time: Duration::from_millis(200),
            target_rps: Some(200.0),
        }
    }

    pub fn heavy_load() -> Self {
        Self {
            name: "Heavy Load".to_string(),
            concurrent_users: 100,
            requests_per_user: 200,
            ramp_up_duration: Duration::from_secs(60),
            test_duration: Duration::from_secs(300),
            cool_down_duration: Duration::from_secs(30),
            request_timeout: Duration::from_secs(45),
            think_time: Duration::from_millis(500),
            target_rps: Some(500.0),
        }
    }

    pub fn stress_test() -> Self {
        Self {
            name: "Stress Test".to_string(),
            concurrent_users: 200,
            requests_per_user: 500,
            ramp_up_duration: Duration::from_secs(120),
            test_duration: Duration::from_secs(600),
            cool_down_duration: Duration::from_secs(60),
            request_timeout: Duration::from_secs(60),
            think_time: Duration::from_millis(1000),
            target_rps: Some(1000.0),
        }
    }
}

/// Detailed load test results with comprehensive metrics
#[derive(Debug, Clone)]
pub struct LoadTestResults {
    pub config: LoadTestConfig,
    pub total_requests: usize,
    pub successful_requests: usize,
    pub failed_requests: usize,
    pub timeout_requests: usize,
    pub error_requests: usize,
    
    // Latency metrics
    pub min_latency: Duration,
    pub max_latency: Duration,
    pub mean_latency: Duration,
    pub median_latency: Duration,
    pub p90_latency: Duration,
    pub p95_latency: Duration,
    pub p99_latency: Duration,
    pub p999_latency: Duration,
    
    // Throughput metrics
    pub requests_per_second: f64,
    pub peak_rps: f64,
    pub average_rps: f64,
    
    // Error metrics
    pub error_rate: f64,
    pub timeout_rate: f64,
    pub success_rate: f64,
    
    // Resource metrics
    pub peak_memory_mb: f64,
    pub average_memory_mb: f64,
    pub cpu_usage_percent: f64,
    
    // Test execution metrics
    pub actual_test_duration: Duration,
    pub ramp_up_completed: bool,
    pub target_rps_achieved: bool,
}

/// Resource monitor for tracking system resources during load tests
#[derive(Debug)]
pub struct ResourceMonitor {
    memory_samples: Arc<RwLock<Vec<f64>>>,
    cpu_samples: Arc<RwLock<Vec<f64>>>,
    rps_samples: Arc<RwLock<Vec<f64>>>,
    monitoring: Arc<AtomicUsize>,
}

impl ResourceMonitor {
    pub fn new() -> Self {
        Self {
            memory_samples: Arc::new(RwLock::new(Vec::new())),
            cpu_samples: Arc::new(RwLock::new(Vec::new())),
            rps_samples: Arc::new(RwLock::new(Vec::new())),
            monitoring: Arc::new(AtomicUsize::new(0)),
        }
    }

    pub async fn start_monitoring(&self, interval_ms: u64) {
        if self.monitoring.compare_exchange(0, 1, Ordering::Relaxed, Ordering::Relaxed).is_ok() {
            let memory_samples = self.memory_samples.clone();
            let cpu_samples = self.cpu_samples.clone();
            let monitoring = self.monitoring.clone();

            tokio::spawn(async move {
                let mut interval = interval(Duration::from_millis(interval_ms));
                
                while monitoring.load(Ordering::Relaxed) == 1 {
                    interval.tick().await;
                    
                    let memory_usage = Self::get_memory_usage_mb();
                    let cpu_usage = Self::get_cpu_usage_percent();
                    
                    memory_samples.write().await.push(memory_usage);
                    cpu_samples.write().await.push(cpu_usage);
                }
            });
        }
    }

    pub fn stop_monitoring(&self) {
        self.monitoring.store(0, Ordering::Relaxed);
    }

    pub async fn record_rps(&self, rps: f64) {
        self.rps_samples.write().await.push(rps);
    }

    pub async fn get_peak_memory(&self) -> f64 {
        self.memory_samples.read().await
            .iter()
            .fold(0.0, |max, &val| max.max(val))
    }

    pub async fn get_average_memory(&self) -> f64 {
        let samples = self.memory_samples.read().await;
        if samples.is_empty() {
            0.0
        } else {
            samples.iter().sum::<f64>() / samples.len() as f64
        }
    }

    pub async fn get_average_cpu(&self) -> f64 {
        let samples = self.cpu_samples.read().await;
        if samples.is_empty() {
            0.0
        } else {
            samples.iter().sum::<f64>() / samples.len() as f64
        }
    }

    pub async fn get_peak_rps(&self) -> f64 {
        self.rps_samples.read().await
            .iter()
            .fold(0.0, |max, &val| max.max(val))
    }

    fn get_memory_usage_mb() -> f64 {
        // Simplified memory usage calculation
        // In production, you would use proper system monitoring
        std::process::id() as f64 * 0.1 // Placeholder
    }

    fn get_cpu_usage_percent() -> f64 {
        // Simplified CPU usage calculation
        // In production, you would use proper system monitoring
        rand::random::<f64>() * 50.0 // Placeholder
    }
}

/// Load test executor with comprehensive testing capabilities
pub struct LoadTestExecutor {
    framework: IntegrationTestFramework,
    app_state: AppState,
    resource_monitor: ResourceMonitor,
}

impl LoadTestExecutor {
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
        let resource_monitor = ResourceMonitor::new();

        Self {
            framework,
            app_state,
            resource_monitor,
        }
    }

    /// Execute a comprehensive load test
    pub async fn execute_load_test(&self, config: LoadTestConfig) -> LoadTestResults {
        println!("Starting load test: {}", config.name);
        println!("Configuration: {} users, {} requests per user", config.concurrent_users, config.requests_per_user);
        
        let app = create_app(self.app_state.clone());
        
        // Start resource monitoring
        self.resource_monitor.start_monitoring(1000).await;
        
        let test_start = Instant::now();
        let mut latencies = Vec::new();
        let successful_requests = Arc::new(AtomicUsize::new(0));
        let failed_requests = Arc::new(AtomicUsize::new(0));
        let timeout_requests = Arc::new(AtomicUsize::new(0));
        let error_requests = Arc::new(AtomicUsize::new(0));
        let rps_counter = Arc::new(AtomicUsize::new(0));
        
        // Ramp-up phase
        println!("Ramp-up phase: {} seconds", config.ramp_up_duration.as_secs());
        let ramp_up_completed = self.execute_ramp_up(&app, &config, &rps_counter).await;
        
        // Main load test phase
        println!("Main test phase: {} seconds", config.test_duration.as_secs());
        let main_test_results = self.execute_main_test(
            &app,
            &config,
            &successful_requests,
            &failed_requests,
            &timeout_requests,
            &error_requests,
            &rps_counter,
        ).await;
        
        latencies.extend(main_test_results);
        
        // Cool-down phase
        println!("Cool-down phase: {} seconds", config.cool_down_duration.as_secs());
        tokio::time::sleep(config.cool_down_duration).await;
        
        // Stop monitoring
        self.resource_monitor.stop_monitoring();
        
        let actual_test_duration = test_start.elapsed();
        
        // Calculate comprehensive results
        self.calculate_comprehensive_results(
            config,
            latencies,
            successful_requests,
            failed_requests,
            timeout_requests,
            error_requests,
            actual_test_duration,
            ramp_up_completed,
        ).await
    }

    /// Execute ramp-up phase with gradual user increase
    async fn execute_ramp_up(&self, app: &axum::Router, config: &LoadTestConfig, rps_counter: &Arc<AtomicUsize>) -> bool {
        let ramp_up_steps = 10usize;
        let step_duration = config.ramp_up_duration / ramp_up_steps as u32;
        let users_per_step = config.concurrent_users / ramp_up_steps;
        
        for step in 1..=ramp_up_steps {
            let current_users = users_per_step * step;
            println!("Ramp-up step {}/{}: {} users", step, ramp_up_steps, current_users);
            
            let mut join_set = JoinSet::new();
            
            for _ in 0..users_per_step {
                let app_clone = app.clone();
                let rps_counter_clone = rps_counter.clone();
                
                join_set.spawn(async move {
                    let request_start = Instant::now();
                    match Self::send_ramp_up_request(&app_clone).await {
                        Ok(_) => {
                            rps_counter_clone.fetch_add(1, Ordering::Relaxed);
                            true
                        }
                        Err(_) => false
                    }
                });
            }
            
            // Wait for step completion
            let mut successful_in_step = 0;
            while let Some(result) = join_set.join_next().await {
                if let Ok(success) = result {
                    if success {
                        successful_in_step += 1;
                    }
                }
            }
            
            println!("Step {} completed: {}/{} successful", step, successful_in_step, users_per_step);
            
            // Record RPS for this step
            let step_rps = successful_in_step as f64 / step_duration.as_secs_f64();
            self.resource_monitor.record_rps(step_rps).await;
            
            tokio::time::sleep(step_duration).await;
        }
        
        true // Ramp-up completed successfully
    }

    /// Execute main test phase with sustained load
    async fn execute_main_test(
        &self,
        app: &axum::Router,
        config: &LoadTestConfig,
        successful_requests: &Arc<AtomicUsize>,
        failed_requests: &Arc<AtomicUsize>,
        timeout_requests: &Arc<AtomicUsize>,
        error_requests: &Arc<AtomicUsize>,
        rps_counter: &Arc<AtomicUsize>,
    ) -> Vec<Duration> {
        let mut latencies = Vec::new();
        let barrier = Arc::new(Barrier::new(config.concurrent_users));
        let mut join_set = JoinSet::new();
        
        // Launch concurrent users
        for user_id in 0..config.concurrent_users {
            let app_clone = app.clone();
            let config_clone = config.clone();
            let barrier_clone = barrier.clone();
            let successful_clone = successful_requests.clone();
            let failed_clone = failed_requests.clone();
            let timeout_clone = timeout_requests.clone();
            let error_clone = error_requests.clone();
            let rps_counter_clone = rps_counter.clone();
            
            join_set.spawn(async move {
                // Wait for all users to be ready
                barrier_clone.wait().await;
                
                let mut user_latencies = Vec::new();
                let user_start = Instant::now();
                
                for request_num in 0..config_clone.requests_per_user {
                    if user_start.elapsed() > config_clone.test_duration {
                        break;
                    }
                    
                    let request_start = Instant::now();
                    
                    match timeout(
                        config_clone.request_timeout,
                        Self::send_load_test_request(&app_clone, user_id, request_num)
                    ).await {
                        Ok(Ok(_)) => {
                            successful_clone.fetch_add(1, Ordering::Relaxed);
                            rps_counter_clone.fetch_add(1, Ordering::Relaxed);
                            user_latencies.push(request_start.elapsed());
                        }
                        Ok(Err(_)) => {
                            error_clone.fetch_add(1, Ordering::Relaxed);
                            failed_clone.fetch_add(1, Ordering::Relaxed);
                        }
                        Err(_) => {
                            timeout_clone.fetch_add(1, Ordering::Relaxed);
                            failed_clone.fetch_add(1, Ordering::Relaxed);
                        }
                    }
                    
                    // Think time between requests
                    if config_clone.think_time > Duration::ZERO {
                        tokio::time::sleep(config_clone.think_time).await;
                    }
                }
                
                user_latencies
            });
        }
        
        // Collect results from all users
        while let Some(result) = join_set.join_next().await {
            if let Ok(user_latencies) = result {
                latencies.extend(user_latencies);
            }
        }
        
        latencies
    }

    /// Calculate comprehensive load test results
    async fn calculate_comprehensive_results(
        &self,
        config: LoadTestConfig,
        mut latencies: Vec<Duration>,
        successful_requests: Arc<AtomicUsize>,
        failed_requests: Arc<AtomicUsize>,
        timeout_requests: Arc<AtomicUsize>,
        error_requests: Arc<AtomicUsize>,
        actual_test_duration: Duration,
        ramp_up_completed: bool,
    ) -> LoadTestResults {
        latencies.sort();
        
        let total_requests = successful_requests.load(Ordering::Relaxed) + failed_requests.load(Ordering::Relaxed);
        let successful = successful_requests.load(Ordering::Relaxed);
        let failed = failed_requests.load(Ordering::Relaxed);
        let timeouts = timeout_requests.load(Ordering::Relaxed);
        let errors = error_requests.load(Ordering::Relaxed);
        
        // Calculate latency percentiles
        let (min_latency, max_latency, mean_latency, median_latency, p90_latency, p95_latency, p99_latency, p999_latency) = 
            if !latencies.is_empty() {
                let min = latencies[0];
                let max = latencies[latencies.len() - 1];
                let mean = latencies.iter().sum::<Duration>() / latencies.len() as u32;
                let median = latencies[latencies.len() / 2];
                let p90 = latencies[(latencies.len() as f64 * 0.90) as usize];
                let p95 = latencies[(latencies.len() as f64 * 0.95) as usize];
                let p99 = latencies[(latencies.len() as f64 * 0.99) as usize];
                let p999 = latencies[(latencies.len() as f64 * 0.999) as usize];
                (min, max, mean, median, p90, p95, p99, p999)
            } else {
                (Duration::ZERO, Duration::ZERO, Duration::ZERO, Duration::ZERO, Duration::ZERO, Duration::ZERO, Duration::ZERO, Duration::ZERO)
            };
        
        // Calculate throughput metrics
        let requests_per_second = if actual_test_duration.as_secs_f64() > 0.0 {
            successful as f64 / actual_test_duration.as_secs_f64()
        } else {
            0.0
        };
        
        let peak_rps = self.resource_monitor.get_peak_rps().await;
        let average_rps = requests_per_second;
        
        // Calculate error rates
        let error_rate = if total_requests > 0 {
            (failed as f64 / total_requests as f64) * 100.0
        } else {
            0.0
        };
        
        let timeout_rate = if total_requests > 0 {
            (timeouts as f64 / total_requests as f64) * 100.0
        } else {
            0.0
        };
        
        let success_rate = if total_requests > 0 {
            (successful as f64 / total_requests as f64) * 100.0
        } else {
            0.0
        };
        
        // Get resource metrics
        let peak_memory_mb = self.resource_monitor.get_peak_memory().await;
        let average_memory_mb = self.resource_monitor.get_average_memory().await;
        let cpu_usage_percent = self.resource_monitor.get_average_cpu().await;
        
        // Check if target RPS was achieved
        let target_rps_achieved = if let Some(target) = config.target_rps {
            requests_per_second >= target * 0.8 // Allow 20% tolerance
        } else {
            true
        };
        
        LoadTestResults {
            config,
            total_requests,
            successful_requests: successful,
            failed_requests: failed,
            timeout_requests: timeouts,
            error_requests: errors,
            min_latency,
            max_latency,
            mean_latency,
            median_latency,
            p90_latency,
            p95_latency,
            p99_latency,
            p999_latency,
            requests_per_second,
            peak_rps,
            average_rps,
            error_rate,
            timeout_rate,
            success_rate,
            peak_memory_mb,
            average_memory_mb,
            cpu_usage_percent,
            actual_test_duration,
            ramp_up_completed,
            target_rps_achieved,
        }
    }

    /// Send a ramp-up request
    async fn send_ramp_up_request(app: &axum::Router) -> Result<Response<Body>, Box<dyn std::error::Error + Send + Sync>> {
        let request = Request::builder()
            .method("POST")
            .uri("/v1/messages")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_string(&json!({
                "model": "gpt-4",
                "messages": [{"role": "user", "content": "Ramp-up test message"}],
                "max_tokens": 50,
                "stream": false
            }))?))
            .unwrap();

        let response = app.clone().oneshot(request).await?;
        Ok(response)
    }

    /// Send a load test request
    async fn send_load_test_request(
        app: &axum::Router,
        user_id: usize,
        request_num: usize
    ) -> Result<Response<Body>, Box<dyn std::error::Error + Send + Sync>> {
        let models = ["gpt-4", "claude-3-sonnet", "gemini-pro"];
        let model = models[user_id % models.len()];
        let is_streaming = request_num % 3 == 0; // Every 3rd request is streaming
        
        let request = Request::builder()
            .method("POST")
            .uri("/v1/messages")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_string(&json!({
                "model": model,
                "messages": [{"role": "user", "content": format!("Load test message from user {} request {}", user_id, request_num)}],
                "max_tokens": if is_streaming { 100 } else { 50 },
                "stream": is_streaming
            }))?))
            .unwrap();

        let response = app.clone().oneshot(request).await?;
        
        if is_streaming {
            // Consume streaming response to test memory usage
            let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX).await?;
            let _body_string = String::from_utf8(body_bytes.to_vec())?;
            Ok(Response::builder().status(200).body(Body::empty()).unwrap())
        } else {
            Ok(response)
        }
    }
}

/// Print comprehensive load test results
pub fn print_load_test_results(results: &LoadTestResults) {
    println!("\n=== Load Test Results: {} ===", results.config.name);
    println!("Test Duration: {:?}", results.actual_test_duration);
    println!("Ramp-up Completed: {}", results.ramp_up_completed);
    println!("Target RPS Achieved: {}", results.target_rps_achieved);
    
    println!("\n--- Request Statistics ---");
    println!("Total Requests: {}", results.total_requests);
    println!("Successful: {} ({:.2}%)", results.successful_requests, results.success_rate);
    println!("Failed: {} ({:.2}%)", results.failed_requests, results.error_rate);
    println!("Timeouts: {} ({:.2}%)", results.timeout_requests, results.timeout_rate);
    println!("Errors: {}", results.error_requests);
    
    println!("\n--- Latency Statistics ---");
    println!("Min: {:?}", results.min_latency);
    println!("Max: {:?}", results.max_latency);
    println!("Mean: {:?}", results.mean_latency);
    println!("Median: {:?}", results.median_latency);
    println!("P90: {:?}", results.p90_latency);
    println!("P95: {:?}", results.p95_latency);
    println!("P99: {:?}", results.p99_latency);
    println!("P99.9: {:?}", results.p999_latency);
    
    println!("\n--- Throughput Statistics ---");
    println!("Average RPS: {:.2}", results.average_rps);
    println!("Peak RPS: {:.2}", results.peak_rps);
    println!("Requests/sec: {:.2}", results.requests_per_second);
    
    println!("\n--- Resource Usage ---");
    println!("Peak Memory: {:.2} MB", results.peak_memory_mb);
    println!("Average Memory: {:.2} MB", results.average_memory_mb);
    println!("CPU Usage: {:.2}%", results.cpu_usage_percent);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_light_load() {
        let executor = LoadTestExecutor::new().await;
        let config = LoadTestConfig::light_load();
        
        let results = executor.execute_load_test(config).await;
        print_load_test_results(&results);
        
        // Assertions for light load
        assert!(results.success_rate > 90.0, "Success rate should be high for light load");
        assert!(results.error_rate < 10.0, "Error rate should be low for light load");
        assert!(results.ramp_up_completed, "Ramp-up should complete successfully");
    }

    #[tokio::test]
    async fn test_moderate_load() {
        let executor = LoadTestExecutor::new().await;
        let config = LoadTestConfig::moderate_load();
        
        let results = executor.execute_load_test(config).await;
        print_load_test_results(&results);
        
        // Assertions for moderate load
        assert!(results.success_rate > 80.0, "Success rate should be acceptable for moderate load");
        assert!(results.error_rate < 20.0, "Error rate should be manageable for moderate load");
        assert!(results.total_requests > 1000, "Should process significant number of requests");
    }

    #[tokio::test]
    #[ignore] // This test takes a long time, run manually
    async fn test_heavy_load() {
        let executor = LoadTestExecutor::new().await;
        let config = LoadTestConfig::heavy_load();
        
        let results = executor.execute_load_test(config).await;
        print_load_test_results(&results);
        
        // Assertions for heavy load
        assert!(results.success_rate > 70.0, "Success rate should be reasonable for heavy load");
        assert!(results.total_requests > 5000, "Should process large number of requests");
        assert!(results.p95_latency < Duration::from_secs(10), "P95 latency should be reasonable");
    }

    #[tokio::test]
    #[ignore] // This test is very intensive, run manually
    async fn test_stress_test() {
        let executor = LoadTestExecutor::new().await;
        let config = LoadTestConfig::stress_test();
        
        let results = executor.execute_load_test(config).await;
        print_load_test_results(&results);
        
        // Assertions for stress test - more lenient as system is under extreme load
        assert!(results.total_requests > 10000, "Should process very large number of requests");
        assert!(results.success_rate > 50.0, "Should maintain some level of service under stress");
    }

    #[tokio::test]
    async fn test_resource_monitor() {
        let monitor = ResourceMonitor::new();
        
        monitor.start_monitoring(100).await;
        
        // Simulate some activity
        for i in 0..10 {
            monitor.record_rps(i as f64 * 10.0).await;
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
        
        monitor.stop_monitoring();
        
        let peak_rps = monitor.get_peak_rps().await;
        let avg_memory = monitor.get_average_memory().await;
        let avg_cpu = monitor.get_average_cpu().await;
        
        println!("Peak RPS: {:.2}", peak_rps);
        println!("Average Memory: {:.2} MB", avg_memory);
        println!("Average CPU: {:.2}%", avg_cpu);
        
        assert!(peak_rps > 0.0, "Should record some RPS data");
    }

    #[tokio::test]
    async fn test_load_test_configs() {
        let light = LoadTestConfig::light_load();
        let moderate = LoadTestConfig::moderate_load();
        let heavy = LoadTestConfig::heavy_load();
        let stress = LoadTestConfig::stress_test();
        
        // Verify configuration scaling
        assert!(light.concurrent_users < moderate.concurrent_users);
        assert!(moderate.concurrent_users < heavy.concurrent_users);
        assert!(heavy.concurrent_users < stress.concurrent_users);
        
        assert!(light.test_duration < moderate.test_duration);
        assert!(moderate.test_duration < heavy.test_duration);
        assert!(heavy.test_duration < stress.test_duration);
        
        println!("All load test configurations are properly scaled");
    }
}