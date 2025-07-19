# Metrics API Documentation

## Overview

The AI Proxy system includes comprehensive metrics collection and monitoring capabilities. The metrics system tracks request counts, latency statistics, error rates, and provider-specific performance data.

## Endpoints

### GET /metrics

Returns detailed system metrics and statistics.

**Response Format:**

```json
{
  "metrics": {
    "uptime_seconds": 3600,
    "total_requests": 1250,
    "successful_requests": 1180,
    "failed_requests": 70,
    "success_rate_percent": 94.4,
    "error_rate_percent": 5.6,
    "avg_latency_ms": 245.7,
    "latency_stats": {
      "total_latency_ms": 307125,
      "min_latency_ms": 45,
      "max_latency_ms": 2340,
      "request_count": 1250
    },
    "provider_metrics": {
      "openai": {
        "total_requests": 650,
        "successful_requests": 620,
        "failed_requests": 30,
        "avg_latency_ms": 230.5,
        "last_request_time": "2024-01-15T10:30:45Z"
      },
      "gemini": {
        "total_requests": 400,
        "successful_requests": 380,
        "failed_requests": 20,
        "avg_latency_ms": 180.2,
        "last_request_time": "2024-01-15T10:29:12Z"
      },
      "anthropic": {
        "total_requests": 200,
        "successful_requests": 180,
        "failed_requests": 20,
        "avg_latency_ms": 320.8,
        "last_request_time": "2024-01-15T10:28:33Z"
      }
    },
    "model_metrics": {
      "gpt-4": {
        "total_requests": 300,
        "successful_requests": 285,
        "failed_requests": 15,
        "avg_latency_ms": 280.4
      },
      "gpt-3.5-turbo": {
        "total_requests": 350,
        "successful_requests": 335,
        "failed_requests": 15,
        "avg_latency_ms": 180.6
      },
      "gemini-pro": {
        "total_requests": 400,
        "successful_requests": 380,
        "failed_requests": 20,
        "avg_latency_ms": 180.2
      },
      "claude-3-sonnet": {
        "total_requests": 200,
        "successful_requests": 180,
        "failed_requests": 20,
        "avg_latency_ms": 320.8
      }
    },
    "timestamp": "2024-01-15T10:30:45Z"
  }
}
```

## Metrics Description

### System-Level Metrics

- **uptime_seconds**: Time since the system started (in seconds)
- **total_requests**: Total number of requests processed
- **successful_requests**: Number of successfully completed requests
- **failed_requests**: Number of failed requests
- **success_rate_percent**: Percentage of successful requests (0-100)
- **error_rate_percent**: Percentage of failed requests (0-100)
- **avg_latency_ms**: Average request processing time in milliseconds

### Latency Statistics

- **total_latency_ms**: Sum of all request processing times
- **min_latency_ms**: Minimum request processing time
- **max_latency_ms**: Maximum request processing time
- **request_count**: Number of requests used for latency calculation

### Provider Metrics

For each configured AI provider:

- **total_requests**: Total requests sent to this provider
- **successful_requests**: Successful requests for this provider
- **failed_requests**: Failed requests for this provider
- **avg_latency_ms**: Average response time for this provider
- **last_request_time**: Timestamp of the last request to this provider

### Model Metrics

For each AI model used:

- **total_requests**: Total requests for this model
- **successful_requests**: Successful requests for this model
- **failed_requests**: Failed requests for this model
- **avg_latency_ms**: Average response time for this model

## Usage Examples

### Basic Monitoring

```bash
# Get current system metrics
curl http://localhost:3000/metrics

# Monitor success rate
curl -s http://localhost:3000/metrics | jq '.metrics.success_rate_percent'

# Check provider performance
curl -s http://localhost:3000/metrics | jq '.metrics.provider_metrics'
```

### Prometheus Integration

The metrics endpoint can be integrated with monitoring systems like Prometheus by parsing the JSON response and converting it to Prometheus format.

### Alerting

You can set up alerts based on:

- Error rate exceeding threshold (e.g., > 5%)
- Average latency exceeding threshold (e.g., > 1000ms)
- Provider-specific failure rates
- System uptime monitoring

## Implementation Details

### Automatic Collection

Metrics are automatically collected for all requests processed through the `/v1/messages` endpoint. The system tracks:

1. Request start time
2. Request completion status (success/failure)
3. Provider used for the request
4. Model requested
5. Processing latency

### Thread Safety

The metrics collector is thread-safe and can handle concurrent requests without data corruption. It uses atomic operations for counters and read-write locks for complex data structures.

### Memory Usage

The metrics system maintains bounded memory usage by:

- Using atomic counters for basic statistics
- Storing aggregated data rather than individual request details
- Automatically calculating averages to avoid storing all latency values

### Performance Impact

The metrics collection has minimal performance impact:

- Atomic operations for counters
- Efficient timestamp recording
- Asynchronous metric updates
- No blocking operations in request path