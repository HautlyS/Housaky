#!/bin/bash
# start_collaborative.sh - Initialize Housaky Collaborative Environment
#
# This starts both instances:
# 1. Housaky-Rust (gateway + TUI)
# 2. Sets up communication with Housaky-OpenClaw

set -e

echo "☸️ Starting Housaky Collaborative Environment..."
echo ""

WORKSPACE="/home/hautly/housaky"
SHARED_DIR="$WORKSPACE/.housaky/shared"

# Create shared directories
mkdir -p "$SHARED_DIR/inbox/housaky-openclaw"
mkdir -p "$SHARED_DIR/inbox/housaky-native"
mkdir -p "$SHARED_DIR/outbox/housaky-openclaw"
mkdir -p "$SHARED_DIR/outbox/housaky-native"

# Environment variables
export HOUSAKY_LUCID_CMD="$HOME/.lucid/bin/lucid"
export HOUSAKY_LUCID_BUDGET=200
export HOUSAKY_API_KEY="modalresearch_YpftD7Ixi0XpfGCnVIiIXechGp9dWgRi507W3yq4tBU"

cd "$WORKSPACE"

# Check if already running
if pgrep -f "housaky gateway" > /dev/null 2>&1; then
    echo "⚠️  Gateway already running"
else
    echo "🚀 Starting Gateway on port 8080..."
    ./target/release/housaky gateway --port 8080 > /tmp/housaky-gateway.log 2>&1 &
    sleep 2

    if curl -s http://127.0.0.1:8080/health > /dev/null 2>&1; then
        echo "✅ Gateway healthy"
    else
        echo "❌ Gateway failed to start"
        cat /tmp/housaky-gateway.log
        exit 1
    fi
fi

# Write status
cat > "$SHARED_DIR/status.json" << EOF
{
  "timestamp": "$(date -Iseconds)",
  "gateway": {
    "url": "http://127.0.0.1:8080",
    "status": "running"
  },
  "instances": {
    "openclaw": {
      "runtime": "typescript",
      "channel": "whatsapp",
      "status": "active"
    },
    "native": {
      "runtime": "rust",
      "status": "gateway_active"
    }
  },
  "memory": {
    "backend": "lucid",
    "db": "$HOME/.lucid/memory.db"
  }
}
EOF

echo ""
echo "☸️ Collaborative Environment Ready!"
echo ""
echo "Gateway:        http://127.0.0.1:8080"
echo "Health:         http://127.0.0.1:8080/health"
echo "Status:         $SHARED_DIR/status.json"
echo ""
echo "TUI (separate terminal):"
echo "  ./target/release/housaky tui"
echo ""
echo "Terminal-MCP:"
echo "  terminal-mcp --socket /tmp/housaky-mcp.sock -- ./target/release/housaky tui"
echo ""
echo "To send message to OpenClaw:"
echo "  echo '{\"from\":\"native\",\"msg\":\"hello\"}' > $SHARED_DIR/inbox/housaky-openclaw/msg.json"
