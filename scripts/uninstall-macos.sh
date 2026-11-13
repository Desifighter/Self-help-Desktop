#!/usr/bin/env bash
set -e

echo "=========================================================="
echo "    Supervision Desktop - macOS Uninstallation            "
echo "=========================================================="

if [ "$EUID" -ne 0 ]; then
  echo "Error: Please run as root (e.g. sudo bash uninstall-macos.sh)"
  exit 1
fi

PLIST_PATH="/Library/LaunchDaemons/com.supervision.daemon.plist"

# Check Anti-Tamper Status
if command -v supervision-cli &> /dev/null; then
    STATUS=$(supervision-cli status 2>&1 || true)
    if echo "$STATUS" | grep -q "LOCKED"; then
        echo ""
        echo "⛔ UNINSTALL REJECTED: Anti-Tamper Protection Plan is Active!"
        echo "To disarm the system, you must start the 15-minute cooling-off period:"
        echo "  $ supervision-cli request-unlock"
        echo "Once the countdown completes, run this uninstallation script again."
        exit 1
    fi
fi

echo "[1/3] Unloading LaunchDaemon..."
launchctl bootout system "$PLIST_PATH" 2>/dev/null || true
rm -f "$PLIST_PATH"

echo "[2/3] Cleaning up hosts file..."
if [ -f /etc/hosts ]; then
    sed -i '' '/# --- BEGIN SUPERVISION DESKTOP MANAGED BLOCKLIST ---/,/# --- END SUPERVISION DESKTOP MANAGED BLOCKLIST ---/d' /etc/hosts
fi

echo "[3/3] Removing binaries..."
rm -f /usr/local/bin/supervision-daemon
rm -f /usr/local/bin/supervision-cli
rm -f /usr/local/bin/supervision-overlay

echo "✓ Supervision Desktop has been safely uninstalled."
