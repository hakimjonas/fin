#!/usr/bin/env bash
set -euo pipefail

##
# 1. Build in release mode
##
echo "Building the launcher in release mode..."
cargo build --release

##
# 2. Define install locations (merged /usr style)
##
INSTALL_BIN_DIR="/usr/bin"
INSTALL_SHARE_DIR="/usr/share/hyprpower"

echo "Using install dirs:"
echo "  Binaries: $INSTALL_BIN_DIR"
echo "  Share:    $INSTALL_SHARE_DIR"

# Create directories if missing
sudo mkdir -p "$INSTALL_BIN_DIR"
sudo mkdir -p "$INSTALL_SHARE_DIR"

##
# 3. Copy the binary
#    (If your Cargo package/binary name isn't 'hyprpower',
#     update LAUNCHER_BIN accordingly.)
##
LAUNCHER_BIN="target/release/hyprpower"
echo "Installing $LAUNCHER_BIN -> $INSTALL_BIN_DIR"
sudo cp "$LAUNCHER_BIN" "$INSTALL_BIN_DIR/"

# Optionally, make sure it's executable:
sudo chmod 755 "$INSTALL_BIN_DIR/hyprpower"

##
# 4. Copy default config and stylesheet
#    (Assumes you have them in an 'assets' folder.)
##
echo "Installing default config.toml and style.css -> $INSTALL_SHARE_DIR"
sudo cp assets/config.toml "$INSTALL_SHARE_DIR/config.toml"
sudo cp assets/style.css   "$INSTALL_SHARE_DIR/style.css"

echo "Installation complete!"
echo ""
echo "You can now run 'hyprpower' from the terminal."
echo "Default config is at: $INSTALL_SHARE_DIR/config.toml"
echo "Default stylesheet is at: $INSTALL_SHARE_DIR/style.css"
echo "User overrides, if any, go in ~/.config/hyprpower/ (for config) or whichever path is set in config."
