#!/usr/bin/env bash
set -e

echo "=========================================================="
echo "    Supervision Desktop - Linux Systemd Installation      "
echo "=========================================================="

if [ "$EUID" -ne 0 ]; then
  echo "Error: Please run as root (e.g. sudo bash install-linux.sh)"
  exit 1
fi

INSTALL_DIR="/usr/local/bin"
CONFIG_DIR="/etc/supervision"

echo "[1/4] Creating configuration directory..."
mkdir -p "$CONFIG_DIR"

echo "[2/4] Deploying binaries to $INSTALL_DIR..."
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
TARGET_DIR="$SCRIPT_DIR/../target/release"

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

echo "[3/4] Installing systemd unit..."
cat << 'EOF' > /etc/systemd/system/supervision.service
[Unit]
Description=Supervision Desktop Privacy-First Digital Self-Control Service
After=network.target

[Service]
Type=simple
User=root
ExecStart=/usr/local/bin/supervision-daemon run
Restart=always
RestartSec=1s
KillMode=process
LimitNOFILE=65535

[Install]
WantedBy=multi-user.target
EOF

echo "[4/4] Reloading systemd and enabling service..."
systemctl daemon-reload
systemctl enable supervision.service
systemctl restart supervision.service

echo ""
echo "✓ Supervision Service successfully installed and running!"
echo "Check status with: supervision-cli status"
