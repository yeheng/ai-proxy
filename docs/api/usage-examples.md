# AI Proxy API Usage Examples

This document provides comprehensive examples of how to use the AI Proxy API with different programming languages and tools.

## Table of Contents

- [Quick Start](#quick-start)
- [Authentication](#authentication)
- [Chat Completion](#chat-completion)
- [Streaming Responses](#streaming-responses)
- [Model Management](#model-management)
- [Health Checks](#health-checks)
- [Error Handling](#error-handling)
- [Language Examples](#language-examples)
- [Advanced Usage](#advanced-usage)

## Quick Start

### 1. Start the Server

```bash
# Using cargo
cargo run

# Using the binary
./target/release/ai-proxy

# With custom config
./target/release/ai-proxy --config my-config.toml

# With command line overrides
./target/release/ai-proxy --host 0.0.0.0 --port 8080 --log-level debug
```

### 2. Basic API Call

```bash
curl -X POST http://localhost:3000/v1/messages \
  -H "Content-Type: application/json" \
  -d '{
    "model": "gpt-3.5-turbo",
    "messages": [
      {"role": "user", "content": "Hello, world!"}
    ],
    "max_tokens": 100
  }'
```

## Authentication

### No Authentication (Default)

By default, AI Proxy doesn't require authentication:

```bash
curl -X POST http://localhost:3000/v1/messages \
  -H "Content-Type: application/json" \
  -d '{"model": "gpt-3.5-turbo", "messages": [{"role": "user", "content": "Hello"}], "max_tokens": 50}'
```

### API Key Authentication

If you've configured API keys in your config:

```bash
curl -X POST http://localhost:3000/v1/messages \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer your-api-key" \
  -d '{"model": "gpt-3.5-turbo", "messages": [{"role": "user", "content": "Hello"}], "max_tokens": 50}'
```

## Chat Completion

### Basic Chat Completion

```bash
curl -X POST http://localhost:3000/v1/messages \
  -H "Content-Type: application/json" \
  -d '{
    "model": "gpt-3.5-turbo",
    "messages": [
      {"role": "system", "content": "You are a helpful assistant."},
      {"role": "user", "content": "What is the capital of France?"}
    ],
    "max_tokens": 100,
    "temperature": 0.7
  }'
```

### Multi-turn Conversation

```bash
curl -X POST http://localhost:3000/v1/messages \
  -H "Content-Type: application/json" \
  -d '{
    "model": "claude-3-haiku-20240307",
    "messages": [
      {"role": "user", "content": "Hello, how are you?"},
      {"role": "assistant", "content": "Hello! I am doing well, thank you for asking. How can I help you today?"},
      {"role": "user", "content": "Can you explain quantum computing?"}
    ],
    "max_tokens": 200
  }'
```

### Using Different Providers

#### OpenAI Models
```bash
curl -X POST http://localhost:3000/v1/messages \
  -H "Content-Type: application/json" \
  -d '{
    "model": "gpt-4",
    "messages": [{"role": "user", "content": "Explain machine learning"}],
    "max_tokens": 150
  }'
```

#### Anthropic Models
```bash
curl -X POST http://localhost:3000/v1/messages \
  -H "Content-Type: application/json" \
  -d '{
    "model": "claude-3-5-sonnet-20241022",
    "messages": [{"role": "user", "content": "Write a haiku about programming"}],
    "max_tokens": 100
  }'
```

#### Google Gemini Models
```bash
curl -X POST http://localhost:3000/v1/messages \
  -H "Content-Type: application/json" \
  -d '{
    "model": "gemini-1.5-pro-latest",
    "messages": [{"role": "user", "content": "What are the benefits of renewable energy?"}],
    "max_tokens": 200
  }'
```

## Streaming Responses

### Basic Streaming

```bash
curl -X POST http://localhost:3000/v1/messages \
  -H "Content-Type: application/json" \
  -d '{
    "model": "gpt-3.5-turbo",
    "messages": [{"role": "user", "content": "Tell me a short story"}],
    "max_tokens": 200,
    "stream": true
  }'
```

### Streaming with Server-Sent Events

```bash
curl -N -X POST http://localhost:3000/v1/messages \
  -H "Content-Type: application/json" \
  -H "Accept: text/event-stream" \
  -d '{
    "model": "claude-3-haiku-20240307",
    "messages": [{"role": "user", "content": "Count from 1 to 10"}],
    "max_tokens": 100,
    "stream": true
  }'
```

## Model Management

### List Available Models

```bash
curl http://localhost:3000/v1/models
```

Example response:
```json
{
  "object": "list",
  "data": [
    {
      "id": "gpt-3.5-turbo",
      "object": "model",
      "created": 1677610602,
      "owned_by": "openai",
      "provider": "openai"
    },
    {
      "id": "claude-3-haiku-20240307",
      "object": "model",
      "created": 1677610602,
      "owned_by": "anthropic",
      "provider": "anthropic"
    }
  ]
}
```

## Health Checks

### Basic Health Check

```bash
curl http://localhost:3000/health
```

Example response:
```json
{
  "status": "healthy",
  "timestamp": "2024-01-15T10:30:00Z",
  "version": "0.1.0"
}
```

### Provider Health Check

```bash
curl http://localhost:3000/health/providers
```

Example response:
```json
{
  "status": "healthy",
  "providers": {
    "openai": {
      "status": "healthy",
      "response_time_ms": 150,
      "last_check": "2024-01-15T10:30:00Z"
    },
    "anthropic": {
      "status": "healthy",
      "response_time_ms": 200,
      "last_check": "2024-01-15T10:30:00Z"
    },
    "gemini": {
      "status": "unhealthy",
      "error": "API key invalid",
      "last_check": "2024-01-15T10:30:00Z"
    }
  }
}
```

## Error Handling

### Invalid Model

```bash
curl -X POST http://localhost:3000/v1/messages \
  -H "Content-Type: application/json" \
  -d '{
    "model": "invalid-model",
    "messages": [{"role": "user", "content": "Hello"}],
    "max_tokens": 50
  }'
```

Response:
```json
{
  "error": {
    "type": "provider_not_found",
    "message": "Provider not found for model: invalid-model",
    "code": 404
  }
}
```

### Malformed Request

```bash
curl -X POST http://localhost:3000/v1/messages \
  -H "Content-Type: application/json" \
  -d '{"invalid": "request"}'
```

Response:
```json
{
  "error": {
    "type": "bad_request",
    "message": "Missing required field: model",
    "code": 400
  }
}
```

## Language Examples

### Python

```python
import requests
import json

# Basic chat completion
def chat_completion(message, model="gpt-3.5-turbo"):
    url = "http://localhost:3000/v1/messages"
    headers = {"Content-Type": "application/json"}
    data = {
        "model": model,
        "messages": [{"role": "user", "content": message}],
        "max_tokens": 100
    }
    
    response = requests.post(url, headers=headers, json=data)
    return response.json()

# Streaming chat completion
def streaming_chat(message, model="gpt-3.5-turbo"):
    url = "http://localhost:3000/v1/messages"
    headers = {"Content-Type": "application/json"}
    data = {
        "model": model,
        "messages": [{"role": "user", "content": message}],
        "max_tokens": 200,
        "stream": True
    }
    
    response = requests.post(url, headers=headers, json=data, stream=True)
    for line in response.iter_lines():
        if line:
            print(line.decode('utf-8'))

# Usage
result = chat_completion("Hello, AI!")
print(json.dumps(result, indent=2))

# Streaming example
streaming_chat("Tell me a joke")
```

### JavaScript/Node.js

```javascript
const axios = require('axios');

// Basic chat completion
async function chatCompletion(message, model = 'gpt-3.5-turbo') {
  try {
    const response = await axios.post('http://localhost:3000/v1/messages', {
      model: model,
      messages: [{ role: 'user', content: message }],
      max_tokens: 100
    });
    return response.data;
  } catch (error) {
    console.error('Error:', error.response?.data || error.message);
  }
}

// Streaming chat completion
async function streamingChat(message, model = 'gpt-3.5-turbo') {
  try {
    const response = await axios.post('http://localhost:3000/v1/messages', {
      model: model,
      messages: [{ role: 'user', content: message }],
      max_tokens: 200,
      stream: true
    }, {
      responseType: 'stream'
    });

    response.data.on('data', (chunk) => {
      console.log(chunk.toString());
    });
  } catch (error) {
    console.error('Error:', error.response?.data || error.message);
  }
}

// Usage
chatCompletion('Hello, AI!').then(result => {
  console.log(JSON.stringify(result, null, 2));
});

// Streaming example
streamingChat('Tell me about space exploration');
```

### Go

```go
package main

import (
    "bytes"
    "encoding/json"
    "fmt"
    "io"
    "net/http"
)

type Message struct {
    Role    string `json:"role"`
    Content string `json:"content"`
}

type ChatRequest struct {
    Model     string    `json:"model"`
    Messages  []Message `json:"messages"`
    MaxTokens int       `json:"max_tokens"`
    Stream    bool      `json:"stream,omitempty"`
}

func chatCompletion(message, model string) error {
    req := ChatRequest{
        Model:     model,
        Messages:  []Message{{Role: "user", Content: message}},
        MaxTokens: 100,
    }

    jsonData, err := json.Marshal(req)
    if err != nil {
        return err
    }

    resp, err := http.Post("http://localhost:3000/v1/messages", 
        "application/json", bytes.NewBuffer(jsonData))
    if err != nil {
        return err
    }
    defer resp.Body.Close()

    body, err := io.ReadAll(resp.Body)
    if err != nil {
        return err
    }

    fmt.Println(string(body))
    return nil
}

func main() {
    err := chatCompletion("Hello, AI!", "gpt-3.5-turbo")
    if err != nil {
        fmt.Printf("Error: %v\n", err)
    }
}
```

### Rust

```rust
use reqwest;
use serde_json::json;
use tokio;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = reqwest::Client::new();
    
    let request_body = json!({
        "model": "gpt-3.5-turbo",
        "messages": [
            {"role": "user", "content": "Hello, AI!"}
        ],
        "max_tokens": 100
    });

    let response = client
        .post("http://localhost:3000/v1/messages")
        .header("Content-Type", "application/json")
        .json(&request_body)
        .send()
        .await?;

    let response_text = response.text().await?;
    println!("{}", response_text);

    Ok(())
}
```

## Advanced Usage

### Custom Headers and Parameters

```bash
curl -X POST http://localhost:3000/v1/messages \
  -H "Content-Type: application/json" \
  -H "X-Request-ID: my-unique-id" \
  -d '{
    "model": "gpt-4",
    "messages": [{"role": "user", "content": "Explain quantum physics"}],
    "max_tokens": 300,
    "temperature": 0.8,
    "top_p": 0.9,
    "presence_penalty": 0.1,
    "frequency_penalty": 0.1
  }'
```

### Batch Processing

```bash
# Process multiple requests in parallel
for i in {1..5}; do
  curl -X POST http://localhost:3000/v1/messages \
    -H "Content-Type: application/json" \
    -d "{\"model\":\"gpt-3.5-turbo\",\"messages\":[{\"role\":\"user\",\"content\":\"Question $i: What is $i + $i?\"}],\"max_tokens\":50}" &
done
wait
```

### Load Testing

```bash
# Simple load test with Apache Bench
ab -n 100 -c 10 -p request.json -T application/json http://localhost:3000/v1/messages

# Where request.json contains:
# {"model":"gpt-3.5-turbo","messages":[{"role":"user","content":"Hello"}],"max_tokens":10}
```

### Monitoring and Metrics

```bash
# Check system metrics (if implemented)
curl http://localhost:3000/metrics

# Monitor health over time
watch -n 5 'curl -s http://localhost:3000/health/providers | jq'
```

## Configuration Examples

### Environment Variables

```bash
# Set environment variables
export AI_PROXY_SERVER_HOST=0.0.0.0
export AI_PROXY_SERVER_PORT=8080
export AI_PROXY_PROVIDERS_OPENAI_API_KEY=your-openai-key
export AI_PROXY_PROVIDERS_ANTHROPIC_API_KEY=your-anthropic-key
export AI_PROXY_LOGGING_LEVEL=debug

# Run with environment variables
cargo run
```

### Docker Usage

```bash
# Build Docker image
docker build -t ai-proxy .

# Run with environment variables
docker run -p 3000:3000 \
  -e AI_PROXY_PROVIDERS_OPENAI_API_KEY=your-key \
  -e AI_PROXY_PROVIDERS_ANTHROPIC_API_KEY=your-key \
  ai-proxy

# Run with config file
docker run -p 3000:3000 \
  -v $(pwd)/config.toml:/app/config.toml \
  ai-proxy
```

## Troubleshooting

### Common Issues

1. **Server not responding**
   ```bash
   # Check if server is running
   curl http://localhost:3000/health
   ```

2. **Invalid API keys**
   ```bash
   # Check provider health
   curl http://localhost:3000/health/providers
   ```

3. **Model not found**
   ```bash
   # List available models
   curl http://localhost:3000/v1/models
   ```

4. **Rate limiting**
   ```bash
   # Check rate limit headers in response
   curl -I -X POST http://localhost:3000/v1/messages \
     -H "Content-Type: application/json" \
     -d '{"model":"gpt-3.5-turbo","messages":[{"role":"user","content":"test"}],"max_tokens":10}'
   ```

### Debug Mode

```bash
# Run in debug mode
AI_PROXY_LOGGING_LEVEL=debug cargo run

# Or with command line
cargo run -- --log-level debug
```

## Best Practices

1. **Always handle errors gracefully**
2. **Use appropriate timeouts for your use case**
3. **Implement retry logic for transient failures**
4. **Monitor API usage and costs**
5. **Use streaming for long responses**
6. **Implement proper authentication in production**
7. **Set up health check monitoring**
8. **Use connection pooling for high-throughput scenarios**

## Support

For more information:
- Check the [API documentation](rest-api.md)
- Review the [configuration guide](../architecture/deployment.md)
- Run the test script: `./scripts/test_api.sh`
- Check server logs for detailed error information