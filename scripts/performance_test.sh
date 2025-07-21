#!/bin/bash

# AI Proxy Performance Test Script
# This script runs various performance tests against the AI Proxy service

set -e

# Configuration
BASE_URL="http://localhost:3000"
CONTENT_TYPE="Content-Type: application/json"
CONCURRENT_REQUESTS=10
TEST_DURATION=30
RESULTS_DIR="./performance_results"

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

# Create results directory
mkdir -p "$RESULTS_DIR"

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

# Test single request latency
test_latency() {
    print_header "Testing Request Latency"
    
    local test_message='{"model":"gpt-3.5-turbo","messages":[{"role":"user","content":"Hello"}],"max_tokens":10}'
    local total_time=0
    local successful_requests=0
    local failed_requests=0
    
    print_info "Running 100 sequential requests..."
    
    for i in {1..100}; do
        local start_time=$(date +%s%N)
        
        if curl -s -X POST "$BASE_URL/v1/messages" \
            -H "$CONTENT_TYPE" \
            -d "$test_message" > /dev/null 2>&1; then
            local end_time=$(date +%s%N)
            local request_time=$((($end_time - $start_time) / 1000000))  # Convert to milliseconds
            total_time=$(($total_time + $request_time))
            successful_requests=$(($successful_requests + 1))
        else
            failed_requests=$(($failed_requests + 1))
        fi
        
        if [ $((i % 10)) -eq 0 ]; then
            echo -n "."
        fi
    done
    
    echo ""
    
    if [ $successful_requests -gt 0 ]; then
        local avg_latency=$(($total_time / $successful_requests))
        print_success "Average latency: ${avg_latency}ms"
        print_success "Successful requests: $successful_requests"
        
        if [ $failed_requests -gt 0 ]; then
            print_error "Failed requests: $failed_requests"
        fi
        
        # Save results
        echo "Latency Test Results" > "$RESULTS_DIR/latency_test.txt"
        echo "===================" >> "$RESULTS_DIR/latency_test.txt"
        echo "Average latency: ${avg_latency}ms" >> "$RESULTS_DIR/latency_test.txt"
        echo "Successful requests: $successful_requests" >> "$RESULTS_DIR/latency_test.txt"
        echo "Failed requests: $failed_requests" >> "$RESULTS_DIR/latency_test.txt"
        echo "Test date: $(date)" >> "$RESULTS_DIR/latency_test.txt"
    else
        print_error "All requests failed"
        exit 1
    fi
}

# Test concurrent requests
test_concurrency() {
    print_header "Testing Concurrent Requests"
    
    local test_message='{"model":"gpt-3.5-turbo","messages":[{"role":"user","content":"Hello"}],"max_tokens":10}'
    local pids=()
    local start_time=$(date +%s)
    
    print_info "Running $CONCURRENT_REQUESTS concurrent requests..."
    
    # Start concurrent requests
    for i in $(seq 1 $CONCURRENT_REQUESTS); do
        {
            local request_start=$(date +%s%N)
            if curl -s -X POST "$BASE_URL/v1/messages" \
                -H "$CONTENT_TYPE" \
                -d "$test_message" > "$RESULTS_DIR/concurrent_${i}.json" 2>&1; then
                local request_end=$(date +%s%N)
                local request_time=$((($request_end - $request_start) / 1000000))
                echo "$request_time" > "$RESULTS_DIR/concurrent_time_${i}.txt"
            else
                echo "FAILED" > "$RESULTS_DIR/concurrent_time_${i}.txt"
            fi
        } &
        pids+=($!)
    done
    
    # Wait for all requests to complete
    for pid in "${pids[@]}"; do
        wait $pid
    done
    
    local end_time=$(date +%s)
    local total_time=$(($end_time - $start_time))
    
    # Analyze results
    local successful=0
    local failed=0
    local total_response_time=0
    
    for i in $(seq 1 $CONCURRENT_REQUESTS); do
        if [ -f "$RESULTS_DIR/concurrent_time_${i}.txt" ]; then
            local time_content=$(cat "$RESULTS_DIR/concurrent_time_${i}.txt")
            if [ "$time_content" != "FAILED" ]; then
                successful=$(($successful + 1))
                total_response_time=$(($total_response_time + $time_content))
            else
                failed=$(($failed + 1))
            fi
        fi
    done
    
    print_success "Concurrent test completed in ${total_time}s"
    print_success "Successful requests: $successful"
    
    if [ $failed -gt 0 ]; then
        print_error "Failed requests: $failed"
    fi
    
    if [ $successful -gt 0 ]; then
        local avg_response_time=$(($total_response_time / $successful))
        print_success "Average response time: ${avg_response_time}ms"
        
        # Calculate requests per second
        local rps=$(echo "scale=2; $successful / $total_time" | bc -l)
        print_success "Requests per second: $rps"
    fi
    
    # Save results
    echo "Concurrency Test Results" > "$RESULTS_DIR/concurrency_test.txt"
    echo "========================" >> "$RESULTS_DIR/concurrency_test.txt"
    echo "Concurrent requests: $CONCURRENT_REQUESTS" >> "$RESULTS_DIR/concurrency_test.txt"
    echo "Total time: ${total_time}s" >> "$RESULTS_DIR/concurrency_test.txt"
    echo "Successful requests: $successful" >> "$RESULTS_DIR/concurrency_test.txt"
    echo "Failed requests: $failed" >> "$RESULTS_DIR/concurrency_test.txt"
    if [ $successful -gt 0 ]; then
        echo "Average response time: ${avg_response_time}ms" >> "$RESULTS_DIR/concurrency_test.txt"
        echo "Requests per second: $rps" >> "$RESULTS_DIR/concurrency_test.txt"
    fi
    echo "Test date: $(date)" >> "$RESULTS_DIR/concurrency_test.txt"
}

