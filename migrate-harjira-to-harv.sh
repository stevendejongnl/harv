#!/bin/bash
# Migrate from harjira to harv systemd timer
#
# This script automates the transition from the old harjira systemd timer to the new harv timer.
# It safely stops and disables the old timer before installing the new one.

set -e  # Exit on error

echo "=== Harjira → Harv Migration Script ==="
echo ""

# Check if old timer exists
if systemctl --user is-enabled harjira.timer &>/dev/null || systemctl --user is-active harjira.timer &>/dev/null; then
    echo "Found existing harjira timer. Stopping and disabling..."

    # Stop the timer
    if systemctl --user is-active harjira.timer &>/dev/null; then
        echo "  Stopping harjira.timer..."
        systemctl --user stop harjira.timer
    fi

    # Stop the service if it's running
    if systemctl --user is-active harjira.service &>/dev/null; then
        echo "  Stopping harjira.service..."
        systemctl --user stop harjira.service
    fi

    # Disable the timer
    if systemctl --user is-enabled harjira.timer &>/dev/null; then
        echo "  Disabling harjira.timer..."
        systemctl --user disable harjira.timer
    fi

    # Remove old timer files
    echo "  Removing old systemd files..."
    rm -f ~/.config/systemd/user/harjira.service
    rm -f ~/.config/systemd/user/harjira.timer

    # Reload systemd
    echo "  Reloading systemd daemon..."
    systemctl --user daemon-reload

    echo "✓ Old harjira timer removed successfully"
    echo ""
else
    echo "No existing harjira timer found. Skipping removal."
    echo ""
fi

# Install new harv timer
echo "Installing new harv timer..."

if [ ! -f "systemd/harv.service" ] || [ ! -f "systemd/harv.timer" ]; then
    echo "Error: Could not find systemd/harv.service or systemd/harv.timer"
    echo "Make sure you're running this script from the harjira project directory."
    exit 1
fi

# Use Makefile to install (cleaner and consistent)
if command -v make &>/dev/null && [ -f "Makefile" ]; then
    echo "  Using Makefile to install..."
    make systemd-install
else
    # Fallback to manual installation
    echo "  Installing manually..."
    mkdir -p ~/.config/systemd/user
    cp systemd/harv.service ~/.config/systemd/user/
    cp systemd/harv.timer ~/.config/systemd/user/
    systemctl --user daemon-reload
    systemctl --user enable harv.timer
    systemctl --user start harv.timer
    echo "✓ Systemd timer installed and started"
fi

echo ""
echo "=== Migration Complete ==="
echo ""
echo "The harv timer is now active. Check status with:"
echo "  systemctl --user status harv.timer"
echo ""
echo "View logs with:"
echo "  journalctl --user -u harv.service -f"
echo ""
echo "Note: Your config was automatically migrated from ~/.config/harjira/ to ~/.config/harv/"
echo "      on first run of the 'harv' command."
