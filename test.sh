#!/bin/bash

# Configuration
FOLDER="./tests"
BASE_URL="http://localhost:8040"   # Serve PDFs from 8040
ENDPOINT="http://localhost:8000/api/v1/hackrx/run"
AUTH_TOKEN="febc0daceda23ebce03d324301d34ad3768494f0b52a39ffb4adaf083d8f9c5c"
MINISERVE_PORT=8040

# Start miniserve in the background
echo "Starting miniserve on port $MINISERVE_PORT..."
miniserve "$FOLDER" --port $MINISERVE_PORT &
MINISERVE_PID=$!

# Wait for miniserve to be ready
sleep 1

# Batch test loop
for pdf in "$FOLDER"/*.pdf; do
    base=$(basename "$pdf" .pdf)
    txt="$FOLDER/$base.txt"
    echo "Found $base pdf and $txt text"

    if [ ! -f "$txt" ]; then
        echo "Warning: Missing $base.txt, skipping..."
        continue
    fi

    questions=$(jq -Rs '[split("\n")[] | select(length > 0)]' < "$txt")

    payload=$(jq -n \
        --arg pdf_path "$BASE_URL/$base.pdf" \
        --argjson questions "$questions" \
        '{documents: $pdf_path, questions: $questions}'
    )

response=$(curl -s -X POST "$ENDPOINT" \
        -H "Authorization: Bearer $AUTH_TOKEN" \
        -H "Content-Type: application/json" \
        -H "Accept: application/json" \
        -d "$payload")

    echo "$response"

    if echo "$response" | grep -q ""answers""; then
        echo "--- $base processed successfully ---"
    else
        echo "Warning: $base may not have been processed successfully. Aborting process"
        kill $MINISERVE_PID
        exit 1
    fi

done

# Kill miniserve
echo "Tests completed. Killing miniserve (PID $MINISERVE_PID)..."
kill $MINISERVE_PID