# Test streaming performance
test_streaming() {
    print_header "Testing Streaming Performance"
    
    local stream_message='{"model":"gpt-3.5-turbo","messages":[{"role":"user","content":"Count from 1 to 20"}],"max_tokens":100,"stream":true}'
    
    print_info "Testing streaming response performance..."
    
    local start_time=$(date +%s%N)
    local response_file="$RESULTS_DIR/streaming_response.txt"
    
    curl -s -X POST "$BASE_URL/v1/messages" \
        -H "$CONTENT_TYPE" \
        -d "$stream_message" > "$response_file"
    
    local end_time=$(date +%s%N)
    local total_time=$((($end_time - $start_time) / 1000000))
    
    # Count events in streaming response
    local event_count=$(grep -c "^data: " "$response_file" || echo "0")
    
    print_success "Streaming test completed in ${total_time}ms"
    print_success "Events received: $event_count"
    
    if [ $event_count -gt 0 ]; then
        local avg_time_per_event=$(($total_time / $event_count))
        print_success "Average time per event: ${avg_time_per_event}ms"
    fi
    
    # Save results
    echo "Streaming Test Results" > "$RESULTS_DIR/streaming_test.txt"
    echo "=====================" >> "$RESULTS_DIR/streaming_test.txt"
    echo "Total time: ${total_time}ms" >> "$RESULTS_DIR/streaming_test.txt"
    echo "Events received: $event_count" >> "$RESULTS_DIR/streaming_test.txt"
    if [ $event_count -gt 0 ]; then
        echo "Average time per event: ${avg_time_per_event}ms" >> "$RESULTS_DIR/streaming_test.txt"
    fi
    echo "Test date: $(date)" >> "$RESULTS_DIR/streaming_test.txt"
}

# Test load over time
test_load() {
    print_header "Testing Load Over Time"
    
    local test_message='{"model":"gpt-3.5-turbo","messages":[{"role":"user","content":"Hello"}],"max_tokens":10}'
    local end_time=$(($(date +%s) + $TEST_DURATION))
    local request_count=0
    local successful_count=0
    local failed_count=0
    
    print_info "Running load test for ${TEST_DURATION} seconds..."
    
    while [ $(date +%s) -lt $end_time ]; do
        if curl -s -X POST "$BASE_URL/v1/messages" \
            -H "$CONTENT_TYPE" \
            -d "$test_message" > /dev/null 2>&1; then
            successful_count=$(($successful_count + 1))
        else
            failed_count=$(($failed_count + 1))
        fi
        
        request_count=$(($request_count + 1))
        
        if [ $((request_count % 10)) -eq 0 ]; then
            echo -n "."
        fi
        
        # Small delay to prevent overwhelming
        sleep 0.1
    done
    
    echo ""
    
    local rps=$(echo "scale=2; $successful_count / $TEST_DURATION" | bc -l)
    
    print_success "Load test completed"
    print_success "Total requests: $request_count"
    print_success "Successful requests: $successful_count"
    print_success "Requests per second: $rps"
    
    if [ $failed_count -gt 0 ]; then
        print_error "Failed requests: $failed_count"
        local error_rate=$(echo "scale=2; $failed_count * 100 / $request_count" | bc -l)
        print_error "Error rate: ${error_rate}%"
    fi
    
    # Save results
    echo "Load Test Results" > "$RESULTS_DIR/load_test.txt"
    echo "=================" >> "$RESULTS_DIR/load_test.txt"
    echo "Test duration: ${TEST_DURATION}s" >> "$RESULTS_DIR/load_test.txt"
    echo "Total requests: $request_count" >> "$RESULTS_DIR/load_test.txt"
    echo "Successful requests: $successful_count" >> "$RESULTS_DIR/load_test.txt"
    echo "Failed requests: $failed_count" >> "$RESULTS_DIR/load_test.txt"
    echo "Requests per second: $rps" >> "$RESULTS_DIR/load_test.txt"
    if [ $failed_count -gt 0 ]; then
        echo "Error rate: ${error_rate}%" >> "$RESULTS_DIR/load_test.txt"
    fi
    echo "Test date: $(date)" >> "$RESULTS_DIR/load_test.txt"
}

