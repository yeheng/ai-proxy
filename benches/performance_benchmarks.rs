/// Criterion Benchmarks for AI Proxy Performance
///
/// This module provides detailed benchmarking using the Criterion framework
/// for precise performance measurements and regression detection.

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId, Throughput};
use ai_proxy::{
    config::{Config, LoggingConfig, PerformanceConfig, ProviderDetail, SecurityConfig, ServerConfig},
    providers::{ProviderRegistry, anthropic::{AnthropicRequest, Message}},
    server::{AppState, create_app},
};
use axum::{
    body::Body,
    http::{Request, StatusCode},
    response::Response,
};
use reqwest::Client;
use serde_json::{json, Value};
use std::{
    collections::HashMap,
    sync::{Arc, atomic::{AtomicUsize, Ordering}},
    time::Duration,
};
use tokio::{sync::RwLock, runtime::Runtime};
use tower::ServiceExt;
use wiremock::{Mock, MockServer, ResponseTemplate, matchers::{method, path}};

/// Setup mock server for benchmarking
async fn setup_benchmark_server() -> (MockServer, AppState) {
    let server = MockServer::start().await;
    
    // Setup basic mock responses
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "bench-123",
            "object": "chat.completion",
            "created": 1234567890,
            "model": "gpt-4",
            "choices": [{
                "index": 0,
                "message": {
                    "role": "assistant",
                    "content": "Benchmark response"
                },
                "finish_reason": "stop"
            }],
            "usage": {
                "prompt_tokens": 10,
                "completion_tokens": 5,
                "total_tokens": 15
            }
        })))
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string("data: {\"type\":\"message_start\",\"message\":{\"id\":\"bench-stream\"}}\n\ndata: {\"type\":\"content_block_delta\",\"delta\":{\"text\":\"Hello\"}}\n\ndata: {\"type\":\"message_stop\"}\n\n")
                .insert_header("content-type", "text/event-stream")
        )
        .mount(&server)
        .await;

    let mut providers = HashMap::new();
    providers.insert("openai".to_string(), ProviderDetail {
        api_key: "bench-key".to_string(),
        api_base: format!("{}/v1/", server.uri()),
        models: Some(vec!["gpt-4".to_string()]),
        timeout_seconds: 30,
        max_retries: 3,
        enabled: true,
        rate_limit: None,
    });

    let config = Config {
        server: ServerConfig {
            host: "127.0.0.1".to_string(),
            port: 0,
            request_timeout_seconds: 30,
            max_request_size_bytes: 1024 * 1024,
        },
        providers,
        logging: LoggingConfig {
            level: "error".to_string(), // Reduce logging for benchmarks
            format: "json".to_string(),
            log_requests: false,
            log_responses: false,
        },
        security: SecurityConfig::default(),
        performance: PerformanceConfig::default(),
    };

    let http_client = Client::new();
    let provider_registry = Arc::new(RwLock::new(
        ProviderRegistry::new(&config, http_client.clone()).unwrap(),
    ));
    let metrics = Arc::new(ai_proxy::metrics::MetricsCollector::new());

    let app_state = AppState {
        config: Arc::new(config),
        http_client,
        provider_registry,
        metrics,
    };

    (server, app_state)
}

/// Benchmark single request processing
fn bench_single_request(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let (_server, app_state) = rt.block_on(setup_benchmark_server());
    let app = create_app(app_state);

    c.bench_function("single_request", |b| {
        b.iter(|| {
            rt.block_on(async {
            let request = Request::builder()
                .method("POST")
                .uri("/v1/messages")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_string(&json!({
                    "model": "gpt-4",
                    "messages": [{"role": "user", "content": "Benchmark test"}],
                    "max_tokens": 50,
                    "stream": false
                })).unwrap()))
                .unwrap();

                let response = app.clone().oneshot(request).await.unwrap();
                black_box(response);
            })
        });
    });
}

/// Benchmark concurrent request processing
fn bench_concurrent_requests(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let (_server, app_state) = rt.block_on(setup_benchmark_server());
    let app = create_app(app_state);

    let mut group = c.benchmark_group("concurrent_requests");
    
    for concurrency in [1, 5, 10, 25, 50].iter() {
        group.throughput(Throughput::Elements(*concurrency as u64));
        group.bench_with_input(
            BenchmarkId::new("concurrent", concurrency),
            concurrency,
            |b, &concurrency| {
                b.iter(|| {
                    rt.block_on(async {
                    let mut handles = Vec::new();
                    
                    for _ in 0..concurrency {
                        let app_clone = app.clone();
                        let handle = tokio::spawn(async move {
                            let request = Request::builder()
                                .method("POST")
                                .uri("/v1/messages")
                                .header("content-type", "application/json")
                                .body(Body::from(serde_json::to_string(&json!({
                                    "model": "gpt-4",
                                    "messages": [{"role": "user", "content": "Concurrent benchmark"}],
                                    "max_tokens": 50,
                                    "stream": false
                                })).unwrap()))
                                .unwrap();

                            app_clone.oneshot(request).await.unwrap()
                        });
                        handles.push(handle);
                    }
                    
                        for handle in handles {
                            let response = handle.await.unwrap();
                            black_box(response);
                        }
                    })
                });
            },
        );
    }
    
    group.finish();
}

