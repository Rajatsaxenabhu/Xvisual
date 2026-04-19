#!/usr/bin/env bash
set -e

APP_NAME="xvisual"
REPO="Rajatsaxenabhu/Rajatsaxenabhu"
INSTALL_DIR="/usr/local/bin"
DESKTOP_DIR="/usr/share/applications"
ICON_DIR="/usr/share/icons/hicolor/256x256/apps"

# Accept a direct download URL as first argument, otherwise use latest GitHub release
DOWNLOAD_URL="${1:-https://github.com/$REPO/releases/download/latest/$APP_NAME}"

echo "==> Updating package lists"
sudo apt-get update -qq

echo "==> Installing runtime dependencies"
ALSA_PKG="libasound2"
apt-cache show libasound2t64 &>/dev/null && ALSA_PKG="libasound2t64"
sudo apt-get install -y \
    libpipewire-0.3-0 \
    libspa-0.2-jack \
    "$ALSA_PKG"

echo "==> Downloading $APP_NAME from:"
echo "    $DOWNLOAD_URL"
TMP_BIN="$(mktemp)"
curl -fSL "$DOWNLOAD_URL" -o "$TMP_BIN"
chmod +x "$TMP_BIN"

echo "==> Installing binary to $INSTALL_DIR/$APP_NAME"
sudo install -m 755 "$TMP_BIN" "$INSTALL_DIR/$APP_NAME"
rm -f "$TMP_BIN"

echo "==> Creating desktop entry"
sudo mkdir -p "$DESKTOP_DIR"
sudo tee "$DESKTOP_DIR/$APP_NAME.desktop" > /dev/null <<EOF
[Desktop Entry]
Version=1.0
Name=Xvisual
Comment=Audio Visualizer
Exec=$APP_NAME
Icon=$APP_NAME
Terminal=true
Type=Application
Categories=AudioVideo;Audio;
Keywords=audio;visualizer;music;
EOF

sudo update-desktop-database "$DESKTOP_DIR" 2>/dev/null || true

echo ""
echo "==> $APP_NAME installed successfully!"
echo "    Binary:  $INSTALL_DIR/$APP_NAME"
echo "    Desktop: $DESKTOP_DIR/$APP_NAME.desktop"
echo ""
echo "Run it with:  $APP_NAME"
