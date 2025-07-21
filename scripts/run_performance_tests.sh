#!/bin/bash

# AI Proxy Performance Test Runner
# This script runs comprehensive performance and load tests

set -e

echo "🚀 Starting AI Proxy Performance Tests"
echo "======================================"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Function to print colored output
print_status() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

print_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Build the project in release mode for performance testing
print_status "Building project in release mode..."
cargo build --release

# Run unit performance tests
print_status "Running concurrent request handling tests..."
cargo test test_concurrent_request_handling --test performance_tests --release -- --nocapture

print_status "Running streaming memory usage tests..."
cargo test test_streaming_memory_usage --test performance_tests --release -- --nocapture

print_status "Running system stability tests..."
cargo test test_system_stability_under_load --test performance_tests --release -- --nocapture

# Run load tests
print_status "Running light load tests..."
cargo test test_light_load --test load_tests --release -- --nocapture

print_status "Running moderate load tests..."
cargo test test_moderate_load --test load_tests --release -- --nocapture

# Run streaming performance tests
print_status "Running concurrent streaming performance tests..."
cargo test test_concurrent_streaming_performance --test streaming_performance_tests --release -- --nocapture

print_status "Running variable chunk streaming tests..."
cargo test test_variable_chunk_streaming --test streaming_performance_tests --release -- --nocapture

# Run memory leak detection tests
print_status "Running memory leak detection tests..."
cargo test test_memory_leak_detector --test streaming_performance_tests --release -- --nocapture

# Run all performance-related tests
print_status "Running all performance tests..."
cargo test --release --test performance_tests --test load_tests --test streaming_performance_tests -- --nocapture

print_success "All performance tests completed!"

echo ""
echo "📊 Performance Test Summary"
echo "=========================="
echo "✅ Concurrent request handling"
echo "✅ Streaming memory usage"
echo "✅ System stability under load"
echo "✅ Light and moderate load testing"
echo "✅ Streaming performance testing"
echo "✅ Memory leak detection"
echo ""
echo "🎯 Performance testing completed successfully!"
echo "   Check the output above for detailed metrics and results."