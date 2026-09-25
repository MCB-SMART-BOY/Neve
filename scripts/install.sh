#!/bin/sh
# n3v3 Linux Installer
# Usage: curl -fsSL https://raw.githubusercontent.com/MCB-SMART-BOY/n3v3/master/scripts/install.sh | sh

set -e

echo ""
echo "    _   __                "
echo "   / | / /___  _   _____  "
echo "  /  |/ / _ \| | / / _ \ "
echo " / /|  /  __/| |/ /  __/ "
echo "/_/ |_/\___/ |___/\___/  "
echo ""
echo "n3v3 Installer for Linux"
echo ""

# Detect architecture
ARCH=$(uname -m)
case "$ARCH" in
    x86_64)
        TARGET="x86_64-unknown-linux-gnu"
        ;;
    aarch64|arm64)
        TARGET="aarch64-unknown-linux-gnu"
        ;;
    *)
        echo "Error: Unsupported architecture: $ARCH"
        echo "Supported: x86_64, aarch64"
        exit 1
        ;;
esac

# Get latest version
echo "Fetching latest release..."
VERSION=$(curl -fsSL https://api.github.com/repos/MCB-SMART-BOY/n3v3/releases/latest | grep '"tag_name"' | cut -d'"' -f4)
if [ -z "$VERSION" ]; then
    echo "Error: Failed to get latest version"
    exit 1
fi
echo "Latest version: $VERSION"

# Download URL
URL="https://github.com/MCB-SMART-BOY/n3v3/releases/download/${VERSION}/n3v3-${TARGET}.tar.gz"

# Install directory
INSTALL_DIR="${HOME}/.local/bin"
mkdir -p "$INSTALL_DIR"

# Download and extract
echo "Downloading n3v3-${TARGET}.tar.gz..."
TEMP_DIR=$(mktemp -d)
curl -fsSL "$URL" -o "$TEMP_DIR/n3v3.tar.gz"

echo "Extracting..."
tar -xzf "$TEMP_DIR/n3v3.tar.gz" -C "$TEMP_DIR"
mv "$TEMP_DIR/n3v3" "$INSTALL_DIR/n3v3"
chmod +x "$INSTALL_DIR/n3v3"
rm -rf "$TEMP_DIR"

# Verify
echo ""
echo "Verifying installation..."
if "$INSTALL_DIR/n3v3" --version; then
    echo ""
    echo "n3v3 installed successfully!"
    echo ""
    echo "Installation path: $INSTALL_DIR/n3v3"
    echo ""
    
    # Check if in PATH
    case ":$PATH:" in
        *":$INSTALL_DIR:"*)
            echo "You can now use 'n3v3' command."
            ;;
        *)
            echo "Add this to your ~/.bashrc or ~/.zshrc:"
            echo ""
            echo "    export PATH=\"\$HOME/.local/bin:\$PATH\""
            echo ""
            echo "Then restart your terminal or run: source ~/.bashrc"
            ;;
    esac
    
    echo ""
    echo "Quick start:"
    echo "  n3v3 repl          # Start interactive REPL"
    echo "  n3v3 doc           # View documentation"
    echo "  n3v3 eval '1 + 2'  # Evaluate expression"
else
    echo "Error: Installation failed"
    exit 1
fi
