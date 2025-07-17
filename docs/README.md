# AI Proxy Documentation

This directory contains comprehensive documentation for the AI Proxy system, organized into logical sections for easy navigation and understanding.

## Documentation Structure

```
docs/
├── README.md                    # This file - documentation overview
├── architecture/                # System architecture and design
│   ├── overview.md             # High-level system overview
│   ├── design-patterns.md      # Architecture patterns and DDD
│   ├── technical-spec.md       # Technical specifications
│   └── deployment.md           # Deployment and operations
├── api/                        # API documentation
│   ├── rest-api.md             # REST API reference
│   └── openapi.yml             # OpenAPI specification
├── guides/                     # Development guides
│   └── module-design.md        # Module design and implementation
└── deployment/                 # Deployment configurations
    ├── kubernetes/             # Kubernetes manifests
    ├── docker/                 # Docker configurations
    └── terraform/              # Infrastructure as code
```

## Quick Start

1. **Read the Overview**: Start with [architecture/overview.md](architecture/overview.md) for a high-level understanding
2. **Check the API**: Review [api/rest-api.md](api/rest-api.md) for API usage
3. **Development**: Use [guides/module-design.md](guides/module-design.md) for development guidance
4. **Deployment**: Follow [architecture/deployment.md](architecture/deployment.md) for deployment instructions

## Documentation Sections

### 🏗️ Architecture

- **System Overview**: Core concepts and benefits
- **Design Patterns**: Adapter, Gateway, and Stream processing patterns
- **Technical Specs**: Technology stack and performance characteristics
- **Deployment Guide**: Local development and production deployment

### 🔌 API Reference

- **REST API**: Complete endpoint documentation with examples
- **Authentication**: API key usage and security
- **Models**: Supported AI providers and models
- **Error Handling**: Common error codes and responses

### 📖 Development Guides

- **Module Design**: Detailed module-by-module implementation guide
- **Adding Providers**: Step-by-step guide for adding new AI providers
- **Testing**: Unit, integration, and load testing strategies
- **Best Practices**: Code organization and performance optimization

### 🚀 Deployment

- **Local Development**: Quick setup and testing
- **Docker**: Containerization guide
- **Kubernetes**: Production deployment with K8s
- **Cloud Platforms**: AWS, GCP, Azure deployment options

## Contributing to Documentation

### Writing Style

- Use clear, concise language
- Include code examples for all concepts
- Provide practical use cases
- Keep documentation up-to-date with code changes

### File Organization

- Use descriptive filenames
- Group related content together
- Cross-reference related sections
- Include table of contents for longer documents

### Code Examples

- Ensure all examples are tested and working
- Use consistent formatting
- Provide both simple and complex examples
- Include error handling examples

## Getting Help

- **Issues**: Report documentation issues on GitHub
- **Questions**: Use GitHub Discussions for questions
- **Updates**: Follow the changelog for documentation updates

## Related Resources

- [Main README](../README.md) - Project overview and quick start
- [Contributing Guide](../CONTRIBUTING.md) - How to contribute
- [Changelog](../CHANGELOG.md) - Version history and updates

## License

This documentation is licensed under the same license as the project. See [LICENSE](../LICENSE) for details.
