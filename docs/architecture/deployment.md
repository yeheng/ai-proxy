# AI Proxy Deployment Guide

This guide covers various deployment options for the AI Proxy service, from development to production environments.

## Table of Contents

- [Prerequisites](#prerequisites)
- [Development Deployment](#development-deployment)
- [Production Deployment](#production-deployment)
- [Docker Deployment](#docker-deployment)
- [Kubernetes Deployment](#kubernetes-deployment)
- [Configuration Management](#configuration-management)
- [Monitoring and Logging](#monitoring-and-logging)
- [Security Considerations](#security-considerations)
- [Performance Tuning](#performance-tuning)
- [Troubleshooting](#troubleshooting)

## Prerequisites

### System Requirements

- **Operating System**: Linux, macOS, or Windows
- **Memory**: Minimum 512MB RAM, recommended 2GB+ for production
- **CPU**: 1+ cores, 2+ cores recommended for production
- **Storage**: 100MB+ free space
- **Network**: Internet access for AI provider APIs

### Software Dependencies

- **Rust**: Version 1.70+ (for building from source)
- **Docker**: Version 20.10+ (for containerized deployment)
- **Kubernetes**: Version 1.20+ (for Kubernetes deployment)

## Development Deployment

### Quick Start

1. **Clone and Build**
   ```bash
   git clone <repository-url>
   cd ai-proxy
   cargo build --release
   ```

2. **Configure**
   ```bash
   cp config.example.toml config.toml
   # Edit config.toml with your API keys
   ```

3. **Run**
   ```bash
   cargo run
   # Or use the binary
   ./target/release/ai-proxy
   ```

### Development Configuration

```toml
# config.toml for development
[server]
host = "127.0.0.1"
port = 3000

[logging]
level = "debug"
format = "pretty"

[providers.openai]
api_key = "your-openai-key"
api_base = "https://api.openai.com/v1/"
enabled = true
```

### Development Commands

```bash
# Run with debug logging
cargo run -- --log-level debug

# Run with custom config
cargo run -- --config dev-config.toml

# Validate configuration
cargo run -- --validate-config

# Run tests
cargo test

# Run with file watching (requires cargo-watch)
cargo watch -x run
```

## Production Deployment

### Binary Deployment

1. **Build Optimized Binary**
   ```bash
   cargo build --release
   strip target/release/ai-proxy  # Optional: reduce binary size
   ```

2. **Create System User**
   ```bash
   sudo useradd --system --shell /bin/false --home /opt/ai-proxy ai-proxy
   sudo mkdir -p /opt/ai-proxy
   sudo chown ai-proxy:ai-proxy /opt/ai-proxy
   ```

3. **Install Binary**
   ```bash
   sudo cp target/release/ai-proxy /usr/local/bin/
   sudo chmod +x /usr/local/bin/ai-proxy
   ```

4. **Create Configuration**
   ```bash
   sudo mkdir -p /etc/ai-proxy
   sudo cp config.example.toml /etc/ai-proxy/config.toml
   sudo chown ai-proxy:ai-proxy /etc/ai-proxy/config.toml
   sudo chmod 600 /etc/ai-proxy/config.toml
   ```

### Systemd Service

Create `/etc/systemd/system/ai-proxy.service`:

```ini
[Unit]
Description=AI Proxy Service
After=network.target
Wants=network.target

[Service]
Type=simple
User=ai-proxy
Group=ai-proxy
ExecStart=/usr/local/bin/ai-proxy --config /etc/ai-proxy/config.toml
Restart=always
RestartSec=5
StandardOutput=journal
StandardError=journal
SyslogIdentifier=ai-proxy

# Security settings
NoNewPrivileges=true
PrivateTmp=true
ProtectSystem=strict
ProtectHome=true
ReadWritePaths=/var/log/ai-proxy

# Resource limits
LimitNOFILE=65536
LimitNPROC=4096

[Install]
WantedBy=multi-user.target
```

Enable and start the service:

```bash
sudo systemctl daemon-reload
sudo systemctl enable ai-proxy
sudo systemctl start ai-proxy
sudo systemctl status ai-proxy
```

### Production Configuration

```toml
# /etc/ai-proxy/config.toml
[server]
host = "0.0.0.0"
port = 3000
request_timeout_seconds = 30
max_request_size_bytes = 10485760  # 10MB

[logging]
level = "info"
format = "json"
log_requests = true
log_responses = false

[security]
cors_enabled = true
allowed_origins = ["https://your-domain.com"]
rate_limit_enabled = true

[performance]
connection_pool_size = 20
max_concurrent_requests = 1000

# Use environment variables for sensitive data
[providers.openai]
api_key = "${AI_PROXY_OPENAI_KEY}"
api_base = "https://api.openai.com/v1/"
timeout_seconds = 60
max_retries = 3
enabled = true

[providers.anthropic]
api_key = "${AI_PROXY_ANTHROPIC_KEY}"
api_base = "https://api.anthropic.com/v1/"
timeout_seconds = 60
max_retries = 3
enabled = true
```

## Docker Deployment

### Dockerfile

```dockerfile
# Multi-stage build for smaller image
FROM rust:1.75-slim as builder

WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY src ./src

# Build optimized binary
RUN cargo build --release

# Runtime image
FROM debian:bookworm-slim

# Install CA certificates for HTTPS
RUN apt-get update && \
    apt-get install -y ca-certificates && \
    rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN useradd --create-home --shell /bin/bash ai-proxy

# Copy binary
COPY --from=builder /app/target/release/ai-proxy /usr/local/bin/ai-proxy
RUN chmod +x /usr/local/bin/ai-proxy

# Create config directory
RUN mkdir -p /app/config && chown ai-proxy:ai-proxy /app/config

USER ai-proxy
WORKDIR /app

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
  CMD curl -f http://localhost:3000/health || exit 1

EXPOSE 3000

CMD ["ai-proxy", "--config", "/app/config/config.toml"]
```

### Build and Run

```bash
# Build image
docker build -t ai-proxy:latest .

# Run with environment variables
docker run -d \
  --name ai-proxy \
  -p 3000:3000 \
  -e AI_PROXY_PROVIDERS_OPENAI_API_KEY=your-openai-key \
  -e AI_PROXY_PROVIDERS_ANTHROPIC_API_KEY=your-anthropic-key \
  -e AI_PROXY_SERVER_HOST=0.0.0.0 \
  ai-proxy:latest

# Run with config file
docker run -d \
  --name ai-proxy \
  -p 3000:3000 \
  -v $(pwd)/config.toml:/app/config/config.toml:ro \
  ai-proxy:latest

# View logs
docker logs -f ai-proxy

# Health check
docker exec ai-proxy curl -f http://localhost:3000/health
```

### Docker Compose

```yaml
# docker-compose.yml
version: '3.8'

services:
  ai-proxy:
    build: .
    ports:
      - "3000:3000"
    environment:
      - AI_PROXY_SERVER_HOST=0.0.0.0
      - AI_PROXY_SERVER_PORT=3000
      - AI_PROXY_PROVIDERS_OPENAI_API_KEY=${OPENAI_API_KEY}
      - AI_PROXY_PROVIDERS_ANTHROPIC_API_KEY=${ANTHROPIC_API_KEY}
      - AI_PROXY_LOGGING_LEVEL=info
    volumes:
      - ./config.toml:/app/config/config.toml:ro
    restart: unless-stopped
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:3000/health"]
      interval: 30s
      timeout: 10s
      retries: 3
      start_period: 40s

  # Optional: Reverse proxy
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

## Kubernetes Deployment

### Namespace

```yaml
# namespace.yaml
apiVersion: v1
kind: Namespace
metadata:
  name: ai-proxy
```

### ConfigMap

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
    
    [logging]
    level = "info"
    format = "json"
    
    [security]
    cors_enabled = true
    
    [performance]
    connection_pool_size = 20
    max_concurrent_requests = 1000
```

### Secret

```yaml
# secret.yaml
apiVersion: v1
kind: Secret
metadata:
  name: ai-proxy-secrets
  namespace: ai-proxy
type: Opaque
data:
  openai-key: <base64-encoded-openai-key>
  anthropic-key: <base64-encoded-anthropic-key>
```

### Deployment

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
        - name: AI_PROXY_SERVER_HOST
          value: "0.0.0.0"
        - name: AI_PROXY_PROVIDERS_OPENAI_API_KEY
          valueFrom:
            secretKeyRef:
              name: ai-proxy-secrets
              key: openai-key
        - name: AI_PROXY_PROVIDERS_ANTHROPIC_API_KEY
          valueFrom:
            secretKeyRef:
              name: ai-proxy-secrets
              key: anthropic-key
        volumeMounts:
        - name: config
          mountPath: /app/config
          readOnly: true
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
        resources:
          requests:
            memory: "256Mi"
            cpu: "100m"
          limits:
            memory: "1Gi"
            cpu: "500m"
      volumes:
      - name: config
        configMap:
          name: ai-proxy-config
```

### Service

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

### Ingress

```yaml
# ingress.yaml
apiVersion: networking.k8s.io/v1
kind: Ingress
metadata:
  name: ai-proxy-ingress
  namespace: ai-proxy
  annotations:
    nginx.ingress.kubernetes.io/rewrite-target: /
    cert-manager.io/cluster-issuer: letsencrypt-prod
spec:
  tls:
  - hosts:
    - api.yourdomain.com
    secretName: ai-proxy-tls
  rules:
  - host: api.yourdomain.com
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
kubectl apply -f configmap.yaml
kubectl apply -f secret.yaml
kubectl apply -f deployment.yaml
kubectl apply -f service.yaml
kubectl apply -f ingress.yaml

# Check deployment status
kubectl get pods -n ai-proxy
kubectl get services -n ai-proxy
kubectl get ingress -n ai-proxy

# View logs
kubectl logs -f deployment/ai-proxy -n ai-proxy

# Scale deployment
kubectl scale deployment ai-proxy --replicas=5 -n ai-proxy
```

## Configuration Management

### Environment Variables

```bash
# Production environment variables
export AI_PROXY_SERVER_HOST=0.0.0.0
export AI_PROXY_SERVER_PORT=3000
export AI_PROXY_PROVIDERS_OPENAI_API_KEY=your-openai-key
export AI_PROXY_PROVIDERS_ANTHROPIC_API_KEY=your-anthropic-key
export AI_PROXY_LOGGING_LEVEL=info
export AI_PROXY_SECURITY_CORS_ENABLED=true
export AI_PROXY_PERFORMANCE_MAX_CONCURRENT_REQUESTS=1000
```

### Configuration Validation

```bash
# Validate configuration before deployment
ai-proxy --validate-config --config /etc/ai-proxy/config.toml

# Test configuration with dry run
ai-proxy --config /etc/ai-proxy/config.toml --validate-config
```

## Monitoring and Logging

### Log Management

```bash
# Systemd logs
journalctl -u ai-proxy -f

# Docker logs
docker logs -f ai-proxy

# Kubernetes logs
kubectl logs -f deployment/ai-proxy -n ai-proxy
```

### Health Monitoring

```bash
# Basic health check
curl http://localhost:3000/health

# Provider health check
curl http://localhost:3000/health/providers

# Automated monitoring script
#!/bin/bash
while true; do
  if ! curl -f http://localhost:3000/health > /dev/null 2>&1; then
    echo "$(date): Health check failed" >> /var/log/ai-proxy-monitor.log
    # Send alert
  fi
  sleep 30
done
```

### Metrics Collection

If metrics endpoint is implemented:

```bash
# Prometheus metrics
curl http://localhost:3000/metrics

# Custom monitoring
curl http://localhost:3000/health/providers | jq '.providers[] | select(.status != "healthy")'
```

## Security Considerations

### Network Security

```bash
# Firewall rules (iptables example)
sudo iptables -A INPUT -p tcp --dport 3000 -s trusted-ip-range -j ACCEPT
sudo iptables -A INPUT -p tcp --dport 3000 -j DROP

# Or use ufw
sudo ufw allow from trusted-ip-range to any port 3000
```

### SSL/TLS Termination

Use a reverse proxy like Nginx:

```nginx
# /etc/nginx/sites-available/ai-proxy
server {
    listen 443 ssl http2;
    server_name api.yourdomain.com;

    ssl_certificate /path/to/certificate.crt;
    ssl_certificate_key /path/to/private.key;

    location / {
        proxy_pass http://127.0.0.1:3000;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
    }
}
```

### API Key Management

```bash
# Use environment variables for sensitive data
export AI_PROXY_PROVIDERS_OPENAI_API_KEY=$(cat /secure/path/openai.key)

# Or use secret management systems
export AI_PROXY_PROVIDERS_OPENAI_API_KEY=$(vault kv get -field=api_key secret/ai-proxy/openai)
```

## Performance Tuning

### System Limits

```bash
# Increase file descriptor limits
echo "ai-proxy soft nofile 65536" >> /etc/security/limits.conf
echo "ai-proxy hard nofile 65536" >> /etc/security/limits.conf

# Kernel parameters
echo "net.core.somaxconn = 65536" >> /etc/sysctl.conf
echo "net.ipv4.tcp_max_syn_backlog = 65536" >> /etc/sysctl.conf
sysctl -p
```

### Application Tuning

```toml
# High-performance configuration
[performance]
connection_pool_size = 50
keep_alive_timeout_seconds = 300
max_concurrent_requests = 2000

[server]
request_timeout_seconds = 60
max_request_size_bytes = 52428800  # 50MB
```

### Load Balancing

```yaml
# Kubernetes HPA
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
  minReplicas: 3
  maxReplicas: 10
  metrics:
  - type: Resource
    resource:
      name: cpu
      target:
        type: Utilization
        averageUtilization: 70
```

## Troubleshooting

### Common Issues

1. **Port already in use**
   ```bash
   sudo lsof -i :3000
   sudo netstat -tulpn | grep :3000
   ```

2. **Permission denied**
   ```bash
   sudo chown -R ai-proxy:ai-proxy /opt/ai-proxy
   sudo chmod +x /usr/local/bin/ai-proxy
   ```

3. **Configuration errors**
   ```bash
   ai-proxy --validate-config --config /path/to/config.toml
   ```

4. **Memory issues**
   ```bash
   # Monitor memory usage
   top -p $(pgrep ai-proxy)
   
   # Check system resources
   free -h
   df -h
   ```

### Debug Mode

```bash
# Run with debug logging
AI_PROXY_LOGGING_LEVEL=debug ai-proxy

# Trace network issues
strace -e network ai-proxy

# Monitor file descriptors
lsof -p $(pgrep ai-proxy)
```

### Log Analysis

```bash
# Parse JSON logs
journalctl -u ai-proxy -o json | jq '.MESSAGE'

# Filter error logs
journalctl -u ai-proxy | grep ERROR

# Monitor real-time logs
tail -f /var/log/ai-proxy.log | grep -E "(ERROR|WARN)"
```

## Backup and Recovery

### Configuration Backup

```bash
# Backup configuration
tar -czf ai-proxy-config-$(date +%Y%m%d).tar.gz /etc/ai-proxy/

# Automated backup script
#!/bin/bash
BACKUP_DIR="/backup/ai-proxy"
DATE=$(date +%Y%m%d_%H%M%S)
mkdir -p $BACKUP_DIR
cp /etc/ai-proxy/config.toml $BACKUP_DIR/config-$DATE.toml
```

### Disaster Recovery

```bash
# Quick recovery steps
1. Restore configuration from backup
2. Verify API keys are still valid
3. Test connectivity to AI providers
4. Restart service
5. Verify health endpoints
```

This deployment guide provides comprehensive instructions for deploying AI Proxy in various environments. Choose the deployment method that best fits your infrastructure and requirements.