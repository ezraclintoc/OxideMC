#!/bin/bash

set -e

REPO="ezraclintoc/OxideMC"
INSTALL_DIR="${HOME}/.local/bin"
BINARY_NAME="oxidemc"

detect_os() {
    case "$(uname -s)" in
        Linux*)     echo "linux";;
        Darwin*)    echo "macos";;
        *)          echo "unknown";;
    esac
}

detect_arch() {
    case "$(uname -m)" in
        x86_64)     echo "x86_64";;
        aarch64|arm64) echo "arm64";;
        *)          echo "unknown";;
    esac
}

get_extension() {
    if [ "$(uname -s)" = "Windows" ] || [ "$(uname -s)" = "MINGW"* ]; then
        echo ".exe"
    else
        echo ""
    fi
}

echo "⬇️  Downloading OxideMC..."

# Get latest release info
LATEST_TAG=$(curl -sL "https://api.github.com/repos/${REPO}/releases/latest" | grep '"tag_name"' | cut -d '"' -f4)

if [ -z "$LATEST_TAG" ]; then
    echo "❌ Error: Could not find latest release"
    exit 1
fi

OS=$(detect_os)
ARCH=$(detect_arch)

if [ "$OS" = "unknown" ]; then
    echo "❌ Error: Unsupported OS"
    exit 1
fi

# Handle macOS special case
if [ "$OS" = "macos" ]; then
    if [ "$ARCH" = "arm64" ]; then
        FILENAME="oxidemc-macos-arm64"
    else
        FILENAME="oxidemc-x86_64"
    fi
else
    FILENAME="oxidemc-linux-${ARCH}"
fi

# Add .exe for Windows
if [ "$(uname -s)" = "MINGW"* ] || [ "$(uname -s)" = "Windows"* ]; then
    FILENAME="${FILENAME}.exe"
fi

DOWNLOAD_URL="https://github.com/${REPO}/releases/download/${LATEST_TAG}/${FILENAME}"

# Create install directory
mkdir -p "$INSTALL_DIR"

# Download binary
echo "📦 Downloading from: $DOWNLOAD_URL"
curl -sL "$DOWNLOAD_URL" -o "${INSTALL_DIR}/${BINARY_NAME}"
chmod +x "${INSTALL_DIR}/${BINARY_NAME}"

echo "✅ Installed to ${INSTALL_DIR}/${BINARY_NAME}"
echo ""
echo "Run 'oxidemc' to start!"
