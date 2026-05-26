#!/bin/bash
# Install script for netwatch-monitor

set -e

# Load Rust environment if available
if [ -f "$HOME/.cargo/env" ]; then
    source "$HOME/.cargo/env"
fi

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BINARY_NAME="netwatch-monitor"

echo "📦 Installing netwatch-monitor..."

# Check if binary exists
if [ ! -f "$SCRIPT_DIR/target/release/$BINARY_NAME" ]; then
    echo "❌ Binary not found. Run './build.sh' first."
    exit 1
fi

# Try system-wide installation (requires sudo)
if [ "$EUID" -eq 0 ]; then
    INSTALL_DIR="/usr/local/bin"
    cp "$SCRIPT_DIR/target/release/$BINARY_NAME" "$INSTALL_DIR/$BINARY_NAME"
    chmod 755 "$INSTALL_DIR/$BINARY_NAME"
    echo "✅ Installed to $INSTALL_DIR/$BINARY_NAME"
else
    # User-wide installation
    INSTALL_DIR="$HOME/.local/bin"
    mkdir -p "$INSTALL_DIR"
    cp "$SCRIPT_DIR/target/release/$BINARY_NAME" "$INSTALL_DIR/$BINARY_NAME"
    chmod 755 "$INSTALL_DIR/$BINARY_NAME"
    
    # Check if bin is in PATH
    if [[ ":$PATH:" != *":$HOME/.local/bin:"* ]]; then
        echo ""
        echo "⚠️  Add to PATH by running:"
        echo "   export PATH=\"\$HOME/.local/bin:\$PATH\""
        echo ""
        echo "   Or add to ~/.bashrc / ~/.zshrc:"
        echo "   echo 'export PATH=\"\$HOME/.local/bin:\$PATH\"' >> ~/.bashrc"
    fi
    
    echo "✅ Installed to $INSTALL_DIR/$BINARY_NAME"
fi

echo "🚀 Run '$BINARY_NAME' to start"
