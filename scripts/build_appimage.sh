#!/bin/bash
set -e

echo "Building Housaky AppImage..."

# Configuration
VERSION="0.1.0"
ARCH="x86_64"
APP_NAME="Housaky"
BINARY_NAME="housaky"
SOURCE_DIR="/home/ubuntu/Housaky"
OUTPUT_DIR="/home/ubuntu/Housaky/scripts/build"
BUILD_DIR="$OUTPUT_DIR/appimage"

# Check for appimagetool
if ! command -v appimagetool &> /dev/null; then
    echo "AppImage tools not found. Installing..."
    wget -q "https://github.com/AppImage/AppImageKit/releases/download/continuous/appimagetool-x86_64.AppImage" -O /tmp/appimagetool
    chmod +x /tmp/appimagetool
    APPIMAGETOOL="/tmp/appimagetool"
else
    APPIMAGETOOL="appimagetool"
fi

# Clean previous build
rm -rf "$BUILD_DIR"
mkdir -p "$BUILD_DIR"

# Create AppDir structure
APP_DIR="$BUILD_DIR/AppDir"
mkdir -p "$APP_DIR/usr/bin"
mkdir -p "$APP_DIR/usr/share/applications"
mkdir -p "$APP_DIR/usr/share/icons/hicolor/256x256/apps"

# Copy binary
cp "$SOURCE_DIR/target/release/$BINARY_NAME" "$APP_DIR/usr/bin/$BINARY_NAME"
chmod 755 "$APP_DIR/usr/bin/$BINARY_NAME"

# Create desktop file
cat > "$APP_DIR/housaky.desktop" << EOF
[Desktop Entry]
Name=Housaky
Comment=Zero overhead. Zero compromise. 100% Rust AI Assistant
Exec=housaky
Icon=housaky
Terminal=true
Type=Application
Categories=Utility;AI;
Keywords=ai;assistant;cli;chat;agent;
EOF

# Copy icon (use default if not exists)
if [ -f "$SOURCE_DIR/dashboard/src-tauri/icons/256x256.png" ]; then
    cp "$SOURCE_DIR/dashboard/src-tauri/icons/256x256.png" "$APP_DIR/usr/share/icons/hicolor/256x256/apps/housaky.png"
else
    # Create a simple placeholder icon
    echo "Creating placeholder icon..."
fi

# Create AppRun script
cat > "$APP_DIR/AppRun" << 'EOF'
#!/bin/bash
set -e

# Get the directory where this script is located
APP_DIR="$(cd "$(dirname "$0")" && pwd)"

# Export library path if needed
export LD_LIBRARY_PATH="$APP_DIR/usr/lib:$LD_LIBRARY_PATH"

# Run the application
exec "$APP_DIR/usr/bin/housaky" "$@"
EOF

chmod +x "$APP_DIR/AppRun"

# Build AppImage
cd "$BUILD_DIR"
chmod +x "$APPIMAGETOOL"
ARCH=$ARCH "$APPIMAGETOOL" AppDir "$APP_NAME-$VERSION-$ARCH.AppImage"

# Move to output
mv "$APP_NAME-$VERSION-$ARCH.AppImage" "$OUTPUT_DIR/"

echo ""
echo "============================================"
echo "  AppImage built successfully!"
echo "============================================"
echo ""
echo "Output: $OUTPUT_DIR/$APP_NAME-$VERSION-$ARCH.AppImage"
echo ""
echo "To run:"
echo "  chmod +x $OUTPUT_DIR/$APP_NAME-$VERSION-$ARCH.AppImage"
echo "  $OUTPUT_DIR/$APP_NAME-$VERSION-$ARCH.AppImage"
echo ""
