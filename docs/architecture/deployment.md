# AI Proxy Deployment Guide

This guide covers various deployment options for the AI Proxy service, from local development to production environments.

## Table of Contents

- [Prerequisites](#prerequisites)
- [Local Development](#local-development)
- [Docker Deployment](#docker-deployment)
- [Kubernetes Deployment](#kubernetes-deployment)
- [Production Considerations](#production-considerations)
- [Monitoring and Logging](#monitoring-and-logging)
- [Security](#security)
- [Troubleshooting](#troubleshooting)

## Prerequisites

### System Requirements

- **CPU**: 2+ cores recommended
- **Memory**: 512MB minimum, 2GB+ recommended for production
- **Storage**: 100MB for application, additional space for logs
- **Network**: Outbound HTTPS access to AI provider APIs

### Software Requirements

- **Rust**: 1.70+ (for building from source)
- **Docker**: 20.10+ (for containerized deployment)
- **Kubernetes**: 1.20+ (for Kubernetes deployment)

## Local Development

### Building from Source

1. **Clone the repository**:
   ```bash
   git clone <repository-url>
   cd ai-proxy
   ```

2. **Install Rust** (if not already installed):
   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   source ~/.cargo/env
   ```

3. **Build the application**:
   ```bash
   # Debug build (faster compilation)
   cargo build
   
   # Release build (optimized)
   cargo build --release
   ```

4. **Configure the application**:
   ```bash
   cp config.example.toml config.toml
   # Edit config.toml with your API keys
   ```

5. **Run the application**:
   ```bash
   # Debug mode
   cargo run
   
   # Release mode
   ./target/release/ai-proxy
   
   # With custom config
   ./target/release/ai-proxy --config /path/to/config.toml
   ```

### Development Commands

```bash
# Check code without building
cargo check

# Run tests
cargo test

# Format code
cargo fmt

# Lint code
cargo clippy

# Generate documentation
cargo doc --open

# Run with debug logging
AI_PROXY_LOGGING_LEVEL=debug cargo run
```

## Docker Deployment

### Building Docker Image

1. **Create Dockerfile**:
   ```dockerfile
   # Multi-stage build for smaller image size
   FROM rust:1.75-slim as builder
   
   # Install build dependencies
   RUN apt-get update && apt-get install -y \
       pkg-config \
       libssl-dev \
       && rm -rf /var/lib/apt/lists/*
   
   # Create app directory
   WORKDIR /app
   
   # Copy dependency files
   COPY Cargo.toml Cargo.lock ./
   
   # Create dummy main.rs to build dependencies
   RUN mkdir src && echo "fn main() {}" > src/main.rs
   RUN cargo build --release && rm -rf src
   
   # Copy source code
   COPY src ./src
   
   # Build application
   RUN touch src/main.rs && cargo build --release
   
   # Runtime stage
   FROM debian:bookworm-slim
   
   # Install runtime dependencies
   RUN apt-get update && apt-get install -y \
       ca-certificates \
       && rm -rf /var/lib/apt/lists/*
   
   # Create app user
   RUN useradd -r -s /bin/false aiproxy
   
   # Create app directory
   WORKDIR /app
   
   # Copy binary from builder stage
   COPY --from=builder /app/target/release/ai-proxy ./
   
   # Copy configuration template
   COPY config.example.toml ./
   
   # Change ownership
   RUN chown -R aiproxy:aiproxy /app
   
   # Switch to app user
   USER aiproxy
   
   # Expose port
   EXPOSE 3000
   
   # Health check
   HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
     CMD curl -f http://localhost:3000/health || exit 1
   
   # Run application
   CMD ["./ai-proxy"]
   ```

2. **Build the image**:
   ```bash
   docker build -t ai-proxy:latest .
   ```

3. **Run the container**:
   ```bash
   # Basic run
   docker run -p 3000:3000 ai-proxy:latest
   
   # With custom configuration
   docker run -p 3000:3000 \
     -v $(pwd)/config.toml:/app/config.toml:ro \
     ai-proxy:latest
   
   # With environment variables
   docker run -p 3000:3000 \
     -e AI_PROXY_PROVIDERS_OPENAI_API_KEY=your-key \
     -e AI_PROXY_PROVIDERS_ANTHROPIC_API_KEY=your-key \
     ai-proxy:latest
   ```

### Docker Compose

Create `docker-compose.yml`:

```yaml
version: '3.8'

services:
  ai-proxy:
    build: .
    ports:
      - "3000:3000"
    environment:
      - AI_PROXY_SERVER_HOST=0.0.0.0
      - AI_PROXY_LOGGING_LEVEL=info
      - AI_PROXY_PROVIDERS_OPENAI_API_KEY=${OPENAI_API_KEY}
      - AI_PROXY_PROVIDERS_ANTHROPIC_API_KEY=${ANTHROPIC_API_KEY}
      - AI_PROXY_PROVIDERS_GEMINI_API_KEY=${GEMINI_API_KEY}
    volumes:
      - ./config.toml:/app/config.toml:ro
      - ./logs:/app/logs
    restart: unless-stopped
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:3000/health"]
      interval: 30s
      timeout: 10s
      retries: 3
      start_period: 40s

  # Optional: Reverse proxy with SSL
  nginx:
    image: nginx:alpine
    ports:
      - "80:80"
      - "443:443"
    volumes:
      - ./nginx.conf:/etc/nginx/nginx.conf:ro
      - ./ssl:/etc/nginx/ssl:ro
    depends_on:
      - ai-proxy
    restart: unless-stopped
```

Run with Docker Compose:

```bash
# Start services
docker-compose up -d

# View logs
docker-compose logs -f ai-proxy

# Stop services
docker-compose down
```

## Kubernetes Deployment

### Basic Deployment

1. **Create namespace**:
   ```yaml
   # namespace.yaml
   apiVersion: v1
   kind: Namespace
   metadata:
     name: ai-proxy
   ```

2. **Create ConfigMap**:
   ```yaml
   # configmap.yaml
   apiVersion: v1
   kind: ConfigMap
   metadata:
     name: ai-proxy-config
     namespace: ai-proxy
   data:
     config.toml: |
       [server]
       host = "0.0.0.0"
       port = 3000
       request_timeout_seconds = 30
       max_request_size_bytes = 1048576
       
       [providers.openai]
       api_key = "${OPENAI_API_KEY}"
       api_base = "https://api.openai.com/v1/"
       models = ["gpt-3.5-turbo", "gpt-4"]
       timeout_seconds = 60
       max_retries = 3
       enabled = true
       
       [providers.anthropic]
       api_key = "${ANTHROPIC_API_KEY}"
       api_base = "https://api.anthropic.com/v1/"
       models = ["claude-3-haiku-20240307", "claude-3-5-sonnet-20241022"]
       timeout_seconds = 60
       max_retries = 3
       enabled = true
       
       [logging]
       level = "info"
       format = "json"
   ```

3. **Create Secret**:
   ```yaml
   # secret.yaml
   apiVersion: v1
   kind: Secret
   metadata:
     name: ai-proxy-secrets
     namespace: ai-proxy
   type: Opaque
   data:
     openai-api-key: <base64-encoded-key>
     anthropic-api-key: <base64-encoded-key>
     gemini-api-key: <base64-encoded-key>
   ```

4. **Create Deployment**:
   ```yaml
   # deployment.yaml
   apiVersion: apps/v1
   kind: Deployment
   metadata:
     name: ai-proxy
     namespace: ai-proxy
     labels:
       app: ai-proxy
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
           - name: AI_PROXY_PROVIDERS_OPENAI_API_KEY
             valueFrom:
               secretKeyRef:
                 name: ai-proxy-secrets
                 key: openai-api-key
           - name: AI_PROXY_PROVIDERS_ANTHROPIC_API_KEY
             valueFrom:
               secretKeyRef:
                 name: ai-proxy-secrets
                 key: anthropic-api-key
           - name: AI_PROXY_PROVIDERS_GEMINI_API_KEY
             valueFrom:
               secretKeyRef:
                 name: ai-proxy-secrets
                 key: gemini-api-key
           volumeMounts:
           - name: config
             mountPath: /app/config.toml
             subPath: config.toml
           resources:
             requests:
               memory: "256Mi"
               cpu: "250m"
             limits:
               memory: "512Mi"
               cpu: "500m"
           livenessProbe:
             httpGet:
               path: /health
               port: 3000
             initialDelaySeconds: 30
             periodSeconds: 10
           readinessProbe:
             httpGet:
               path: /health
               port: 3000
             initialDelaySeconds: 5
             periodSeconds: 5
         volumes:
         - name: config
           configMap:
             name: ai-proxy-config
   ```

5. **Create Service**:
   ```yaml
   # service.yaml
   apiVersion: v1
   kind: Service
   metadata:
     name: ai-proxy-service
     namespace: ai-proxy
   spec:
     selector:
       app: ai-proxy
     ports:
     - protocol: TCP
       port: 80
       targetPort: 3000
     type: ClusterIP
   ```

6. **Create Ingress** (optional):
   ```yaml
   # ingress.yaml
   apiVersion: networking.k8s.io/v1
   kind: Ingress
   metadata:
     name: ai-proxy-ingress
     namespace: ai-proxy
     annotations:
       kubernetes.io/ingress.class: nginx
       cert-manager.io/cluster-issuer: letsencrypt-prod
   spec:
     tls:
     - hosts:
       - ai-proxy.yourdomain.com
       secretName: ai-proxy-tls
     rules:
     - host: ai-proxy.yourdomain.com
       http:
         paths:
         - path: /
           pathType: Prefix
           backend:
             service:
               name: ai-proxy-service
               port:
                 number: 80
   ```

### Deploy to Kubernetes

```bash
# Apply all manifests
kubectl apply -f namespace.yaml
kubectl apply -f secret.yaml
kubectl apply -f configmap.yaml
kubectl apply -f deployment.yaml
kubectl apply -f service.yaml
kubectl apply -f ingress.yaml

# Check deployment status
kubectl get pods -n ai-proxy
kubectl get services -n ai-proxy
kubectl get ingress -n ai-proxy

# View logs
kubectl logs -f deployment/ai-proxy -n ai-proxy

# Port forward for testing
kubectl port-forward service/ai-proxy-service 3000:80 -n ai-proxy
```

### Horizontal Pod Autoscaler

```yaml
# hpa.yaml
apiVersion: autoscaling/v2
kind: HorizontalPodAutoscaler
metadata:
  name: ai-proxy-hpa
  namespace: ai-proxy
spec:
  scaleTargetRef:
    apiVersion: apps/v1
    kind: Deployment
    name: ai-proxy
  minReplicas: 2
  maxReplicas: 10
  metrics:
  - type: Resource
    resource:
      name: cpu
      target:
        type: Utilization
        averageUtilization: 70
  - type: Resource
    resource:
      name: memory
      target:
        type: Utilization
        averageUtilization: 80
```

## Production Considerations

### Performance Tuning

1. **Resource Allocation**:
   ```toml
   [performance]
   connection_pool_size = 20
   keep_alive_timeout_seconds = 60
   max_concurrent_requests = 200
   ```

2. **JVM-like Tuning** (Rust equivalent):
   ```bash
   # Set environment variables
   export RUST_LOG=info
   export RUST_BACKTRACE=1
   
   # For production, consider:
   export MALLOC_ARENA_MAX=2  # Limit memory arenas
   ```

### Load Balancing

Use a load balancer (nginx, HAProxy, or cloud load balancer):

```nginx
# nginx.conf
upstream ai_proxy {
    least_conn;
    server ai-proxy-1:3000 max_fails=3 fail_timeout=30s;
    server ai-proxy-2:3000 max_fails=3 fail_timeout=30s;
    server ai-proxy-3:3000 max_fails=3 fail_timeout=30s;
}

server {
    listen 80;
    server_name ai-proxy.yourdomain.com;
    
    location / {
        proxy_pass http://ai_proxy;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
        
        # Timeouts
        proxy_connect_timeout 5s;
        proxy_send_timeout 60s;
        proxy_read_timeout 60s;
        
        # Buffering for streaming
        proxy_buffering off;
        proxy_cache off;
    }
    
    location /health {
        proxy_pass http://ai_proxy;
        access_log off;
    }
}
```

### Database and Persistence

For production, consider adding:

1. **Redis for caching**:
   ```yaml
   # redis.yaml
   apiVersion: apps/v1
   kind: Deployment
   metadata:
     name: redis
   spec:
     replicas: 1
     selector:
       matchLabels:
         app: redis
     template:
       metadata:
         labels:
           app: redis
       spec:
         containers:
         - name: redis
           image: redis:7-alpine
           ports:
           - containerPort: 6379
   ```

2. **PostgreSQL for analytics**:
   ```yaml
   # postgres.yaml
   apiVersion: apps/v1
   kind: StatefulSet
   metadata:
     name: postgres
   spec:
     serviceName: postgres
     replicas: 1
     selector:
       matchLabels:
         app: postgres
     template:
       metadata:
         labels:
           app: postgres
       spec:
         containers:
         - name: postgres
           image: postgres:15
           env:
           - name: POSTGRES_DB
             value: aiproxy
           - name: POSTGRES_USER
             value: aiproxy
           - name: POSTGRES_PASSWORD
             valueFrom:
               secretKeyRef:
                 name: postgres-secret
                 key: password
           volumeMounts:
           - name: postgres-storage
             mountPath: /var/lib/postgresql/data
     volumeClaimTemplates:
     - metadata:
         name: postgres-storage
       spec:
         accessModes: ["ReadWriteOnce"]
         resources:
           requests:
             storage: 10Gi
   ```

## Monitoring and Logging

### Prometheus Metrics

Add metrics endpoint configuration:

```toml
[monitoring]
metrics_enabled = true
metrics_port = 9090
prometheus_format = true
```

### Grafana Dashboard

Create a Grafana dashboard to monitor:

- Request rate and latency
- Error rates by provider
- Active connections
- Memory and CPU usage
- Provider response times

### Log Aggregation

Use ELK stack or similar:

```yaml
# filebeat.yaml
apiVersion: v1
kind: ConfigMap
metadata:
  name: filebeat-config
data:
  filebeat.yml: |
    filebeat.inputs:
    - type: container
      paths:
        - /var/log/containers/*ai-proxy*.log
      processors:
      - add_kubernetes_metadata:
          host: ${NODE_NAME}
          matchers:
          - logs_path:
              logs_path: "/var/log/containers/"
    
    output.elasticsearch:
      hosts: ["elasticsearch:9200"]
    
    setup.kibana:
      host: "kibana:5601"
```

## Security

### API Key Management

1. **Use Kubernetes Secrets**:
   ```bash
   kubectl create secret generic ai-proxy-secrets \
     --from-literal=openai-api-key=your-key \
     --from-literal=anthropic-api-key=your-key \
     -n ai-proxy
   ```

2. **Use external secret management**:
   - AWS Secrets Manager
   - Azure Key Vault
   - HashiCorp Vault
   - Google Secret Manager

### Network Security

1. **Network Policies**:
   ```yaml
   apiVersion: networking.k8s.io/v1
   kind: NetworkPolicy
   metadata:
     name: ai-proxy-netpol
     namespace: ai-proxy
   spec:
     podSelector:
       matchLabels:
         app: ai-proxy
     policyTypes:
     - Ingress
     - Egress
     ingress:
     - from:
       - namespaceSelector:
           matchLabels:
             name: ingress-nginx
       ports:
       - protocol: TCP
         port: 3000
     egress:
     - to: []
       ports:
       - protocol: TCP
         port: 443  # HTTPS to AI providers
   ```

2. **TLS Configuration**:
   ```toml
   [security]
   tls_enabled = true
   cert_file = "/etc/ssl/certs/server.crt"
   key_file = "/etc/ssl/private/server.key"
   ```

### Authentication

Enable API key authentication:

```toml
[security]
api_keys = [
    "your-client-api-key-1",
    "your-client-api-key-2"
]
```

## Troubleshooting

### Common Issues

1. **Pod CrashLoopBackOff**:
   ```bash
   kubectl describe pod <pod-name> -n ai-proxy
   kubectl logs <pod-name> -n ai-proxy --previous
   ```

2. **Service not accessible**:
   ```bash
   kubectl get endpoints -n ai-proxy
   kubectl port-forward service/ai-proxy-service 3000:80 -n ai-proxy
   ```

3. **High memory usage**:
   ```bash
   kubectl top pods -n ai-proxy
   kubectl describe pod <pod-name> -n ai-proxy
   ```

### Debug Commands

```bash
# Check all resources
kubectl get all -n ai-proxy

# Describe deployment
kubectl describe deployment ai-proxy -n ai-proxy

# Check events
kubectl get events -n ai-proxy --sort-by='.lastTimestamp'

# Execute into pod
kubectl exec -it <pod-name> -n ai-proxy -- /bin/sh

# Check configuration
kubectl get configmap ai-proxy-config -n ai-proxy -o yaml
```

### Performance Monitoring

```bash
# Monitor resource usage
kubectl top pods -n ai-proxy --containers

# Check HPA status
kubectl get hpa -n ai-proxy

# View detailed metrics
kubectl describe hpa ai-proxy-hpa -n ai-proxy
```

## Backup and Recovery

### Configuration Backup

```bash
# Backup all configurations
kubectl get all,configmap,secret -n ai-proxy -o yaml > ai-proxy-backup.yaml

# Restore from backup
kubectl apply -f ai-proxy-backup.yaml
```

### Disaster Recovery

1. **Multi-region deployment**
2. **Database replication**
3. **Configuration versioning**
4. **Automated failover**

## Scaling Strategies

### Vertical Scaling

```yaml
resources:
  requests:
    memory: "512Mi"
    cpu: "500m"
  limits:
    memory: "1Gi"
    cpu: "1000m"
```

### Horizontal Scaling

```yaml
spec:
  replicas: 5  # Increase replica count
```

### Auto-scaling

Use HPA with custom metrics:

```yaml
metrics:
- type: Pods
  pods:
    metric:
      name: requests_per_second
    target:
      type: AverageValue
      averageValue: "100"
```

This deployment guide provides comprehensive coverage of deploying AI Proxy in various environments, from development to production-ready Kubernetes clusters.