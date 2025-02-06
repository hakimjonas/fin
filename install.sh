#!/bin/bash
set -e

# Function to check if a command exists
check_command() {
    if ! command -v "$1" &>/dev/null; then
        echo "Error: Required command '$1' is not installed." >&2
        exit 1
    fi
}

# Function to create a directory with error handling
create_directory() {
    if ! sudo mkdir -p "$1"; then
        echo "Error: Failed to create directory '$1'." >&2
        exit 1
    fi
    if ! sudo chmod 755 "$1"; then
        echo "Error: Failed to set permissions for directory '$1'." >&2
        exit 1
    fi
}

# Function to write a file with error handling
write_file() {
    if ! sudo tee "$1" >/dev/null; then
        echo "Error: Failed to write to file '$1'." >&2
        exit 1
    fi
}

# Check for required commands
echo "Checking for required commands..."
check_command cargo
check_command sudo
check_command tee

# Build the project
echo "Building HyprPower in release mode..."
if ! cargo build --release; then
    echo "Error: Build failed." >&2
    exit 1
fi

# System-wide installation
CONFIG_DIR="/usr/share/hyprpower"
CONFIG_FILE="$CONFIG_DIR/config.toml"
STYLE_FILE="$CONFIG_DIR/style.css"

echo "Installing system-wide to $CONFIG_DIR"
create_directory "$CONFIG_DIR"

# Install configuration
echo "Installing default configuration..."
write_file "$CONFIG_FILE" <<'EOF'
# HyprPower Default Configuration
title = "HyprPower"
columns = 2
stylesheet = "style.css"
use_system_theme = false

[default_commands]
# GNOME
gnome = [
    { label = "󰌾", command = "xdg-screensaver lock" },
    { label = "󰍃", command = "gnome-session-quit --logout" },
    { label = "󰜉", command = "systemctl reboot" },
    { label = "󰐥", command = "systemctl poweroff" }
]

# Hyprland
hyprland = [
    { label = "󰌾", command = "hyprctl dispatch exec hyprlock" },
    { label = "󰍃", command = "loginctl terminate-user $USER" },
    { label = "󰜉", command = "systemctl reboot" },
    { label = "󰐥", command = "systemctl poweroff" }
]

# Budgie
budgie = [
    { label = "󰌾", command = "xdg-screensaver lock" },
    { label = "󰍃", command = "budgie-session-quit --logout" },
    { label = "󰜉", command = "systemctl reboot" },
    { label = "󰐥", command = "systemctl poweroff" }
]

# XFCE
xfce = [
    { label = "󰌾", command = "xflock4" },
    { label = "󰍃", command = "xfce4-session-logout --logout" },
    { label = "󰜉", command = "systemctl reboot" },
    { label = "󰐥", command = "systemctl poweroff" }
]

# i3 Window Manager
i3 = [
    { label = "󰌾", command = "i3lock" },
    { label = "󰍃", command = "i3-msg exit" },
    { label = "󰜉", command = "systemctl reboot" },
    { label = "󰐥", command = "systemctl poweroff" }
]

# Sway (Wayland i3 Equivalent)
sway = [
    { label = "󰌾", command = "swaylock" },
    { label = "󰍃", command = "swaymsg exit" },
    { label = "󰜉", command = "systemctl reboot" },
    { label = "󰐥", command = "systemctl poweroff" }
]

# COSMIC DE (Pop!_OS)
cosmic = [
    { label = "󰌾", command = "system76-power suspend" },
    { label = "󰍃", command = "system76-power shutdown" },
    { label = "󰜉", command = "systemctl reboot" },
    { label = "󰐥", command = "systemctl poweroff" }
]

# Default (Fallback for unknown DEs)
default = [
    { label = "󰌾", command = "xdg-screensaver lock" },
    { label = "󰜉", command = "systemctl reboot" },
    { label = "󰐥", command = "systemctl poweroff" }
]
EOF

echo "Installing default style.css..."
write_file "$STYLE_FILE" <<'EOF'
/* Define color palette as CSS variables */
:root {
    --palette-8: rgba(43, 53, 48, 1);
    --palette-15: rgba(242, 236, 228, 1);
    --palette-7: rgba(215, 211, 204, 1);
    --palette-2: rgba(107, 133, 124, 1);
    --palette-9: rgba(217, 140, 80, .4);
    --background: rgba(32, 44, 40, .5);
}

button {
    background-color: var(--palette-8);
    color: var(--palette-15);
    font-family: "Mononoki Nerd Font", monospace;
    font-size: 72px;
    border-radius: 7px;
    border: none;
    transition: all 0.2s ease;
    box-shadow: 0 4px 6px rgba(0, 0, 0, 0.1);
    padding: 2px;

    outline: none;
}

button:hover, button:active, button:focus {
    background-color: var(--palette-2);
    color: var(--palette-7);
    transform: translateY(-2px);
    border: 1px solid var(--palette-9);
}

window {
    background-color: var(--background);
    border: 0px solid var(--palette-9);
    padding: 2px;
    border-radius: 11px;
}

grid {
    padding: 1px;
}
EOF

echo "Installation complete. System-wide configuration installed to: $CONFIG_DIR"