# Technology Stack & Build System

## Core Technology Stack

### Runtime & Framework

- **Runtime**: Tokio (async runtime for high-performance concurrent operations)
- **Web Framework**: Axum (type-safe routing with excellent performance)
- **HTTP Client**: Reqwest (async HTTP client for provider API calls)

### Data & Configuration

- **Serialization**: Serde (JSON handling and struct serialization/deserialization)
- **Configuration**: Figment (TOML config files with environment variable overrides)

### Error Handling

- **Structured Errors**: Thiserror (well-typed errors for API responses)
- **Error Context**: Anyhow (rich error context for internal operations)

### Logging & Observability

- **Tracing**: Tracing crate (structured logging and distributed tracing)
- **Subscriber**: Tracing-subscriber (log formatting and output)

### Async & Concurrency

- **Async Traits**: Async-trait (trait objects with async methods)
- **Streams**: Futures crate (stream processing for SSE responses)

## Build System

### Cargo Configuration

- **Edition**: 2024 (latest Rust edition)
- **Package Name**: ai-proxy
- **Version**: 0.1.0

### Common Commands

#### Development

```bash
# Build in debug mode
cargo build

# Build optimized release
cargo build --release

# Run the application
cargo run

# Run with specific config
./target/release/ai-proxy --config config.toml
```

#### Testing

```bash
# Run all tests
cargo test

# Run integration tests
cargo test --test integration_tests

# Run with output
cargo test -- --nocapture
```

#### Development Tools

```bash
# Check code without building
cargo check

# Format code
cargo fmt

# Lint code
cargo clippy

# Generate documentation
cargo doc --open
```

## Configuration Management

### File Structure

- **Primary Config**: `config.toml` (TOML format)
- **Environment Override**: `AI_PROXY_` prefixed variables
- **Example Config**: `config.example.toml`

### Configuration Hierarchy

1. Default values in code
2. TOML configuration file
3. Environment variable overrides

## Deployment

### Docker

```bash
# Build image
docker build -t ai-proxy .

# Run container
docker run -p 3000:3000 -v $(pwd)/config.toml:/app/config.toml ai-proxy
```

### Kubernetes

```bash
# Apply manifests
kubectl apply -f docs/deployment/kubernetes/

# Port forward for testing
kubectl port-forward service/ai-proxy 3000:3000
```

## Performance Considerations

- Use connection pooling for HTTP clients (single `reqwest::Client` instance)
- Implement proper streaming for large responses
- Cache provider configurations in memory
- Monitor memory usage under load
- Use appropriate timeouts for external API calls
