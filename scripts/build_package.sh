#!/bin/bash
set -e

echo "Building Housaky Ubuntu packages..."

# Configuration
VERSION="0.1.0"
ARCH="amd64"
PACKAGE_NAME="housaky"
BINARY_NAME="housaky"
SOURCE_DIR="/home/ubuntu/Housaky"
OUTPUT_DIR="/home/ubuntu/Housaky/scripts/build"
BUILD_DIR="$OUTPUT_DIR/build"

# Clean previous build
rm -rf "$BUILD_DIR"
mkdir -p "$BUILD_DIR"

# Create package structure
PKG_DIR="$BUILD_DIR/$PACKAGE_NAME-$VERSION-$ARCH"
mkdir -p "$PKG_DIR/usr/bin"
mkdir -p "$PKG_DIR/usr/share/doc/$PACKAGE_NAME"
mkdir -p "$PKG_DIR/usr/share/bash-completion/completions"
mkdir -p "$PKG_DIR/DEBIAN"

# Copy binary
cp "$SOURCE_DIR/target/release/$BINARY_NAME" "$PKG_DIR/usr/bin/$BINARY_NAME"
chmod 755 "$PKG_DIR/usr/bin/$BINARY_NAME"

# Copy completion (if exists)
if [ -f "$SOURCE_DIR/completions/$BINARY_NAME.bash" ]; then
    cp "$SOURCE_DIR/completions/$BINARY_NAME.bash" "$PKG_DIR/usr/share/bash-completion/completions/$BINARY_NAME"
fi

# Create control file
cat > "$PKG_DIR/DEBIAN/control" << EOF
Package: $PACKAGE_NAME
Version: $VERSION
Architecture: $ARCH
Maintainer: Housaky Team <team@housaky.ai>
Description: Zero overhead. Zero compromise. 100% Rust. The fastest, smallest AI assistant.
 Housaky is an autonomous AI assistant designed for persistent, goal-oriented operation.
 It maintains context across sessions, pursues long-term objectives, and continuously
 improves its capabilities through a sophisticated AGI core.
Homepage: https://housaky.ai
EOF

# Create postinst script
cat > "$PKG_DIR/DEBIAN/postinst" << 'EOF'
#!/bin/bash
set -e

# Create housaky config directory
mkdir -p ~/.housaky

# Create completion if not exists
if [ -f /usr/share/bash-completion/completions/housaky ]; then
    echo "Housaky installation complete!"
else
    echo "Housaky installed (bash completion not available)"
fi

# Print welcome message
echo ""
echo "============================================"
echo "  Welcome to Housaky!"
echo "============================================"
echo ""
echo "Get started:"
echo "  housaky --version    # Check version"
echo "  housaky status      # Show system status"
echo "  housaky onboard     # Initial setup"
echo "  housaky chat        # Start chatting"
echo ""
echo "For more information: https://housaky.ai/docs"
echo ""
exit 0
EOF

chmod 755 "$PKG_DIR/DEBIAN/postinst"

# Create prerm script
cat > "$PKG_DIR/DEBIAN/prerm" << 'EOF'
#!/bin/bash
set -e
echo "Removing Housaky..."
exit 0
EOF

chmod 755 "$PKG_DIR/DEBIAN/prerm"

# Create copyright file
cat > "$PKG_DIR/usr/share/doc/$PACKAGE_NAME/copyright" << 'EOF'
Format: https://www.debian.org/doc/packaging-manuals/copyright-format/1.0/
Source: https://github.com/HautlyS/Housaky
Copyright: 2024 Housaky Team
License: MIT
EOF

# Build .deb package
cd "$OUTPUT_DIR"
fakeroot dpkg-deb --build "$PKG_DIR" "$PACKAGE_NAME-$VERSION-$ARCH.deb"

echo ""
echo "============================================"
echo "  Package built successfully!"
echo "============================================"
echo ""
echo "Output: $OUTPUT_DIR/$PACKAGE_NAME-$VERSION-$ARCH.deb"
echo ""

# Verify package
dpkg-deb -I "$OUTPUT_DIR/$PACKAGE_NAME-$VERSION-$ARCH.deb"

echo ""
echo "To install:"
echo "  sudo dpkg -i $OUTPUT_DIR/$PACKAGE_NAME-$VERSION-$ARCH.deb"
echo "  sudo apt-get install -f  # Install dependencies"
echo ""
