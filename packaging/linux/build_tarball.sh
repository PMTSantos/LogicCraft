#!/usr/bin/env bash
set -euo pipefail

# LogicCraft CAD - Linux Portable Archive Builder
# Generates target/installer/LogicCraft-v${VERSION}-linux-x64.tar.gz

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

echo "========================================================"
echo "    LogicCraft CAD - Linux Portable Archive Builder     "
echo "========================================================"

cd "$PROJECT_ROOT"

OUT_DIR="$PROJECT_ROOT/target/installer"
VERSION=$(grep -m1 '^version' "$PROJECT_ROOT/Cargo.toml" | cut -d '"' -f 2)
STAGE_DIR="$OUT_DIR/LogicCraft-v${VERSION}-linux-x64"
TARBALL_OUTPUT="$OUT_DIR/LogicCraft-v${VERSION}-linux-x64.tar.gz"

echo "[*] Preparing Linux distribution stage..."
rm -rf "$STAGE_DIR" "$TARBALL_OUTPUT"
mkdir -p "$STAGE_DIR"

# Copy binaries
cp "target/release/logiccraft" "$STAGE_DIR/logiccraft"
chmod +x "$STAGE_DIR/logiccraft"

if [ -f "target/release/logiccraft-updater" ]; then
    cp "target/release/logiccraft-updater" "$STAGE_DIR/logiccraft-updater"
    chmod +x "$STAGE_DIR/logiccraft-updater"
fi

# Copy assets & desktop shortcut
cp "packaging/linux/logiccraft.desktop" "$STAGE_DIR/logiccraft.desktop"
cp "packaging/assets/icon.png" "$STAGE_DIR/icon.png"
if [ -f "README.md" ]; then
    cp "README.md" "$STAGE_DIR/"
fi

# Create install.sh inside the tarball for easy 1-click user installation
cat << 'EOF' > "$STAGE_DIR/install.sh"
#!/usr/bin/env bash
set -e
INSTALL_DIR="$HOME/.local/bin"
DESKTOP_DIR="$HOME/.local/share/applications"
ICON_DIR="$HOME/.local/share/icons/hicolor/256x256/apps"

mkdir -p "$INSTALL_DIR" "$DESKTOP_DIR" "$ICON_DIR"

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cp "$SCRIPT_DIR/logiccraft" "$INSTALL_DIR/logiccraft"
chmod +x "$INSTALL_DIR/logiccraft"

if [ -f "$SCRIPT_DIR/logiccraft-updater" ]; then
    cp "$SCRIPT_DIR/logiccraft-updater" "$INSTALL_DIR/logiccraft-updater"
    chmod +x "$INSTALL_DIR/logiccraft-updater"
fi

cp "$SCRIPT_DIR/icon.png" "$ICON_DIR/logiccraft.png"
sed "s|Exec=logiccraft|Exec=$INSTALL_DIR/logiccraft|g" "$SCRIPT_DIR/logiccraft.desktop" > "$DESKTOP_DIR/logiccraft.desktop"
chmod +x "$DESKTOP_DIR/logiccraft.desktop"

echo "LogicCraft CAD successfully installed to $INSTALL_DIR/logiccraft"
echo "Desktop launcher installed to $DESKTOP_DIR/logiccraft.desktop"
EOF
chmod +x "$STAGE_DIR/install.sh"

tar -czf "$TARBALL_OUTPUT" -C "$OUT_DIR" "LogicCraft-v${VERSION}-linux-x64"
rm -rf "$STAGE_DIR"

echo "========================================================"
echo " [SUCCESS] Created Linux Distribution Tarball:"
echo "           $TARBALL_OUTPUT"
echo "========================================================"
