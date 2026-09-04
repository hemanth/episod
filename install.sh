#!/bin/sh
set -e

REPO="hemanth/episod"
VERSION="${VERSION:-latest}"

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

# Determine installation directory
if [ -z "$INSTALL_DIR" ]; then
  if [ -w "/usr/local/bin" ]; then
    INSTALL_DIR="/usr/local/bin"
  elif [ -d "$HOME/.local/bin" ] || [ -w "$HOME" ]; then
    INSTALL_DIR="$HOME/.local/bin"
    mkdir -p "$INSTALL_DIR"
  else
    INSTALL_DIR="/usr/local/bin"
  fi
fi

if [ "$VERSION" = "latest" ]; then
  RELEASE_URL="https://github.com/${REPO}/releases/latest/download/episod-${TARGET}.tar.gz"
else
  RELEASE_URL="https://github.com/${REPO}/releases/download/${VERSION}/episod-${TARGET}.tar.gz"
fi

echo "Installing episod (${TARGET})..."

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

# Verify PATH
case ":$PATH:" in
  *":$INSTALL_DIR:"*) ;;
  *)
    echo ""
    echo "Warning: $INSTALL_DIR is not in your PATH."
    echo "Add it by running:"
    echo "  export PATH=\"$INSTALL_DIR:\$PATH\""
    ;;
esac
