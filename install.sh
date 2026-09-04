#!/bin/sh
set -e

REPO="hemanth/episod"
INSTALL_DIR="${INSTALL_DIR:-/usr/local/bin}"

OS="$(uname -s | tr '[:upper:]' '[:lower:]')"
ARCH="$(uname -m)"

case "$ARCH" in
  x86_64|amd64)
    ARCH="x86_64"
    ;;
  arm64|aarch64)
    ARCH="aarch64"
    ;;
  *)
    echo "Unsupported architecture: $ARCH"
    exit 1
    ;;
esac

case "$OS" in
  darwin)
    TARGET="${ARCH}-apple-darwin"
    ;;
  linux)
    TARGET="${ARCH}-unknown-linux-musl"
    ;;
  *)
    echo "Unsupported operating system: $OS"
    exit 1
    ;;
esac

echo "Fetching latest episod release for ${TARGET}..."
RELEASE_URL="https://github.com/${REPO}/releases/latest/download/episod-${TARGET}.tar.gz"

TMP_DIR="$(mktemp -d)"
trap 'rm -rf "$TMP_DIR"' EXIT

if command -v curl >/dev/null 2>&1; then
  curl -fsSL "$RELEASE_URL" | tar -xz -C "$TMP_DIR"
elif command -v wget >/dev/null 2>&1; then
  wget -qO- "$RELEASE_URL" | tar -xz -C "$TMP_DIR"
else
  echo "Error: curl or wget is required"
  exit 1
fi

if [ -w "$INSTALL_DIR" ]; then
  mv "$TMP_DIR/episod" "$INSTALL_DIR/episod"
else
  echo "Elevated permissions required to install to $INSTALL_DIR"
  sudo mv "$TMP_DIR/episod" "$INSTALL_DIR/episod"
fi

chmod +x "$INSTALL_DIR/episod"
echo "Successfully installed episod to $INSTALL_DIR/episod"
