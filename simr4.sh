#!/bin/bash

run_test() {
    local payload=$1
    local test_label=$2

    # Use curl's built-in timing and capture output to a temp file
    temp_file=$(mktemp)
    
    # Measure time and capture response
    start_time=$(date +%s.%N)
    response=$(curl -s -w "\n%{time_total}" -X POST http://localhost:8000/api/v1/hackrx/run \
    -H "Content-Type: application/json" \
    -H "Accept: application/json" \
    -H "Authorization: Bearer febc0daceda23ebce03d324301d34ad3768494f0b52a39ffb4adaf083d8f9c5c" \
    -d @"$payload" 2>/dev/null)
    
    curl_exit_code=$?
    end_time=$(date +%s.%N)

    if [ $curl_exit_code -ne 0 ]; then
        echo "Error: cURL command failed for $test_label"
        return
    fi

    # Extract time from the last line and response from all but last line
    time_taken=$(echo "$response" | tail -n1)
    response_json=$(echo "$response" | head -n -1)

    # Validate that time_taken is a number
    if ! [[ $time_taken =~ ^[0-9]+\.?[0-9]*$ ]]; then
        # Fallback to manual time calculation if curl timing fails
        time_taken=$(echo "$end_time - $start_time" | bc -l)
    fi

    echo "=========================================="
    echo "Response for $test_label:"
    echo "$response_json"
    echo ""

    if echo "$response_json" | grep -q '"answers"'; then
        echo "--- $test_label processed successfully in ${time_taken}s ---"
    else
        echo "Warning: $test_label not processed successfully."
    fi

    # Store the time taken in the array
    times+=($(printf "%.3f" $time_taken))
    echo ""
}

# Initialize time array
times=()

# Run tests
run_test "payloads/r41.json" "test 1 apis" 
run_test "payloads/r42.json" "test 2 secret key"
run_test "payloads/r43.json" "test 3 language"

# Calculate and display statistics
echo "=========================================="
echo "TIMING STATISTICS:"
echo "=========================================="

# Display individual times
echo "Individual request times:"
for i in "${!times[@]}"; do
    echo "Test $((i+1)): ${times[$i]}s"
done

# Calculate average using bc for precise floating point arithmetic
sum=0
for t in "${times[@]}"; do
    sum=$(echo "$sum + $t" | bc -l)
done

if [ ${#times[@]} -gt 0 ]; then
    average=$(echo "scale=3; $sum / ${#times[@]}" | bc -l)
    echo ""
    echo "Total requests: ${#times[@]}"
    echo "Total time: $(printf "%.3f" $sum)s"
    echo "Average time per request: ${average}s"
    
    # Additional statistics
    min_time=$(printf '%s\n' "${times[@]}" | sort -n | head -n1)
    max_time=$(printf '%s\n' "${times[@]}" | sort -n | tail -n1)
    echo "Fastest request: ${min_time}s"
    echo "Slowest request: ${max_time}s"
else
    echo "No successful requests to calculate average"
fi
