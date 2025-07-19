#!/bin/bash

# Build script for Trident macOS app bundle
# This script creates a release build and patches the Info.plist for proper menubar app behavior

echo "Building Trident.app..."

# Build the release bundle
cargo bundle --release

if [ $? -ne 0 ]; then
    echo "Failed to build app bundle"
    exit 1
fi

APP_PATH="target/release/bundle/osx/Trident.app"
PLIST_PATH="$APP_PATH/Contents/Info.plist"

echo "Adding LSUIElement to Info.plist..."

# Add LSUIElement if not already present
if ! grep -q "LSUIElement" "$PLIST_PATH"; then
    # Use sed to add LSUIElement before NSHighResolutionCapable
    sed -i '' '/<key>NSHighResolutionCapable<\/key>/i\
  <key>LSUIElement</key>\
  <true/>
' "$PLIST_PATH"
    echo "Added LSUIElement to Info.plist"
else
    echo "LSUIElement already present in Info.plist"
fi

echo "Build complete: $APP_PATH"
echo "App bundle size: $(du -h "$APP_PATH" | cut -f1)"

# Make the app executable if needed
chmod +x "$APP_PATH/Contents/MacOS/trident"

echo "Ready to distribute!"