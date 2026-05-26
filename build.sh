#!/bin/bash
# Build script for netwatch-monitor

set -e

# Load Rust environment if available
if [ -f "$HOME/.cargo/env" ]; then
    source "$HOME/.cargo/env"
fi

echo "🔨 Building netwatch-monitor..."

# Check if Rust is installed
if ! command -v cargo &> /dev/null; then
    echo "❌ Rust is not installed. Please install Rust first:"
    echo "   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
    exit 1
fi

# Build in release mode
cargo build --release

echo "✅ Build complete!"
echo "📦 Binary: ./target/release/netwatch-monitor"
