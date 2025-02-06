#!/bin/bash
set -euo pipefail

# Paths (adjust these as needed)
PROJECT_ROOT="$(dirname "$(realpath "$0")")"
ASSETS_DIR="$PROJECT_ROOT/assets"
INSTALL_DIR="/usr/share/hyprpower"
BIN_INSTALL_DIR="/usr/local/bin"

# Files in assets
DEFAULT_CONFIG="$ASSETS_DIR/config.toml"
DEFAULT_STYLE="$ASSETS_DIR/style.css"

# Check if required commands exist
check_command() {
    if ! command -v "$1" &>/dev/null; then
        echo "Error: Required command '$1' is not installed." >&2
        exit 1
    fi
}

echo "Checking for required commands..."
check_command cargo
check_command sudo
check_command tee
check_command realpath

# Build the project in release mode
echo "Building HyprPower in release mode..."
cargo build --release

# Verify that assets exist
if [ ! -f "$DEFAULT_CONFIG" ]; then
    echo "Error: Default configuration file not found at $DEFAULT_CONFIG" >&2
    exit 1
fi

if [ ! -f "$DEFAULT_STYLE" ]; then
    echo "Error: Default stylesheet not found at $DEFAULT_STYLE" >&2
    exit 1
fi

# Create the installation directory
echo "Installing system-wide to $INSTALL_DIR..."
sudo mkdir -p "$INSTALL_DIR"
sudo chmod 755 "$INSTALL_DIR"

# Copy the assets
echo "Installing default configuration..."
sudo cp "$DEFAULT_CONFIG" "$INSTALL_DIR/config.toml"
echo "Installing default stylesheet..."
sudo cp "$DEFAULT_STYLE" "$INSTALL_DIR/style.css"

# Install the binary
# Determine the binary name (assumed to be "hyprpower" from Cargo.toml)
BINARY="$PROJECT_ROOT/target/release/hyprpower"
if [ ! -f "$BINARY" ]; then
    echo "Error: Binary not found at $BINARY" >&2
    exit 1
fi
echo "Installing binary to $BIN_INSTALL_DIR..."
sudo cp "$BINARY" "$BIN_INSTALL_DIR/"

# Optionally, update permissions on the binary if needed
sudo chmod 755 "$BIN_INSTALL_DIR/hyprpower"

echo "Installation complete."
echo "  - Binary installed to $BIN_INSTALL_DIR/hyprpower"
echo "  - Assets installed to $INSTALL_DIR"

