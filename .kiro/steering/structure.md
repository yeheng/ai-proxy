# Project Structure & Organization

## Source Code Organization

```
src/
├── main.rs           # Application entry point and server setup
├── lib.rs            # Library exports and module declarations
├── config.rs         # Configuration management with Figment
├── errors.rs         # Error handling with anyhow + thiserror
├── server.rs         # Web server, routing, and request handlers
└── providers/        # AI provider implementations
    ├── mod.rs        # Provider trait and common types
    ├── anthropic.rs  # Anthropic API format definitions (standard)
    ├── gemini.rs     # Google Gemini provider implementation
    └── openai.rs     # OpenAI provider implementation
```

## Documentation Structure

```
docs/
├── README.md                    # Documentation index
├── api/
│   ├── openapi.yml             # OpenAPI specification
│   └── rest-api.md             # REST API documentation
├── architecture/
│   ├── overview.md             # System architecture overview
│   ├── design-patterns.md      # Design patterns used
│   ├── technical-spec.md       # Technical specifications
│   └── deployment.md           # Deployment guide
└── guides/
    └── module-design.md        # Module design and implementation guide
```

## Configuration Structure

- **Root Config**: `config.toml` - Main configuration file
- **Example Config**: `config.example.toml` - Template with examples
- **Environment Variables**: `AI_PROXY_*` prefixed overrides

## Module Responsibilities

### Core Modules

- **`main.rs`**: Application bootstrap, logging setup, server initialization
- **`lib.rs`**: Public API exports and module organization
- **`config.rs`**: Configuration loading with TOML and environment support
- **`errors.rs`**: Centralized error handling with HTTP response mapping
- **`server.rs`**: HTTP server, routing, middleware, and request handling

### Provider System

- **`providers/mod.rs`**: `AIProvider` trait definition and common types
- **`providers/anthropic.rs`**: Standard API format (request/response structs)
- **Provider implementations**: Individual modules for each AI service

## Naming Conventions

### Files & Modules

- Use snake_case for file names and module names
- Group related functionality in modules
- Keep module files focused and single-purpose

### Structs & Types

- Use PascalCase for struct names (`AnthropicRequest`, `AppState`)
- Use descriptive names that indicate purpose
- Suffix error types with `Error` (`AppError`)
- Suffix configuration types with `Config` (`ServerConfig`)

### Functions & Variables

- Use snake_case for function and variable names
- Use descriptive names that indicate action or purpose
- Prefix boolean functions with `is_` or `has_` when appropriate

## Code Organization Patterns

### Error Handling Strategy

- **Internal Operations**: Use `anyhow::Result<T>` for rich error context
- **API Responses**: Use `AppError` for well-typed HTTP responses
- **Handler Functions**: Return `AppResult<T>` for automatic error conversion

### Async Patterns

- Use `#[async_trait]` for trait objects with async methods
- Implement `Send + Sync` bounds for shared state
- Use `Arc<T>` for shared immutable data
- Use connection pooling for HTTP clients

### Configuration Pattern

- Centralize all configuration in `config.rs`
- Support both file-based and environment-based configuration
- Use strongly-typed structs for configuration validation
- Implement configuration hot-reloading for production systems

## Testing Organization

### Test Structure

```
tests/
├── integration_tests.rs    # Full API integration tests
├── provider_tests.rs       # Provider-specific tests
└── common/
    └── mod.rs             # Shared test utilities
```

### Test Categories

- **Unit Tests**: In-module tests for individual functions
- **Integration Tests**: Full request/response flow tests
- **Provider Tests**: Mock external API interactions
- **Load Tests**: Performance and concurrency testing

## Build Artifacts

### Target Directory

```
target/
├── debug/          # Debug builds
├── release/        # Optimized release builds
└── doc/           # Generated documentation
```

### Key Artifacts

- **Binary**: `target/release/ai-proxy` - Main executable
- **Documentation**: `target/doc/ai_proxy/` - Generated docs
- **Test Results**: Cargo test output and coverage reports