# Test memory usage
test_memory() {
    print_header "Testing Memory Usage"
    
    print_info "Monitoring memory usage during requests..."
    
    # Get initial memory usage
    local initial_memory=$(ps -o pid,vsz,rss,comm -p $(pgrep ai-proxy) | tail -1 | awk '{print $3}')
    
    if [ -z "$initial_memory" ]; then
        print_error "Could not find ai-proxy process"
        return
    fi
    
    print_info "Initial memory usage: ${initial_memory}KB"
    
    # Run some requests while monitoring memory
    local test_message='{"model":"gpt-3.5-turbo","messages":[{"role":"user","content":"Generate a long response about artificial intelligence and its applications in modern technology."}],"max_tokens":500}'
    
    for i in {1..20}; do
        curl -s -X POST "$BASE_URL/v1/messages" \
            -H "$CONTENT_TYPE" \
            -d "$test_message" > /dev/null 2>&1 &
        
        # Monitor memory every few requests
        if [ $((i % 5)) -eq 0 ]; then
            local current_memory=$(ps -o pid,vsz,rss,comm -p $(pgrep ai-proxy) | tail -1 | awk '{print $3}')
            echo "Memory after $i requests: ${current_memory}KB"
        fi
    done
    
    # Wait for all requests to complete
    wait
    
    # Get final memory usage
    sleep 2
    local final_memory=$(ps -o pid,vsz,rss,comm -p $(pgrep ai-proxy) | tail -1 | awk '{print $3}')
    local memory_increase=$(($final_memory - $initial_memory))
    
    print_success "Final memory usage: ${final_memory}KB"
    print_success "Memory increase: ${memory_increase}KB"
    
    # Save results
    echo "Memory Test Results" > "$RESULTS_DIR/memory_test.txt"
    echo "==================" >> "$RESULTS_DIR/memory_test.txt"
    echo "Initial memory: ${initial_memory}KB" >> "$RESULTS_DIR/memory_test.txt"
    echo "Final memory: ${final_memory}KB" >> "$RESULTS_DIR/memory_test.txt"
    echo "Memory increase: ${memory_increase}KB" >> "$RESULTS_DIR/memory_test.txt"
    echo "Test date: $(date)" >> "$RESULTS_DIR/memory_test.txt"
}

# Generate performance report
generate_report() {
    print_header "Generating Performance Report"
    
    local report_file="$RESULTS_DIR/performance_report.md"
    
    cat > "$report_file" << EOF
# AI Proxy Performance Test Report

Generated on: $(date)

## Test Environment

- Server URL: $BASE_URL
- Concurrent Requests: $CONCURRENT_REQUESTS
- Test Duration: ${TEST_DURATION}s

## Test Results

EOF
    
    # Add each test result if file exists
    for test_file in latency_test.txt concurrency_test.txt streaming_test.txt load_test.txt memory_test.txt; do
        if [ -f "$RESULTS_DIR/$test_file" ]; then
            echo "### $(echo $test_file | sed 's/_/ /g' | sed 's/.txt//g' | sed 's/\b\w/\U&/g')" >> "$report_file"
            echo "" >> "$report_file"
            echo '```' >> "$report_file"
            cat "$RESULTS_DIR/$test_file" >> "$report_file"
            echo '```' >> "$report_file"
            echo "" >> "$report_file"
        fi
    done
    
    cat >> "$report_file" << EOF
## Recommendations

Based on the test results:

1. **Latency**: Average response time should be under 1000ms for good user experience
2. **Concurrency**: The system should handle at least 10 concurrent requests without failures
3. **Throughput**: Target at least 10 requests per second for production use
4. **Memory**: Memory usage should remain stable over time
5. **Streaming**: Streaming responses should have consistent event delivery

## Next Steps

- Monitor these metrics in production
- Set up alerting for performance degradation
- Consider scaling if limits are reached
- Optimize based on bottlenecks identified

EOF
    
    print_success "Performance report generated: $report_file"
}

# Cleanup function
cleanup() {
    print_info "Cleaning up temporary files..."
    rm -f "$RESULTS_DIR"/concurrent_*.json
    rm -f "$RESULTS_DIR"/concurrent_time_*.txt
}

# Main execution
main() {
    print_header "AI Proxy Performance Test Suite"
    print_info "This script will run comprehensive performance tests"
    print_info "Results will be saved to: $RESULTS_DIR"
    
    # Check dependencies
    if ! command -v bc &> /dev/null; then
        print_error "bc calculator is required but not installed"
        print_info "Install with: apt-get install bc (Ubuntu/Debian) or brew install bc (macOS)"
        exit 1
    fi
    
    # Run tests
    check_server
    
    print_info "Starting performance tests..."
    
    test_latency
    test_concurrency
    test_streaming
    test_load
    test_memory
    
    generate_report
    cleanup
    
    print_header "Performance Testing Completed"
    print_success "All tests completed successfully"
    print_info "Check $RESULTS_DIR for detailed results"
    print_info "Performance report: $RESULTS_DIR/performance_report.md"
}

# Handle script interruption
trap cleanup EXIT

# Execute main function
main "$@"