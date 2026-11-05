#!/usr/bin/env bash
set -euo pipefail

# LogicCraft CAD - macOS App Bundle Builder
# Creates target/installer/LogicCraft.app

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

echo "========================================================"
echo "   LogicCraft CAD - macOS Application Bundle Builder    "
echo "========================================================"

cd "$PROJECT_ROOT"

echo "[1/3] Compiling release binary..."
cargo build --release

OUT_DIR="$PROJECT_ROOT/target/installer"
APP_DIR="$OUT_DIR/LogicCraft.app"
CONTENTS_DIR="$APP_DIR/Contents"
MACOS_DIR="$CONTENTS_DIR/MacOS"
RESOURCES_DIR="$CONTENTS_DIR/Resources"

echo "[2/3] Constructing LogicCraft.app directory structure..."
rm -rf "$APP_DIR"
mkdir -p "$MACOS_DIR" "$RESOURCES_DIR"

# Copy executable
cp "target/release/logiccraft" "$MACOS_DIR/logiccraft"
chmod +x "$MACOS_DIR/logiccraft"

# Copy Info.plist
cp "packaging/macos/Info.plist" "$CONTENTS_DIR/Info.plist"

# Copy Icon
if [ -f "packaging/assets/icon.png" ]; then
    cp "packaging/assets/icon.png" "$RESOURCES_DIR/icon.png"
fi

# If iconutil exists on macOS, convert PNG to ICNS
if command -v iconutil >/dev/null 2>&1 && [ -f "packaging/assets/icon.png" ]; then
    echo "[*] Generating native AppIcon.icns..."
    ICONSET_DIR="$OUT_DIR/AppIcon.iconset"
    mkdir -p "$ICONSET_DIR"
    sips -z 16 16     "packaging/assets/icon.png" --out "$ICONSET_DIR/icon_16x16.png" >/dev/null 2>&1 || true
    sips -z 32 32     "packaging/assets/icon.png" --out "$ICONSET_DIR/icon_16x16@2x.png" >/dev/null 2>&1 || true
    sips -z 32 32     "packaging/assets/icon.png" --out "$ICONSET_DIR/icon_32x32.png" >/dev/null 2>&1 || true
    sips -z 64 64     "packaging/assets/icon.png" --out "$ICONSET_DIR/icon_32x32@2x.png" >/dev/null 2>&1 || true
    sips -z 128 128   "packaging/assets/icon.png" --out "$ICONSET_DIR/icon_128x128.png" >/dev/null 2>&1 || true
    sips -z 256 256   "packaging/assets/icon.png" --out "$ICONSET_DIR/icon_128x128@2x.png" >/dev/null 2>&1 || true
    sips -z 256 256   "packaging/assets/icon.png" --out "$ICONSET_DIR/icon_256x256.png" >/dev/null 2>&1 || true
    sips -z 512 512   "packaging/assets/icon.png" --out "$ICONSET_DIR/icon_256x256@2x.png" >/dev/null 2>&1 || true
    sips -z 512 512   "packaging/assets/icon.png" --out "$ICONSET_DIR/icon_512x512.png" >/dev/null 2>&1 || true
    iconutil -c icns "$ICONSET_DIR" -o "$RESOURCES_DIR/icon.icns" || true
    rm -rf "$ICONSET_DIR"
fi

echo "[3/3] Application bundle built successfully at:"
echo "      $APP_DIR"
