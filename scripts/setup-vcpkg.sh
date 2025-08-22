#!/bin/bash
# Setup vcpkg for QuickLook development on Linux/macOS

set -e

echo "Setting up vcpkg for QuickLook development..."

# Check if vcpkg directory exists
if [ ! -d "vcpkg" ]; then
    echo "Cloning vcpkg repository..."
    git clone https://github.com/microsoft/vcpkg.git
    cd vcpkg
    ./bootstrap-vcpkg.sh
    cd ..
else
    echo "vcpkg directory already exists, updating..."
    cd vcpkg
    git pull
    cd ..
fi

# Set VCPKG_ROOT environment variable
export VCPKG_ROOT="$(pwd)/vcpkg"

echo "Installing FFmpeg dependencies..."

# Detect architecture and OS
ARCH=$(uname -m)
OS=$(uname -s)

if [ "$OS" = "Linux" ]; then
    if [ "$ARCH" = "x86_64" ]; then
        TRIPLET="x64-linux"
    elif [ "$ARCH" = "aarch64" ]; then
        TRIPLET="arm64-linux"
    else
        echo "Warning: Unknown architecture $ARCH, defaulting to x64-linux"
        TRIPLET="x64-linux"
    fi
elif [ "$OS" = "Darwin" ]; then
    if [ "$ARCH" = "x86_64" ]; then
        TRIPLET="x64-osx"
    elif [ "$ARCH" = "arm64" ]; then
        TRIPLET="arm64-osx"
    else
        echo "Warning: Unknown architecture $ARCH, defaulting to x64-osx"
        TRIPLET="x64-osx"
    fi
else
    echo "Warning: Unknown OS $OS, defaulting to x64-linux"
    TRIPLET="x64-linux"
fi

echo "Using triplet: $TRIPLET"
vcpkg/vcpkg install --triplet $TRIPLET

echo ""
echo "============================================"
echo "vcpkg setup complete!"
echo ""
echo "To use vcpkg in future sessions, run:"
echo "export VCPKG_ROOT=$(pwd)/vcpkg"
echo ""
echo "Or add this to your ~/.bashrc or ~/.zshrc"
echo "============================================"