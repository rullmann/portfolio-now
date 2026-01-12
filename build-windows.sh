#!/bin/bash
# Windows Release Build Script for Portfolio Now
# Requires: Homebrew, mingw-w64

set -e

echo "=== Portfolio Now Windows Build ==="
echo ""

# Check for Homebrew
if ! command -v brew &> /dev/null; then
    echo "ERROR: Homebrew is not installed."
    echo ""
    echo "Install Homebrew with:"
    echo '  /bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"'
    echo ""
    echo "Then run this script again."
    exit 1
fi

# Check for mingw-w64
if ! command -v x86_64-w64-mingw32-gcc &> /dev/null; then
    echo "Installing mingw-w64 cross-compiler..."
    brew install mingw-w64
fi

# Check for Windows target
if ! rustup target list --installed | grep -q "x86_64-pc-windows-gnu"; then
    echo "Installing Windows target for Rust..."
    rustup target add x86_64-pc-windows-gnu
fi

# Configure cargo for Windows cross-compilation
CARGO_CONFIG="$HOME/.cargo/config.toml"
if ! grep -q "x86_64-pc-windows-gnu" "$CARGO_CONFIG" 2>/dev/null; then
    echo "Configuring cargo for Windows cross-compilation..."
    cat >> "$CARGO_CONFIG" << 'EOF'

[target.x86_64-pc-windows-gnu]
linker = "x86_64-w64-mingw32-gcc"
EOF
fi

echo ""
echo "Building Windows release..."
cd "$(dirname "$0")/apps/desktop"

# Build
pnpm tauri build --target x86_64-pc-windows-gnu

echo ""
echo "=== Build Complete ==="
echo "Output: apps/desktop/src-tauri/target/x86_64-pc-windows-gnu/release/bundle/"
