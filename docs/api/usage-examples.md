# AI Proxy API Usage Examples

This document provides comprehensive examples of how to use the AI Proxy API with different programming languages and scenarios.

## Table of Contents

- [Basic Chat Completion](#basic-chat-completion)
- [Streaming Responses](#streaming-responses)
- [Different Providers](#different-providers)
- [Error Handling](#error-handling)
- [Model Management](#model-management)
- [Health Checks](#health-checks)
- [Programming Language Examples](#programming-language-examples)

## Basic Chat Completion

### Simple Chat Request

```bash
curl -X POST http://localhost:3000/v1/messages \
  -H "Content-Type: application/json" \
  -d '{
    "model": "gpt-3.5-turbo",
    "messages": [
      {
        "role": "user",
        "content": "Hello! How are you today?"
      }
    ],
    "max_tokens": 100
  }'
```

### Response Format

```json
{
  "id": "msg_01ABC123",
  "model": "gpt-3.5-turbo",
  "content": [
    {
      "type": "text",
      "text": "Hello! I'm doing well, thank you for asking. How can I help you today?"
    }
  ],
  "usage": {
    "input_tokens": 12,
    "output_tokens": 18,
    "total_tokens": 30
  }
}
```

### Multi-turn Conversation

```bash
curl -X POST http://localhost:3000/v1/messages \
  -H "Content-Type: application/json" \
  -d '{
    "model": "claude-3-haiku-20240307",
    "messages": [
      {
        "role": "user",
        "content": "What is the capital of France?"
      },
      {
        "role": "assistant",
        "content": "The capital of France is Paris."
      },
      {
        "role": "user",
        "content": "What is its population?"
      }
    ],
    "max_tokens": 150
  }'
```

## Streaming Responses

### Enable Streaming

```bash
curl -X POST http://localhost:3000/v1/messages \
  -H "Content-Type: application/json" \
  -d '{
    "model": "gpt-4",
    "messages": [
      {
        "role": "user",
        "content": "Write a short poem about artificial intelligence."
      }
    ],
    "max_tokens": 200,
    "stream": true
  }'
```

### Streaming Response Format

```
data: {"type":"message_start","message":{"id":"msg_01ABC123","model":"gpt-4","content":[],"usage":{"input_tokens":12,"output_tokens":0,"total_tokens":12}}}

data: {"type":"content_block_start","index":0,"content_block":{"type":"text","text":""}}

data: {"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"In"}}

data: {"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":" circuits"}}

data: {"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":" bright"}}

data: {"type":"content_block_stop","index":0}

data: {"type":"message_delta","delta":{"usage":{"output_tokens":45,"total_tokens":57}}}

data: {"type":"message_stop"}
```

## Different Providers

### OpenAI Models

```bash
# GPT-3.5 Turbo
curl -X POST http://localhost:3000/v1/messages \
  -H "Content-Type: application/json" \
  -d '{
    "model": "gpt-3.5-turbo",
    "messages": [{"role": "user", "content": "Hello!"}],
    "max_tokens": 50
  }'

# GPT-4
curl -X POST http://localhost:3000/v1/messages \
  -H "Content-Type: application/json" \
  -d '{
    "model": "gpt-4",
    "messages": [{"role": "user", "content": "Explain quantum computing"}],
    "max_tokens": 200
  }'
```

### Anthropic Models

```bash
# Claude 3 Haiku
curl -X POST http://localhost:3000/v1/messages \
  -H "Content-Type: application/json" \
  -d '{
    "model": "claude-3-haiku-20240307",
    "messages": [{"role": "user", "content": "Hello!"}],
    "max_tokens": 50
  }'

# Claude 3.5 Sonnet
curl -X POST http://localhost:3000/v1/messages \
  -H "Content-Type: application/json" \
  -d '{
    "model": "claude-3-5-sonnet-20241022",
    "messages": [{"role": "user", "content": "Write a Python function"}],
    "max_tokens": 300
  }'
```

### Google Gemini Models

```bash
# Gemini Pro
curl -X POST http://localhost:3000/v1/messages \
  -H "Content-Type: application/json" \
  -d '{
    "model": "gemini-pro",
    "messages": [{"role": "user", "content": "Hello!"}],
    "max_tokens": 50
  }'

# Gemini 1.5 Pro
curl -X POST http://localhost:3000/v1/messages \
  -H "Content-Type: application/json" \
  -d '{
    "model": "gemini-1.5-pro-latest",
    "messages": [{"role": "user", "content": "Analyze this data"}],
    "max_tokens": 500
  }'
```

## Error Handling

### Invalid Model Error

```bash
curl -X POST http://localhost:3000/v1/messages \
  -H "Content-Type: application/json" \
  -d '{
    "model": "invalid-model",
    "messages": [{"role": "user", "content": "Hello!"}],
    "max_tokens": 50
  }'
```

Response:
```json
{
  "error": {
    "type": "provider_not_found",
    "message": "Provider not found for model: invalid-model"
  }
}
```

### Validation Error

```bash
curl -X POST http://localhost:3000/v1/messages \
  -H "Content-Type: application/json" \
  -d '{
    "model": "gpt-3.5-turbo",
    "max_tokens": 50
  }'
```

Response:
```json
{
  "error": {
    "type": "validation_error",
    "message": "Request validation failed: missing required field 'messages'"
  }
}
```

## Model Management

### List Available Models

```bash
curl -X GET http://localhost:3000/v1/models
```

Response:
```json
{
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
      "created": 1709856000,
      "owned_by": "anthropic",
      "provider": "anthropic"
    },
    {
      "id": "gemini-pro",
      "object": "model",
      "created": 1701388800,
      "owned_by": "google",
      "provider": "gemini"
    }
  ]
}
```

### Refresh Models

```bash
curl -X POST http://localhost:3000/v1/models/refresh
```

## Health Checks

### System Health

```bash
curl -X GET http://localhost:3000/health
```

Response:
```json
{
  "status": "healthy",
  "timestamp": "2024-01-15T10:30:00Z",
  "version": "0.1.0",
  "uptime_seconds": 3600
}
```

### Provider Health

```bash
curl -X GET http://localhost:3000/health/providers
```

Response:
```json
{
  "status": "healthy",
  "providers": {
    "openai": {
      "status": "healthy",
      "response_time_ms": 150,
      "last_check": "2024-01-15T10:29:45Z"
    },
    "anthropic": {
      "status": "healthy",
      "response_time_ms": 200,
      "last_check": "2024-01-15T10:29:45Z"
    },
    "gemini": {
      "status": "degraded",
      "response_time_ms": 2000,
      "last_check": "2024-01-15T10:29:45Z",
      "error": "High response time"
    }
  }
}
```

## Programming Language Examples

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
        "max_tokens": 150
    }
    
    response = requests.post(url, headers=headers, json=data)
    return response.json()

# Streaming chat completion
def chat_completion_stream(message, model="gpt-3.5-turbo"):
    url = "http://localhost:3000/v1/messages"
    headers = {"Content-Type": "application/json"}
    data = {
        "model": model,
        "messages": [{"role": "user", "content": message}],
        "max_tokens": 150,
        "stream": True
    }
    
    response = requests.post(url, headers=headers, json=data, stream=True)
    
    for line in response.iter_lines():
        if line:
            line = line.decode('utf-8')
            if line.startswith('data: '):
                data = line[6:]  # Remove 'data: ' prefix
                if data.strip() != '[DONE]':
                    try:
                        event = json.loads(data)
                        yield event
                    except json.JSONDecodeError:
                        continue

# Usage examples
if __name__ == "__main__":
    # Basic completion
    result = chat_completion("Hello, how are you?")
    print(json.dumps(result, indent=2))
    
    # Streaming completion
    print("\nStreaming response:")
    for event in chat_completion_stream("Tell me a short story"):
        print(event)
```

### JavaScript/Node.js

```javascript
const axios = require('axios');

// Basic chat completion
async function chatCompletion(message, model = 'gpt-3.5-turbo') {
    const url = 'http://localhost:3000/v1/messages';
    const data = {
        model: model,
        messages: [{ role: 'user', content: message }],
        max_tokens: 150
    };
    
    try {
        const response = await axios.post(url, data);
        return response.data;
    } catch (error) {
        console.error('Error:', error.response?.data || error.message);
        throw error;
    }
}

// Streaming chat completion
async function chatCompletionStream(message, model = 'gpt-3.5-turbo') {
    const url = 'http://localhost:3000/v1/messages';
    const data = {
        model: model,
        messages: [{ role: 'user', content: message }],
        max_tokens: 150,
        stream: true
    };
    
    try {
        const response = await axios.post(url, data, {
            responseType: 'stream'
        });
        
        response.data.on('data', (chunk) => {
            const lines = chunk.toString().split('\n');
            for (const line of lines) {
                if (line.startsWith('data: ')) {
                    const data = line.slice(6);
                    if (data.trim() !== '[DONE]') {
                        try {
                            const event = JSON.parse(data);
                            console.log(event);
                        } catch (e) {
                            // Skip invalid JSON
                        }
                    }
                }
            }
        });
        
    } catch (error) {
        console.error('Error:', error.response?.data || error.message);
        throw error;
    }
}

// Usage examples
async function main() {
    try {
        // Basic completion
        const result = await chatCompletion('Hello, how are you?');
        console.log(JSON.stringify(result, null, 2));
        
        // Streaming completion
        console.log('\nStreaming response:');
        await chatCompletionStream('Tell me a short story');
        
    } catch (error) {
        console.error('Failed:', error);
    }
}

main();
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

type ChatResponse struct {
    ID      string `json:"id"`
    Model   string `json:"model"`
    Content []struct {
        Type string `json:"type"`
        Text string `json:"text"`
    } `json:"content"`
    Usage struct {
        InputTokens  int `json:"input_tokens"`
        OutputTokens int `json:"output_tokens"`
        TotalTokens  int `json:"total_tokens"`
    } `json:"usage"`
}

func chatCompletion(message, model string) (*ChatResponse, error) {
    url := "http://localhost:3000/v1/messages"
    
    req := ChatRequest{
        Model:     model,
        Messages:  []Message{{Role: "user", Content: message}},
        MaxTokens: 150,
    }
    
    jsonData, err := json.Marshal(req)
    if err != nil {
        return nil, err
    }
    
    resp, err := http.Post(url, "application/json", bytes.NewBuffer(jsonData))
    if err != nil {
        return nil, err
    }
    defer resp.Body.Close()
    
    body, err := io.ReadAll(resp.Body)
    if err != nil {
        return nil, err
    }
    
    var chatResp ChatResponse
    err = json.Unmarshal(body, &chatResp)
    if err != nil {
        return nil, err
    }
    
    return &chatResp, nil
}

func main() {
    result, err := chatCompletion("Hello, how are you?", "gpt-3.5-turbo")
    if err != nil {
        fmt.Printf("Error: %v\n", err)
        return
    }
    
    fmt.Printf("Response: %+v\n", result)
}
```

## Advanced Usage

### Custom Parameters

```bash
curl -X POST http://localhost:3000/v1/messages \
  -H "Content-Type: application/json" \
  -d '{
    "model": "gpt-4",
    "messages": [
      {
        "role": "system",
        "content": "You are a helpful assistant that responds in a formal tone."
      },
      {
        "role": "user",
        "content": "Explain machine learning"
      }
    ],
    "max_tokens": 300,
    "temperature": 0.7,
    "top_p": 0.9,
    "stream": false
  }'
```

### Batch Processing

```python
import asyncio
import aiohttp
import json

async def process_batch(messages, model="gpt-3.5-turbo"):
    url = "http://localhost:3000/v1/messages"
    
    async with aiohttp.ClientSession() as session:
        tasks = []
        for message in messages:
            data = {
                "model": model,
                "messages": [{"role": "user", "content": message}],
                "max_tokens": 100
            }
            task = session.post(url, json=data)
            tasks.append(task)
        
        responses = await asyncio.gather(*tasks)
        results = []
        
        for response in responses:
            result = await response.json()
            results.append(result)
        
        return results

# Usage
messages = [
    "What is AI?",
    "Explain machine learning",
    "What is deep learning?"
]

results = asyncio.run(process_batch(messages))
for i, result in enumerate(results):
    print(f"Response {i+1}: {result}")
```

## Monitoring and Metrics

### Get System Metrics

```bash
curl -X GET http://localhost:3000/metrics
```

Response:
```json
{
  "requests_total": 1250,
  "requests_per_second": 5.2,
  "average_response_time_ms": 850,
  "error_rate": 0.02,
  "active_connections": 12,
  "provider_stats": {
    "openai": {
      "requests": 800,
      "errors": 15,
      "avg_response_time_ms": 750
    },
    "anthropic": {
      "requests": 300,
      "errors": 5,
      "avg_response_time_ms": 900
    },
    "gemini": {
      "requests": 150,
      "errors": 8,
      "avg_response_time_ms": 1200
    }
  }
}
```

## Best Practices

1. **Error Handling**: Always check response status and handle errors appropriately
2. **Rate Limiting**: Implement client-side rate limiting to avoid overwhelming the proxy
3. **Timeouts**: Set appropriate timeouts for your requests
4. **Streaming**: Use streaming for long responses to improve user experience
5. **Model Selection**: Choose the appropriate model for your use case and budget
6. **Monitoring**: Monitor your usage and performance metrics regularly

## Troubleshooting

### Common Issues

1. **Connection Refused**: Make sure the AI Proxy server is running
2. **Invalid API Key**: Check your provider API keys in the configuration
3. **Model Not Found**: Verify the model name is correct and the provider is configured
4. **Rate Limit Exceeded**: Implement backoff and retry logic
5. **Timeout Errors**: Increase timeout values in configuration if needed

### Debug Mode

Run the server with debug logging:

```bash
AI_PROXY_LOGGING_LEVEL=debug cargo run
```

Or use the command line flag:

```bash
cargo run -- --log-level debug
```