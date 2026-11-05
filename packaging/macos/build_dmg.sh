#!/usr/bin/env bash
set -euo pipefail

# LogicCraft CAD - macOS DMG Installer Builder
# Generates target/installer/LogicCraft-1.0.1.dmg with /Applications symlink

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

echo "========================================================"
echo "    LogicCraft CAD - macOS DMG Installer Builder        "
echo "========================================================"

cd "$PROJECT_ROOT"

# Ensure LogicCraft.app bundle exists
bash "$SCRIPT_DIR/build_app.sh"

OUT_DIR="$PROJECT_ROOT/target/installer"
APP_DIR="$OUT_DIR/LogicCraft.app"
DMG_STAGE="$OUT_DIR/dmg_stage"
DMG_OUTPUT="$OUT_DIR/LogicCraft-1.0.1.dmg"

echo "[*] Preparing DMG staging directory..."
rm -rf "$DMG_STAGE" "$DMG_OUTPUT"
mkdir -p "$DMG_STAGE"

cp -R "$APP_DIR" "$DMG_STAGE/"
ln -s "/Applications" "$DMG_STAGE/Applications"

if command -v hdiutil >/dev/null 2>&1; then
    echo "[*] Creating compressed DMG disk image with hdiutil..."
    hdiutil create \
        -volname "LogicCraft CAD" \
        -srcfolder "$DMG_STAGE" \
        -ov \
        -format UDZO \
        "$DMG_OUTPUT"

    rm -rf "$DMG_STAGE"
    echo "========================================================"
    echo " [SUCCESS] Created macOS DMG Installer:"
    echo "           $DMG_OUTPUT"
    echo "========================================================"
else
    echo "[!] 'hdiutil' command not found (requires macOS)."
    echo "[*] Staging folder preserved at: $DMG_STAGE"
fi
