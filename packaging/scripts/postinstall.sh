#!/bin/sh
set -e

if ! id ports >/dev/null 2>&1; then
    useradd --system --no-create-home --home-dir /var/lib/ports --shell /usr/sbin/nologin ports 2>/dev/null \
        || adduser --system --home /var/lib/ports --shell /usr/sbin/nologin ports 2>/dev/null \
        || true
fi

mkdir -p /etc/ports/config /etc/ports/keys /var/lib/ports /var/log/ports
chmod 0750 /etc/ports/keys
chmod 0755 /etc/ports/config

if command -v systemctl >/dev/null 2>&1; then
    systemctl daemon-reload || true
fi

echo "ports-agent installed."
echo "Enroll this node with:"
echo "  sudo portsctl login <backend-url> <enrollment-token>            # reverse mode"
echo "  sudo portsctl login <backend-url> <enrollment-token> --mode ssh # direct SSH mode"
