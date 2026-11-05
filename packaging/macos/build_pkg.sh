#!/usr/bin/env bash
set -euo pipefail

# LogicCraft CAD - macOS PKG Installer Builder
# Generates target/installer/LogicCraft-2.5.0.pkg using Apple pkgbuild / productbuild

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

echo "========================================================"
echo "    LogicCraft CAD - macOS PKG Installer Builder        "
echo "========================================================"

cd "$PROJECT_ROOT"

# Ensure LogicCraft.app bundle exists
bash "$SCRIPT_DIR/build_app.sh"

OUT_DIR="$PROJECT_ROOT/target/installer"
APP_DIR="$OUT_DIR/LogicCraft.app"
PKG_OUTPUT="$OUT_DIR/LogicCraft-2.5.0.pkg"

echo "[*] Creating macOS Component Package (.pkg)..."

if command -v pkgbuild >/dev/null 2>&1; then
    pkgbuild \
        --component "$APP_DIR" \
        --install-location "/Applications" \
        --identifier "com.craftanything.logiccraft" \
        --version "2.5.0" \
        "$PKG_OUTPUT"

    echo "========================================================"
    echo " [SUCCESS] Created macOS PKG Installer:"
    echo "           $PKG_OUTPUT"
    echo "========================================================"
else
    echo "[!] 'pkgbuild' command not found (requires macOS)."
    echo "[*] Packaging .app into compressed tarball for macOS transfer..."
    tar -czf "$OUT_DIR/LogicCraft-v2.5.0-macos.tar.gz" -C "$OUT_DIR" "LogicCraft.app"
    echo " [SUCCESS] Created fallback macOS archive:"
    echo "           $OUT_DIR/LogicCraft-v2.5.0-macos.tar.gz"
fi
