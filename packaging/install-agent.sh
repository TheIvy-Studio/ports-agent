#!/bin/sh
set -e

BACKEND=""
TOKEN=""
MODE="reverse"
RELEASE_BASE="${PORTS_RELEASE_BASE:-https://github.com/IMDelewer/ports/releases/latest/download}"

while [ $# -gt 0 ]; do
    case "$1" in
        --backend) [ $# -ge 2 ] || { echo "--backend requires a value" >&2; exit 1; }; BACKEND="$2"; shift 2 ;;
        --token) [ $# -ge 2 ] || { echo "--token requires a value" >&2; exit 1; }; TOKEN="$2"; shift 2 ;;
        --mode) [ $# -ge 2 ] || { echo "--mode requires a value" >&2; exit 1; }; MODE="$2"; shift 2 ;;
        *) echo "unknown argument: $1" >&2; exit 1 ;;
    esac
done

if [ "$(id -u)" -ne 0 ]; then
    echo "this installer must run as root (use sudo)" >&2
    exit 1
fi

case "$(uname -m)" in
    x86_64|amd64) ARCH="amd64" ;;
    aarch64|arm64) ARCH="arm64" ;;
    *) ARCH="$(uname -m)" ;;
esac

install_deb() { tmp="$(mktemp)"; curl -fsSL "$RELEASE_BASE/ports-agent_${ARCH}.deb" -o "$tmp" && dpkg -i "$tmp"; rm -f "$tmp"; }
install_rpm() { tmp="$(mktemp)"; curl -fsSL "$RELEASE_BASE/ports-agent_${ARCH}.rpm" -o "$tmp" && rpm -Uvh --replacepkgs "$tmp"; rm -f "$tmp"; }
install_apk() { tmp="$(mktemp)"; curl -fsSL "$RELEASE_BASE/ports-agent_${ARCH}.apk" -o "$tmp" && apk add --allow-untrusted "$tmp"; rm -f "$tmp"; }
install_pacman() { tmp="$(mktemp --suffix=.pkg.tar.zst)"; curl -fsSL "$RELEASE_BASE/ports-agent_${ARCH}.pkg.tar.zst" -o "$tmp" && pacman -U --noconfirm "$tmp"; rm -f "$tmp"; }

build_from_source() {
    echo "no prebuilt package available; building from source with cargo"
    command -v cargo >/dev/null 2>&1 || { echo "cargo is required to build from source" >&2; exit 1; }
    cargo build --release --locked
    install -Dm755 target/release/ports-agent /usr/bin/ports-agent
    install -Dm755 target/release/portsctl /usr/bin/portsctl
    install -Dm644 packaging/systemd/ports-agent.service /usr/lib/systemd/system/ports-agent.service
    sh packaging/scripts/postinstall.sh
}

installed=0
if command -v apt-get >/dev/null 2>&1; then
    install_deb && installed=1 || true
elif command -v dnf >/dev/null 2>&1 || command -v yum >/dev/null 2>&1; then
    install_rpm && installed=1 || true
elif command -v pacman >/dev/null 2>&1; then
    install_pacman && installed=1 || true
elif command -v apk >/dev/null 2>&1; then
    install_apk && installed=1 || true
fi

if [ "$installed" -ne 1 ]; then
    if [ -f Cargo.toml ]; then
        build_from_source
    else
        echo "could not install a prebuilt package and no source tree present" >&2
        exit 1
    fi
fi

if [ -n "$BACKEND" ] && [ -n "$TOKEN" ]; then
    portsctl login "$BACKEND" "$TOKEN" --mode "$MODE"
else
    echo "ports-agent installed. Enroll with: sudo portsctl login <backend-url> <enrollment-token> --mode $MODE"
fi
