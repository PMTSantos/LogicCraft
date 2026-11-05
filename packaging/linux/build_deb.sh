#!/usr/bin/env bash
set -euo pipefail

# LogicCraft CAD - Debian / Ubuntu (.deb) Package Builder
# Generates target/installer/logiccraft_1.0.1_amd64.deb

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

echo "========================================================"
echo "    LogicCraft CAD - Linux DEB Package Builder          "
echo "========================================================"

cd "$PROJECT_ROOT"

OUT_DIR="$PROJECT_ROOT/target/installer"
STAGE_DIR="$OUT_DIR/deb_stage"
DEB_OUTPUT="$OUT_DIR/logiccraft_1.0.1_amd64.deb"

echo "[*] Preparing DEB stage directory structure..."
rm -rf "$STAGE_DIR" "$DEB_OUTPUT"
mkdir -p "$STAGE_DIR/DEBIAN"
mkdir -p "$STAGE_DIR/usr/bin"
mkdir -p "$STAGE_DIR/usr/share/applications"
mkdir -p "$STAGE_DIR/usr/share/icons/hicolor/256x256/apps"

# 1. Copy binaries
if [ -f "target/release/logiccraft" ]; then
    cp "target/release/logiccraft" "$STAGE_DIR/usr/bin/logiccraft"
    chmod 755 "$STAGE_DIR/usr/bin/logiccraft"
fi

if [ -f "target/release/logiccraft-updater" ]; then
    cp "target/release/logiccraft-updater" "$STAGE_DIR/usr/bin/logiccraft-updater"
    chmod 755 "$STAGE_DIR/usr/bin/logiccraft-updater"
fi

# 2. Copy Desktop file and Icon
cp "packaging/linux/logiccraft.desktop" "$STAGE_DIR/usr/share/applications/logiccraft.desktop"
cp "packaging/assets/icon.png" "$STAGE_DIR/usr/share/icons/hicolor/256x256/apps/logiccraft.png"

# 3. Create DEBIAN/control file
cat << 'EOF' > "$STAGE_DIR/DEBIAN/control"
Package: logiccraft
Version: 1.0.1
Section: electronics
Priority: optional
Architecture: amd64
Maintainer: Craft Anything <support@craftanything.com>
Depends: libc6, libgcc-s1, libxcb1, libxkbcommon0
Description: LogicCraft CAD - Modern digital logic circuit builder and simulator
 Modern cross-platform digital logic circuit simulator, technical logic analyzer,
 and Karnaugh Map synthesis studio built in Rust.
EOF

chmod 644 "$STAGE_DIR/DEBIAN/control"

# 4. Build .deb package
if command -v dpkg-deb >/dev/null 2>&1; then
    dpkg-deb --build "$STAGE_DIR" "$DEB_OUTPUT"
    rm -rf "$STAGE_DIR"
    echo "========================================================"
    echo " [SUCCESS] Created Debian/Ubuntu Package (.deb):"
    echo "           $DEB_OUTPUT"
    echo "========================================================"
else
    echo "[-] 'dpkg-deb' command not found. Skipping .deb compilation."
fi
