#!/bin/bash
# Collaborative Housaky Startup Script
# Starts both instances and bridges them

echo "☸️ Starting Collaborative Housaky Environment..."

# 1. Start Housaky-Rust gateway in background
echo "Starting Housaky-Rust gateway..."
./target/release/housaky gateway --port 8080 &
GATEWAY_PID=$!
echo "Gateway PID: $GATEWAY_PID"

# 2. Wait for gateway to be ready
sleep 2

# 3. Check gateway health
if curl -s http://127.0.0.1:8080/health > /dev/null 2>&1; then
    echo "✅ Gateway is healthy"
else
    echo "❌ Gateway failed to start"
    exit 1
fi

# 4. Write PID to shared file
echo "{\"gateway_pid\": $GATEWAY_PID, \"started\": \"$(date -Iseconds)\"}" > .housaky/shared/gateway_status.json

echo ""
echo "☸️ Collaborative environment ready!"
echo "   Gateway: http://127.0.0.1:8080"
echo "   Health:  http://127.0.0.1:8080/health"
echo ""
echo "To start TUI (separate terminal):"
echo "   ./target/release/housaky tui"
echo ""
echo "To use terminal-mcp:"
echo "   terminal-mcp housaky-tui -- ./target/release/housaky tui"
echo ""
echo "Press Ctrl+C to stop gateway..."

wait $GATEWAY_PID
