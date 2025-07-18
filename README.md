# AI Proxy 🚀

A high-performance Rust-based API gateway that unifies multiple AI providers (Gemini, OpenAI, Anthropic, etc.) into a single, consistent interface.

## ✨ Key Features

- **🎯 Unified API**: Single endpoint for all AI providers
- **⚡ High Performance**: Built with Rust for maximum efficiency
- **🔄 Real-time Streaming**: SSE support for streaming responses
- **🔧 Easy Provider Switching**: Change models via configuration
- **📊 Extensible**: Built for adding caching, monitoring, and authentication
- **🛡️ Production Ready**: Docker and Kubernetes deployment support

## 🚀 Quick Start

### 1. Clone and Build

```bash
git clone https://github.com/yeheng/ai-proxy.git
cd ai-proxy
cargo build --release
```

### 2. Configuration

Copy the example configuration and add your API keys:

```bash
cp config.example.toml config.toml
# Edit config.toml with your API keys
```

### 3. Run the Server

```bash
./target/release/ai-proxy --config config.toml
# Server will start on http://localhost:3000
```

### 4. Test the API

```bash
# Non-streaming request
curl -X POST http://localhost:3000/v1/messages \
  -H "Content-Type: application/json" \
  -d '{
    "model": "gemini-1.5-pro-latest",
    "messages": [{"role": "user", "content": "Hello, how are you?"}],
    "max_tokens": 100
  }'

# Streaming request
curl -X POST http://localhost:3000/v1/messages \
  -H "Content-Type: application/json" \
  -d '{
    "model": "gpt-4",
    "messages": [{"role": "user", "content": "Write a haiku"}],
    "max_tokens": 100,
    "stream": true
  }'

# List available models
curl http://localhost:3000/v1/models

# Refresh models from providers
curl -X POST http://localhost:3000/v1/models/refresh

# Check system health
curl http://localhost:3000/health

# Check provider health
curl http://localhost:3000/health/providers
```

## 📡 API Endpoints

- **POST** `/v1/messages` - Chat completion (streaming and non-streaming)
- **GET** `/v1/models` - List available models from all providers
- **POST** `/v1/models/refresh` - Refresh models by fetching latest from providers
- **GET** `/health` - System health check
- **GET** `/health/providers` - Provider health status

## 📋 Configuration

### Basic Configuration (`config.toml`)

```toml
[server]
host = "0.0.0.0"
port = 3000

[providers.gemini]
api_key = "YOUR_GEMINI_API_KEY"
api_base = "https://generativelanguage.googleapis.com/v1beta/models/"
models = ["gemini-1.5-pro-latest", "gemini-1.5-flash-latest"]

[providers.openai]
api_key = "YOUR_OPENAI_API_KEY"
api_base = "https://api.openai.com/v1/"
models = ["gpt-4", "gpt-3.5-turbo"]

[providers.anthropic]
api_key = "YOUR_ANTHROPIC_API_KEY"
api_base = "https://api.anthropic.com/v1/"
models = ["claude-3-5-sonnet-20241022", "claude-3-opus-20240229"]
```

### Environment Variables

```bash
export AI_PROXY_PROVIDERS__GEMINI__API_KEY="your-gemini-key"
export AI_PROXY_PROVIDERS__OPENAI__API_KEY="your-openai-key"
export AI_PROXY_SERVER__PORT=8080
```

## 🏗️ Architecture Overview

### Core Design Patterns

- **Adapter Pattern**: Standardizes different AI provider APIs
- **Gateway Pattern**: Centralized request routing and response handling
- **Stream Processing**: Real-time response streaming with format conversion

### System Components

```
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│                 │    │                 │    │                 │
│   Client App    │───▶│   AI Proxy      │───▶│   AI Provider   │
│                 │    │   (Gateway)     │    │   (Gemini,      │
└─────────────────┘    │                 │    │   OpenAI, etc.) │
                       └─────────────────┘    └─────────────────┘
```

### Technology Stack

