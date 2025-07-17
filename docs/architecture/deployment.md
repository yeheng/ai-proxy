# Deployment Architecture

## Local Development

### Quick Start

```bash
# Start local server
cargo run -- --config config.toml

# Test with curl
curl -X POST http://localhost:3000/v1/messages \
  -H "Content-Type: application/json" \
  -d '{"model":"gemini-1.5-pro-latest","messages":[{"role":"user","content":"Hello"}],"max_tokens":100}'
```

### Development Setup

1. **Clone Repository**

   ```bash
   git clone <repository-url>
   cd ai-proxy
   ```

2. **Install Dependencies**

   ```bash
   cargo build
   ```

3. **Configuration**

   ```bash
   cp config.example.toml config.toml
   # Edit config.toml with your API keys
   ```

4. **Run Tests**

   ```bash
   cargo test
   ```

## Production Deployment

### Containerization

**Dockerfile**

```dockerfile
FROM rust:1.70 as builder
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY src ./src
RUN cargo build --release

FROM debian:bullseye-slim
WORKDIR /app
COPY --from=builder /app/target/release/ai-proxy ./
COPY config.toml ./
EXPOSE 3000
CMD ["./ai-proxy", "--config", "config.toml"]
```

### Kubernetes Deployment

**Deployment.yaml**

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: ai-proxy
spec:
  replicas: 3
  selector:
    matchLabels:
      app: ai-proxy
  template:
    metadata:
      labels:
        app: ai-proxy
    spec:
      containers:
      - name: ai-proxy
        image: ai-proxy:latest
        ports:
        - containerPort: 3000
        env:
        - name: AI_PROXY_SERVER__PORT
          value: "3000"
        - name: AI_PROXY_PROVIDERS__GEMINI__API_KEY
          valueFrom:
            secretKeyRef:
              name: ai-proxy-secrets
              key: gemini-api-key
```

### Orchestration with Kubernetes

- **Service Mesh**: Istio or Linkerd for advanced traffic management
- **Auto-scaling**: Horizontal Pod Autoscaler based on CPU/memory metrics
- **Service Discovery**: Kubernetes DNS for service discovery
- **Load Balancing**: Kubernetes Service with load balancer

### Monitoring and Observability

#### Metrics Collection

- **Prometheus**: Metrics scraping and storage
- **Grafana**: Visualization and dashboards
- **Custom Metrics**: Request count, latency, error rates

#### Health Checks

- **Liveness Probe**: `/health/live`
- **Readiness Probe**: `/health/ready`
- **Startup Probe**: `/health/startup`

#### Logging

- **Structured Logging**: JSON format with correlation IDs
- **Centralized Logging**: ELK Stack or Fluentd
- **Log Levels**: Configurable via environment variables

### Cloud Deployment Options

#### AWS

- **ECS**: Amazon Elastic Container Service
- **EKS**: Amazon Elastic Kubernetes Service
- **Fargate**: Serverless container deployment

#### Google Cloud

- **GKE**: Google Kubernetes Engine
- **Cloud Run**: Serverless container deployment

#### Azure

- **AKS**: Azure Kubernetes Service
- **Container Instances**: Serverless container deployment

## Environment Configuration

### Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `AI_PROXY_SERVER__HOST` | Server bind address | `0.0.0.0` |
| `AI_PROXY_SERVER__PORT` | Server port | `3000` |
| `AI_PROXY_PROVIDERS__GEMINI__API_KEY` | Gemini API key | - |
| `AI_PROXY_PROVIDERS__OPENAI__API_KEY` | OpenAI API key | - |
| `AI_PROXY_PROVIDERS__ANTHROPIC__API_KEY` | Anthropic API key | - |
| `RUST_LOG` | Log level | `info` |

### Secrets Management

#### Kubernetes Secrets

```yaml
apiVersion: v1
kind: Secret
metadata:
  name: ai-proxy-secrets
type: Opaque
data:
  gemini-api-key: <base64-encoded-key>
  openai-api-key: <base64-encoded-key>
  anthropic-api-key: <base64-encoded-key>
```

#### AWS Secrets Manager

```bash
aws secretsmanager create-secret \
  --name ai-proxy/api-keys \
  --secret-string '{"gemini":"key1","openai":"key2","anthropic":"key3"}'
```

## Performance Tuning

### Rust Optimization

- **Release Build**: Use `cargo build --release` for production
- **Link-time Optimization**: Enable LTO in Cargo.toml
- **Profile-guided Optimization**: Use `cargo pgo` for maximum performance

### System Optimization

- **CPU**: Multi-core systems for concurrent request handling
- **Memory**: Sufficient RAM for connection pooling and caching
- **Network**: High-bandwidth, low-latency network for provider communication

### Scaling Strategies

- **Horizontal Scaling**: Multiple instances behind load balancer
- **Vertical Scaling**: More CPU/memory per instance
- **Auto-scaling**: Based on request volume and resource usage