/// Benchmark streaming request processing
fn bench_streaming_requests(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let (_server, app_state) = rt.block_on(setup_benchmark_server());
    let app = create_app(app_state);

    c.bench_function("streaming_request", |b| {
        b.iter(|| {
            rt.block_on(async {
            let request = Request::builder()
                .method("POST")
                .uri("/v1/messages")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_string(&json!({
                    "model": "gpt-4",
                    "messages": [{"role": "user", "content": "Streaming benchmark test"}],
                    "max_tokens": 100,
                    "stream": true
                })).unwrap()))
                .unwrap();

            let response = app.clone().oneshot(request).await.unwrap();
            
                // Consume the streaming response
                let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
                let body_string = String::from_utf8(body_bytes.to_vec()).unwrap();
                black_box(body_string);
            })
        });
    });
}

/// Benchmark request parsing and validation
fn bench_request_parsing(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    
    let test_requests = vec![
        json!({
            "model": "gpt-4",
            "messages": [{"role": "user", "content": "Short message"}],
            "max_tokens": 50
        }),
        json!({
            "model": "claude-3-sonnet",
            "messages": [
                {"role": "user", "content": "Medium length message with more content"},
                {"role": "assistant", "content": "Previous response"},
                {"role": "user", "content": "Follow-up question"}
            ],
            "max_tokens": 200,
            "temperature": 0.7,
            "top_p": 0.9
        }),
        json!({
            "model": "gemini-pro",
            "messages": [{"role": "user", "content": "Very long message with extensive content that would typically be used in real-world scenarios where users provide detailed context and ask complex questions that require comprehensive responses from the AI system"}],
            "max_tokens": 500,
            "temperature": 0.8,
            "top_p": 0.95,
            "stream": true
        }),
    ];

    let mut group = c.benchmark_group("request_parsing");
    
    for (i, request_json) in test_requests.iter().enumerate() {
        group.throughput(Throughput::Bytes(request_json.to_string().len() as u64));
        group.bench_with_input(
            BenchmarkId::new("parse", i),
            request_json,
            |b, request_json| {
                b.iter(|| {
                    let request_str = serde_json::to_string(request_json).unwrap();
                    let parsed: AnthropicRequest = serde_json::from_str(&request_str).unwrap();
                    black_box(parsed);
                });
            },
        );
    }
    
    group.finish();
}

/// Benchmark response serialization
fn bench_response_serialization(c: &mut Criterion) {
    use ai_proxy::providers::anthropic::{AnthropicResponse, ContentBlock, Usage};
    
    let test_responses = vec![
        AnthropicResponse {
            id: "resp-1".to_string(),
            model: "gpt-4".to_string(),
            content: vec![ContentBlock {
                type_field: "text".to_string(),
                text: "Short response".to_string(),
            }],
            usage: Usage {
                input_tokens: 10,
                output_tokens: 5,
            },
        },
        AnthropicResponse {
            id: "resp-2".to_string(),
            model: "claude-3-sonnet".to_string(),
            content: vec![ContentBlock {
                type_field: "text".to_string(),
                text: "Medium length response with more detailed content and explanations".to_string(),
            }],
            usage: Usage {
                input_tokens: 50,
                output_tokens: 25,
            },
        },
        AnthropicResponse {
            id: "resp-3".to_string(),
            model: "gemini-pro".to_string(),
            content: vec![ContentBlock {
                type_field: "text".to_string(),
                text: "Very comprehensive and detailed response that would typically be generated in real-world usage scenarios where the AI provides extensive information, analysis, examples, and thorough explanations to complex user queries".to_string(),
            }],
            usage: Usage {
                input_tokens: 200,
                output_tokens: 150,
            },
        },
    ];

    let mut group = c.benchmark_group("response_serialization");
    
    for (i, response) in test_responses.iter().enumerate() {
        group.bench_with_input(
            BenchmarkId::new("serialize", i),
            response,
            |b, response| {
                b.iter(|| {
                    let serialized = serde_json::to_string(response).unwrap();
                    black_box(serialized);
                });
            },
        );
    }
    
    group.finish();
}

/// Benchmark memory allocation patterns
fn bench_memory_allocation(c: &mut Criterion) {
    c.bench_function("memory_allocation", |b| {
        b.iter(|| {
            // Simulate typical memory allocation patterns
            let mut data = Vec::new();
            
            for i in 0..1000 {
                let message = format!("Message number {} with some content", i);
                data.push(message);
            }
            
            // Simulate processing
            let processed: Vec<String> = data.iter()
                .map(|s| format!("Processed: {}", s))
                .collect();
            
            black_box(processed);
        });
    });
}

/// Benchmark provider registry operations
fn bench_provider_registry(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let (_server, app_state) = rt.block_on(setup_benchmark_server());

    c.bench_function("provider_lookup", |b| {
        b.iter(|| {
            rt.block_on(async {
                let registry = app_state.provider_registry.read().await;
                let provider = registry.get_provider_for_model("gpt-4");
                black_box(provider);
            })
        });
    });
}

criterion_group!(
    benches,
    bench_single_request,
    bench_concurrent_requests,
    bench_streaming_requests,
    bench_request_parsing,
    bench_response_serialization,
    bench_memory_allocation,
    bench_provider_registry
);

criterion_main!(benches);