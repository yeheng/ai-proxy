# REST API Documentation

## Overview

The AI Proxy provides a unified REST API that works across multiple AI providers (Gemini, OpenAI, Anthropic) using a consistent interface based on the Anthropic API format.

## Base URL

- **Local Development**: `http://localhost:3000/v1`
- **Production**: `https://api.ai-proxy.dev/v1`

## Authentication

All API requests require authentication using a Bearer token:

```
Authorization: Bearer YOUR_API_KEY
```

## Endpoints

### Chat Completions

Create a chat completion using the specified model.

**Endpoint**: `POST /v1/messages`

#### Request

**Headers**:

```
Content-Type: application/json
Authorization: Bearer YOUR_API_KEY
```

**Body Schema**:

```json
{
  "model": "string",
  "messages": [
    {
      "role": "user" | "assistant",
      "content": "string"
    }
  ],
  "max_tokens": "integer (1-4096)",
  "stream": "boolean (default: false)",
  "temperature": "number (0.0-2.0, default: 1.0)",
  "top_p": "number (0.0-1.0, default: 1.0)"
}
```

#### Request Examples

**Non-streaming Request**:

```bash
curl -X POST http://localhost:3000/v1/messages \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer YOUR_API_KEY" \
  -d '{
    "model": "gemini-1.5-pro-latest",
    "messages": [
      {"role": "user", "content": "Hello, how are you?"}
    ],
    "max_tokens": 100,
    "temperature": 0.7
  }'
```

**Streaming Request**:

```bash
curl -X POST http://localhost:3000/v1/messages \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer YOUR_API_KEY" \
  -d '{
    "model": "gpt-4",
    "messages": [
      {"role": "user", "content": "Write a haiku about programming"}
    ],
    "max_tokens": 100,
    "stream": true
  }'
```

#### Response

**Non-streaming Response**:

```json
{
  "id": "msg_123abc456def",
  "model": "gemini-1.5-pro-latest",
  "content": [
    {
      "type": "text",
      "text": "Hello! I'm doing well, thank you for asking. I'm here to help you with any questions or tasks you have."
    }
  ],
  "usage": {
    "input_tokens": 10,
    "output_tokens": 25
  }
}
```

**Streaming Response**:
The streaming response uses Server-Sent Events (SSE) format:

```
event: message_start
data: {"type":"message_start","message":{"id":"msg_123abc","model":"gpt-4","content":[],"usage":null}}

event: content_block_delta
data: {"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"Programming"}}

event: content_block_delta
data: {"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":" is the art"}}

event: message_stop
data: {"type":"message_stop"}
```

### List Models

Get a list of available models from all configured providers.

**Endpoint**: `GET /v1/models`

#### Request

**Headers**:

```
Authorization: Bearer YOUR_API_KEY
```

#### Response

```json
{
  "object": "list",
  "data": [
    {
      "id": "gemini-1.5-pro-latest",
      "object": "model",
      "created": 1714560000,
      "owned_by": "google"
    },
    {
      "id": "gpt-4",
      "object": "model",
      "created": 1714560000,
      "owned_by": "openai"
    },
    {
      "id": "claude-3-sonnet-20240229",
      "object": "model",
      "created": 1714560000,
      "owned_by": "anthropic"
    }
  ]
}
```

## Error Handling

### Error Response Format

All errors follow a consistent format:

```json
{
  "error": {
    "message": "Error description",
    "type": "error_type",
    "param": "parameter_name",
    "code": "error_code"
  }
}
```

### Common Error Codes

| HTTP Status | Error Type | Description |
|-------------|------------|-------------|
| 400 | invalid_request_error | Invalid request format or parameters |
| 401 | authentication_error | Invalid or missing API key |
| 404 | not_found | Model not found or not configured |
| 429 | rate_limit_exceeded | Rate limit exceeded |
| 500 | api_error | Internal server error |
| 503 | service_unavailable | Provider service unavailable |

### Error Examples

**400 Bad Request**:

```json
{
  "error": {
    "message": "Invalid request format",
    "type": "invalid_request_error",
    "param": "messages"
  }
}
```

**404 Model Not Found**:

```json
{
  "error": {
    "message": "No provider configured for model: unknown-model",
    "type": "not_found",
    "param": "model"
  }
}
```

