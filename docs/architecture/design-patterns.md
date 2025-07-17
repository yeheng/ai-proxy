# Design Patterns

## Core Patterns

### 1. Adapter Pattern (Primary)

- **Purpose**: Standardize different AI provider APIs into a single interface
- **Implementation**: Each provider implements the `AIProvider` trait
- **Benefits**:
  - Loose coupling between client and provider APIs
  - Easy provider addition without client changes
  - Consistent client experience across all providers

### 2. Gateway Pattern

- **Purpose**: Centralized request routing and response handling
- **Implementation**: Single entry point (`/v1/messages`) with intelligent routing
- **Benefits**:
  - Simplified client integration
  - Centralized monitoring and logging
  - Request/response transformation in one place

### 3. Stream Processing Pattern

- **Purpose**: Real-time response streaming with format conversion
- **Implementation**: Async stream processing with SSE (Server-Sent Events)
- **Benefits**:
  - Low latency for end users
  - Memory efficient processing
  - Real-time user experience

## Data Flow Architecture

```
Client Request → Request Router → Provider Adapter → AI Provider API
                     ↓                    ↓                    ↓
               Validation        Format Conversion    Provider-specific
               Routing           Request Forwarding   Processing
                     ↓                    ↓                    ↓
Client Response ← Response Handler ← Provider Adapter ← AI Provider API
```

## Domain-Driven Design (DDD)

### Bounded Contexts

#### 1. Proxy Context

- **Entities**: Request, Response, Stream
- **Value Objects**: ModelId, ProviderId
- **Services**: RequestRouter, ResponseHandler

#### 2. Provider Context

- **Entities**: Provider, Adapter
- **Value Objects**: ApiKey, BaseUrl, Configuration
- **Services**: ProviderFactory, AdapterRegistry

#### 3. Configuration Context

- **Entities**: Config, ProviderConfig
- **Value Objects**: Settings, Overrides
- **Services**: ConfigLoader, ConfigValidator
