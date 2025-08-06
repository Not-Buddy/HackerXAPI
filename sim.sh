#!/bin/bash

run_test() {
    local payload=$1
    local test_label=$2

    response=$(curl -s -X POST http://localhost:8000/api/v1/hackrx/run \
    -H "Content-Type: application/json" \
    -H "Accept: application/json" \
    -H "Authorization: Bearer febc0daceda23ebce03d324301d34ad3768494f0b52a39ffb4adaf083d8f9c5c" \
    -d @"$payload")

    if [ $? -ne 0 ]; then
        echo "Error: cURL command failed for $test_label"
        return
    fi

    if echo "$response" | grep -q "\"answers\""; then
        echo "--- $test_label processed successfully ---"
    else
        echo "Warning: $test_label not processed successfully."
    fi
}

# Run tests
run_test "payloads/aarogya.json" "test asoaksoakso" 
run_test "payloads/aarogya2.json" "test 2"
run_test "payloads/splendor.json" "test 3"
run_test "payloads/medicare.json" "test 4"
run_test "payloads/medicare.json" "test 5"
run_test "payloads/cons.json" "test 6"
run_test "payloads/cons2.json" "test 7"
run_test "payloads/principia.json" "test 8"
run_test "payloads/uni.json" "test 9"
run_test "payloads/happy.json" "test 10"
run_test "payloads/xl.json" "test 11"
run_test "payloads/xl2.json" "test 12"
run_test "payloads/xl3.json" "test 13"
run_test "payloads/image.json" "test 14"
run_test "payloads/image2.json" "test 15"
run_test "payloads/bomb.json" "test 16"
run_test "payloads/bin.json" "test 17"
run_test "payloads/fact.json" "test 18"
