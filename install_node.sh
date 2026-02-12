#!/bin/bash
# Install and run Housaky AGI as a system node

set -e

echo "🚀 Installing Housaky AGI Node on Pop!_OS"
echo "=========================================="
echo ""

# Check if running as root for systemd install
if [ "$EUID" -ne 0 ] && [ "$1" = "--system" ]; then
    echo "❌ Run with sudo for system-wide installation"
    echo "   sudo ./install_node.sh --system"
    exit 1
fi

# Build release binary
echo "🔨 Building release binary..."
cargo build --release
echo "✅ Build complete"

# Run tests
echo "🧪 Running tests..."
if cargo test --release 2>&1 | grep -q "test result: ok"; then
    echo "✅ All tests passed"
else
    echo "❌ Tests failed"
    exit 1
fi

# Get binary path
BINARY_PATH="$(pwd)/target/release/housaky"
WORK_DIR="$(pwd)"

if [ "$1" = "--system" ]; then
    # System-wide installation
    echo ""
    echo "📦 Installing system-wide service..."
    
    # Create systemd service
    cat > /tmp/housaky-node.service <<EOF
[Unit]
Description=Housaky AGI Node
After=network.target

[Service]
Type=simple
User=$SUDO_USER
WorkingDirectory=$WORK_DIR
ExecStart=$BINARY_PATH --federated --port 9000 --node-id $(hostname)-node
Restart=always
RestartSec=10
StandardOutput=append:$WORK_DIR/housaky-node.log
StandardError=append:$WORK_DIR/housaky-node.log

[Install]
WantedBy=multi-user.target
EOF

    # Install service
    mv /tmp/housaky-node.service /etc/systemd/system/
    systemctl daemon-reload
    systemctl enable housaky-node
    systemctl start housaky-node
    
    echo "✅ Service installed and started"
    echo ""
    echo "Commands:"
    echo "  Status:  sudo systemctl status housaky-node"
    echo "  Stop:    sudo systemctl stop housaky-node"
    echo "  Restart: sudo systemctl restart housaky-node"
    echo "  Logs:    sudo journalctl -u housaky-node -f"
    echo "  Remove:  sudo systemctl stop housaky-node && sudo systemctl disable housaky-node"
    
else
    # User-mode installation
    echo ""
    echo "🏃 Running in user mode..."
    echo ""
    echo "Starting node on port 9000..."
    echo "Press Ctrl+C to stop"
    echo ""
    
    $BINARY_PATH --federated --port 9000 --node-id $(hostname)-node
fi

echo ""
echo "✅ Installation complete!"
