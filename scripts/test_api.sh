#!/bin/bash

# AI Proxy API Test Script
# This script demonstrates how to use the AI Proxy API with different providers

set -e

# Configuration
BASE_URL="http://localhost:3000"
CONTENT_TYPE="Content-Type: application/json"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Helper functions
print_header() {
    echo -e "\n${BLUE}=== $1 ===${NC}"
}

print_success() {
    echo -e "${GREEN}✓ $1${NC}"
}

print_error() {
    echo -e "${RED}✗ $1${NC}"
}

print_info() {
    echo -e "${YELLOW}ℹ $1${NC}"
}

# Check if server is running
check_server() {
    print_header "Checking Server Status"
    
    if curl -s -f "$BASE_URL/health" > /dev/null; then
        print_success "Server is running at $BASE_URL"
    else
        print_error "Server is not running at $BASE_URL"
        print_info "Please start the server with: cargo run"
        exit 1
    fi
}

# Test health endpoints
test_health() {
    print_header "Testing Health Endpoints"
    
    # Basic health check
    print_info "Testing basic health check..."
    response=$(curl -s "$BASE_URL/health")
    echo "Response: $response"
    print_success "Basic health check completed"
    
    # Provider health check
    print_info "Testing provider health check..."
    response=$(curl -s "$BASE_URL/health/providers")
    echo "Response: $response"
    print_success "Provider health check completed"
}

# Test model listing
test_models() {
    print_header "Testing Model Listing"
    
    print_info "Fetching available models..."
    response=$(curl -s "$BASE_URL/v1/models")
    echo "Response: $response"
    print_success "Model listing completed"
}

# Test chat completion with different providers
test_chat_completion() {
    print_header "Testing Chat Completion"
    
    # Test data
    local test_message='{"model":"gpt-3.5-turbo","messages":[{"role":"user","content":"Hello! Please respond with just \"Hello from AI Proxy!\""}],"max_tokens":50}'
    
    print_info "Testing OpenAI-compatible chat completion..."
    response=$(curl -s -X POST "$BASE_URL/v1/messages" \
        -H "$CONTENT_TYPE" \
        -d "$test_message")
    echo "Response: $response"
    print_success "OpenAI chat completion test completed"
    
    # Test with Anthropic model
    local anthropic_message='{"model":"claude-3-haiku-20240307","messages":[{"role":"user","content":"Hello! Please respond with just \"Hello from Claude!\""}],"max_tokens":50}'
    
    print_info "Testing Anthropic chat completion..."
    response=$(curl -s -X POST "$BASE_URL/v1/messages" \
        -H "$CONTENT_TYPE" \
        -d "$anthropic_message")
    echo "Response: $response"
    print_success "Anthropic chat completion test completed"
    
    # Test with Gemini model
    local gemini_message='{"model":"gemini-pro","messages":[{"role":"user","content":"Hello! Please respond with just \"Hello from Gemini!\""}],"max_tokens":50}'
    
    print_info "Testing Gemini chat completion..."
    response=$(curl -s -X POST "$BASE_URL/v1/messages" \
        -H "$CONTENT_TYPE" \
        -d "$gemini_message")
    echo "Response: $response"
    print_success "Gemini chat completion test completed"
}

# Test streaming responses
test_streaming() {
    print_header "Testing Streaming Responses"
    
    local stream_message='{"model":"gpt-3.5-turbo","messages":[{"role":"user","content":"Count from 1 to 5, one number per line."}],"max_tokens":100,"stream":true}'
    
    print_info "Testing streaming chat completion..."
    print_info "Streaming response (first 10 lines):"
    
    curl -s -X POST "$BASE_URL/v1/messages" \
        -H "$CONTENT_TYPE" \
        -d "$stream_message" | head -10
    
    print_success "Streaming test completed"
}

# Test error handling
test_error_handling() {
    print_header "Testing Error Handling"
    
    # Test invalid model
    print_info "Testing invalid model error..."
    response=$(curl -s -X POST "$BASE_URL/v1/messages" \
        -H "$CONTENT_TYPE" \
        -d '{"model":"invalid-model","messages":[{"role":"user","content":"test"}],"max_tokens":10}')
    echo "Response: $response"
    print_success "Invalid model error test completed"
    
    # Test malformed request
    print_info "Testing malformed request error..."
    response=$(curl -s -X POST "$BASE_URL/v1/messages" \
        -H "$CONTENT_TYPE" \
        -d '{"invalid":"json"}')
    echo "Response: $response"
    print_success "Malformed request error test completed"
    
    # Test missing required fields
    print_info "Testing missing required fields error..."
    response=$(curl -s -X POST "$BASE_URL/v1/messages" \
        -H "$CONTENT_TYPE" \
        -d '{"model":"gpt-3.5-turbo"}')
    echo "Response: $response"
    print_success "Missing fields error test completed"
}

# Performance test
test_performance() {
    print_header "Testing Performance"
    
    print_info "Running concurrent requests test (10 requests)..."
    
    local test_message='{"model":"gpt-3.5-turbo","messages":[{"role":"user","content":"Say hello"}],"max_tokens":10}'
    
    # Run 10 concurrent requests
    for i in {1..10}; do
        curl -s -X POST "$BASE_URL/v1/messages" \
            -H "$CONTENT_TYPE" \
            -d "$test_message" > /dev/null &
    done
    
    # Wait for all background jobs to complete
    wait
    
    print_success "Concurrent requests test completed"
}

# Main execution
main() {
    print_header "AI Proxy API Test Suite"
    print_info "This script tests the AI Proxy API functionality"
    print_info "Make sure the server is running before executing tests"
    
    # Run tests
    check_server
    test_health
    test_models
    
    # Only run chat tests if API keys are configured
    print_info "Note: Chat completion tests require valid API keys in config.toml"
    read -p "Do you want to run chat completion tests? (y/N): " -n 1 -r
    echo
    if [[ $REPLY =~ ^[Yy]$ ]]; then
        test_chat_completion
        test_streaming
        test_performance
    else
        print_info "Skipping chat completion tests"
    fi
    
    test_error_handling
    
    print_header "Test Suite Completed"
    print_success "All tests have been executed"
    print_info "Check the output above for any errors or issues"
}

# Execute main function
main "$@"