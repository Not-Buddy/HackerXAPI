#!/bin/bash

# Configuration
FOLDER="./tests"
BASE_URL="http://localhost:8040"   # Serve files from 8040
ENDPOINT="http://localhost:8000/api/v1/rag/query"
AUTH_TOKEN="febc0daceda23ebce03d324301d34ad3768494f0b52a39ffb4adaf083d8f9c5c"
MINISERVE_PORT=8040

# Supported extensions
EXTENSIONS=("pdf" "docx" "xlsx" "pptx")

# Start miniserve in the background
echo "Starting miniserve on port $MINISERVE_PORT..."
miniserve "$FOLDER" --port $MINISERVE_PORT &
MINISERVE_PID=$!

# Wait for miniserve to be ready
sleep 1

PASS=0
FAIL=0

for ext in "${EXTENSIONS[@]}"; do
    for doc in "$FOLDER"/*."$ext"; do
        [ -f "$doc" ] || continue

        base=$(basename "$doc" ".$ext")
        txt="$FOLDER/${base}_${ext}.txt"
        echo ""
        echo "=========================================="
        echo "Testing: $base.$ext"
        echo "=========================================="

        if [ ! -f "$txt" ]; then
            echo "Warning: Missing $txt, skipping..."
            continue
        fi

        questions=$(jq -Rs '[split("\n")[] | select(length > 0)]' < "$txt")

        payload=$(jq -n \
            --arg doc_path "$BASE_URL/$base.$ext" \
            --argjson questions "$questions" \
            '{documents: $doc_path, questions: $questions}'
        )

        echo "Payload: $payload"

        response=$(curl -s -X POST "$ENDPOINT" \
            -H "Authorization: Bearer $AUTH_TOKEN" \
            -H "Content-Type: application/json" \
            -H "Accept: application/json" \
            -d "$payload")

        echo "Response: $response"

        if echo "$response" | grep -q '"answers"'; then
            echo "--- $base.$ext processed successfully ---"
            PASS=$((PASS + 1))
        else
            echo "WARNING: $base.$ext may not have been processed successfully."
            FAIL=$((FAIL + 1))
        fi
    done
done

echo ""
echo "=========================================="
echo "Results: $PASS passed, $FAIL failed"
echo "=========================================="

# Kill miniserve
echo "Killing miniserve (PID $MINISERVE_PID)..."
kill $MINISERVE_PID

if [ $FAIL -gt 0 ]; then
    exit 1
fi
