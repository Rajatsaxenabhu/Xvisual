#!/usr/bin/env bash
set -e

echo "==> updating package lists"
sudo apt-get update

echo "==> installing build tools"
sudo apt-get install -y \
    build-essential \
    pkg-config \
    clang \
    libclang-dev

echo "==> installing PipeWire dev headers"
sudo apt-get install -y \
    libpipewire-0.3-dev \
    libspa-0.2-dev

echo "==> installing ALSA dev headers (required by cpal)"
sudo apt-get install -y \
    libasound2-dev

echo "==> checking for Rust toolchain"
if ! command -v cargo &>/dev/null; then
    echo "==> Rust not found — installing via rustup"
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --no-modify-path
    source "$HOME/.cargo/env"
else
    echo "==> Rust already installed ($(rustc --version))"
fi

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BUILD="$SCRIPT_DIR/xvisual-build/xvisual"

echo "==> running xvisual build"
chmod +x "$BUILD"
"$BUILD"