**429 Rate Limit Exceeded**:

```json
{
  "error": {
    "message": "Rate limit exceeded for provider: gemini",
    "type": "rate_limit_exceeded",
    "code": "rate_limit_exceeded"
  }
}
```

### Refresh Models

Refresh the model list by fetching the latest models from all configured providers.

**Endpoint**: `POST /v1/models/refresh`

#### Request

**Headers**:

```
Authorization: Bearer YOUR_API_KEY
```

#### Response

**Success Response (200)**:

```json
{
  "status": "success",
  "message": "Models refreshed successfully",
  "provider_stats": {
    "openai": 5,
    "gemini": 3,
    "anthropic": 4
  },
  "timestamp": "2024-01-15T10:30:00Z"
}
```

**Error Response (500)**:

```json
{
  "error": {
    "message": "Failed to refresh models from some providers",
    "type": "provider_error"
  }
}
```

## Model Support

### Supported Models

The following models are currently supported:

#### Google Gemini

- `gemini-1.5-pro-latest`
- `gemini-1.5-flash-latest`
- `gemini-pro`
- `gemini-pro-vision`

#### OpenAI

- `gpt-4`
- `gpt-4-turbo-preview`
- `gpt-3.5-turbo`
- `gpt-3.5-turbo-16k`

#### Anthropic

- `claude-3-5-sonnet-20241022`
- `claude-3-5-haiku-20241022`
- `claude-3-opus-20240229`

### Model Selection

Models are selected based on the `model` parameter in the request. The proxy automatically routes to the appropriate provider:

- `gemini-*` → Google Gemini API
- `gpt-*` → OpenAI API
- `claude-*` → Anthropic Claude API

## Rate Limits

### Provider Rate Limits

Each provider has its own rate limits:

| Provider | Requests per minute | Tokens per minute |
|----------|--------------------|-------------------|
| Gemini | 60 | 60,000 |
| OpenAI | 3,000 | 250,000 |
| Anthropic | 1,000 | 400,000 |

### Proxy Rate Limits

The proxy enforces additional rate limits:

- **Per API Key**: 100 requests per minute
- **Per IP**: 20 requests per minute

## SDKs and Libraries

### Official SDKs

#### JavaScript/TypeScript

```bash
npm install ai-proxy-client
```

```javascript
import { AIProxyClient } from 'ai-proxy-client';

const client = new AIProxyClient({
  apiKey: 'YOUR_API_KEY',
  baseURL: 'http://localhost:3000/v1'
});

const response = await client.messages.create({
  model: 'gemini-1.5-pro-latest',
  messages: [
    { role: 'user', content: 'Hello, world!' }
  ],
  max_tokens: 100
});
```

#### Python

```bash
pip install ai-proxy-client
```

```python
from ai_proxy_client import AIProxyClient

client = AIProxyClient(
    api_key="YOUR_API_KEY",
    base_url="http://localhost:3000/v1"
)

response = client.messages.create(
    model="gemini-1.5-pro-latest",
    messages=[{"role": "user", "content": "Hello, world!"}],
    max_tokens=100
)
```

## Testing

### API Testing with curl

**Test basic functionality**:

```bash
curl -X POST http://localhost:3000/v1/messages \
  -H "Content-Type: application/json" \
  -d '{
    "model": "gemini-1.5-pro-latest",
    "messages": [{"role": "user", "content": "Say hello"}],
    "max_tokens": 10
  }'
```

**Test streaming**:

```bash
curl -X POST http://localhost:3000/v1/messages \
  -H "Content-Type: application/json" \
  -d '{
    "model": "gpt-4",
    "messages": [{"role": "user", "content": "Count to 5"}],
    "max_tokens": 20,
    "stream": true
  }'
```

**Test model listing**:

```bash
curl http://localhost:3000/v1/models
```

### Load Testing

**Using hey**:

```bash
hey -n 100 -c 10 \
  -H "Content-Type: application/json" \
  -d '{"model":"gemini-1.5-pro-latest","messages":[{"role":"user","content":"test"}],"max_tokens":10}' \
  http://localhost:3000/v1/messages
```

**Using Apache Bench**:

```bash
ab -n 100 -c 10 -T "application/json" \
  -p test-payload.json \
  http://localhost:3000/v1/messages
```
