#!/usr/bin/env bash
set -e

echo "=========================================================="
echo "    Supervision Desktop - macOS LaunchDaemon Setup        "
echo "=========================================================="

if [ "$EUID" -ne 0 ]; then
  echo "Error: Please run as root (e.g. sudo bash install-macos.sh)"
  exit 1
fi

INSTALL_DIR="/usr/local/bin"
PLIST_PATH="/Library/LaunchDaemons/com.supervision.daemon.plist"

echo "[1/4] Deploying binaries to $INSTALL_DIR..."
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
TARGET_DIR="$SCRIPT_DIR/../target/release"

mkdir -p "$INSTALL_DIR"
if [ -f "$TARGET_DIR/supervision-daemon" ]; then
    cp "$TARGET_DIR/supervision-daemon" "$INSTALL_DIR/"
    cp "$TARGET_DIR/supervision-cli" "$INSTALL_DIR/"
    cp "$TARGET_DIR/supervision-overlay" "$INSTALL_DIR/"
else
    echo "Compiling release binaries..."
    cargo build --release --bin supervision-daemon --bin supervision-cli --bin supervision-overlay
    cp "$TARGET_DIR/supervision-daemon" "$INSTALL_DIR/"
    cp "$TARGET_DIR/supervision-cli" "$INSTALL_DIR/"
    cp "$TARGET_DIR/supervision-overlay" "$INSTALL_DIR/"
fi

chmod +x "$INSTALL_DIR/supervision-daemon"
chmod +x "$INSTALL_DIR/supervision-cli"
chmod +x "$INSTALL_DIR/supervision-overlay"

echo "[2/4] Generating LaunchDaemon plist..."
cat << 'EOF' > "$PLIST_PATH"
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>com.supervision.daemon</string>
    <key>ProgramArguments</key>
    <array>
        <string>/usr/local/bin/supervision-daemon</string>
        <string>run</string>
    </array>
    <key>RunAtLoad</key>
    <true/>
    <key>KeepAlive</key>
    <true/>
    <key>StandardErrorPath</key>
    <string>/var/log/supervision.err.log</string>
    <key>StandardOutPath</key>
    <string>/var/log/supervision.out.log</string>
</dict>
</plist>
EOF

chmod 644 "$PLIST_PATH"
chown root:wheel "$PLIST_PATH"

echo "[3/4] Unloading previous daemon instance (if any)..."
launchctl bootout system "$PLIST_PATH" 2>/dev/null || true

echo "[4/4] Loading and starting Supervision LaunchDaemon..."
launchctl bootstrap system "$PLIST_PATH"

echo ""
echo "✓ Supervision LaunchDaemon successfully installed and running!"
echo "Check status with: supervision-cli status"