- **Runtime**: Tokio (async runtime)
- **Web Framework**: Axum (type-safe routing)
- **HTTP Client**: Reqwest (async HTTP client)
- **Serialization**: Serde (JSON handling)
- **Configuration**: Figment (config management)
- **Error Handling**: Thiserror (structured errors)

## 📚 Documentation

### 📖 [Complete Documentation](./docs/README.md)

Our documentation is organized into clear sections:

- **[📋 Architecture Guide](./docs/architecture/overview.md)** - System design and patterns
- **[🔌 API Reference](./docs/api/rest-api.md)** - REST API documentation with examples
- **[⚙️ Deployment Guide](./docs/architecture/deployment.md)** - Local and production deployment
- **[🛠️ Development Guide](./docs/guides/module-design.md)** - Module design and implementation

### Quick Navigation

1. **New users**: Start with the [Architecture Overview](./docs/architecture/overview.md)
2. **API users**: Check the [REST API Reference](./docs/api/rest-api.md)
3. **Developers**: Use the [Module Design Guide](./docs/guides/module-design.md)
4. **DevOps**: Follow the [Deployment Guide](./docs/architecture/deployment.md)

## 🐳 Docker Deployment

### Quick Start with Docker

```bash
# Build the image
docker build -t ai-proxy .

# Run with configuration
docker run -p 3000:3000 \
  -v $(pwd)/config.toml:/app/config.toml \
  ai-proxy
```

### Docker Compose

```yaml
version: '3.8'
services:
  ai-proxy:
    build: .
    ports:
      - "3000:3000"
    volumes:
      - ./config.toml:/app/config.toml
    environment:
      - RUST_LOG=info
```

## ☸️ Kubernetes Deployment

### Using Helm (Future)

```bash
# Install with Helm (when available)
helm install ai-proxy ./charts/ai-proxy \
  --set config.gemini.apiKey=YOUR_GEMINI_KEY \
  --set config.openai.apiKey=YOUR_OPENAI_KEY
```

### Direct K8s Deployment

```bash
kubectl apply -f docs/deployment/kubernetes/
kubectl port-forward service/ai-proxy 3000:3000
```

## 🔧 Supported Models

### Google Gemini

- `gemini-1.5-pro-latest`
- `gemini-1.5-flash-latest`
- `gemini-pro`
- `gemini-pro-vision`

### OpenAI

- `gpt-4`
- `gpt-4-turbo-preview`
- `gpt-3.5-turbo`
- `gpt-3.5-turbo-16k`

### Anthropic Claude

- `claude-3-5-sonnet-20241022`
- `claude-3-5-haiku-20241022`
- `claude-3-opus-20240229`

## 🧪 Testing

### Unit Tests

```bash
cargo test
```

### Integration Tests

```bash
# Test with example configuration
cargo test --test integration_tests
```

### Load Testing

```bash
# Using hey
hey -n 1000 -c 10 \
  -H "Content-Type: application/json" \
  -d '{"model":"gemini-1.5-pro-latest","messages":[{"role":"user","content":"test"}],"max_tokens":10}' \
  http://localhost:3000/v1/messages
```

## 🤝 Contributing

We welcome contributions! Please see our [Contributing Guide](./CONTRIBUTING.md) for details.

### Development Setup

```bash
git clone https://github.com/yeheng/ai-proxy.git
cd ai-proxy
cargo build
```

### Adding New Providers

1. Create new provider module in `src/providers/[provider].rs`
2. Implement the `AIProvider` trait
3. Add configuration to `config.toml` schema
4. Update provider matching logic in `server.rs`
5. Add comprehensive tests

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## 🙏 Acknowledgments

- Built with the amazing Rust ecosystem
- Inspired by the need for unified AI API interfaces
- Community contributions and feedback welcome

---

## 📞 Support

- **Issues**: [GitHub Issues](https://github.com/yeheng/ai-proxy/issues)
- **Discussions**: [GitHub Discussions](https://github.com/yeheng/ai-proxy/discussions)
- **Documentation**: [Full Documentation](./docs/README.md)
