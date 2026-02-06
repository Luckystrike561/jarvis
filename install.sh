#!/usr/bin/env bash
# Jarvis installer script
# Usage: curl -fsSL https://raw.githubusercontent.com/Luckystrike561/jarvis/main/install.sh | bash
#
# Environment variables:
#   INSTALL_DIR - Installation directory (default: ~/.local/bin)
#   VERSION     - Specific version to install (default: latest)

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

REPO="Luckystrike561/jarvis"
BINARY_NAME="jarvis"

info() {
    printf "${BLUE}info${NC}: %s\n" "$1"
}

success() {
    printf "${GREEN}success${NC}: %s\n" "$1"
}

warn() {
    printf "${YELLOW}warning${NC}: %s\n" "$1"
}

error() {
    printf "${RED}error${NC}: %s\n" "$1" >&2
    exit 1
}

detect_platform() {
    local os arch

    os="$(uname -s)"
    arch="$(uname -m)"

    case "$os" in
        Linux)
            OS="linux"
            ;;
        Darwin)
            OS="macos"
            ;;
        *)
            error "Unsupported operating system: $os"
            ;;
    esac

    case "$arch" in
        x86_64|amd64)
            ARCH="x86_64"
            ;;
        aarch64|arm64)
            ARCH="aarch64"
            ;;
        *)
            error "Unsupported architecture: $arch"
            ;;
    esac

    # Validate supported platform combinations
    if [[ "$OS" == "linux" && "$ARCH" != "x86_64" ]]; then
        error "Linux $ARCH is not currently supported. Only x86_64 is available."
    fi

    if [[ "$OS" == "macos" && "$ARCH" != "aarch64" ]]; then
        error "macOS $ARCH is not currently supported. Only Apple Silicon (aarch64) is available."
    fi

    PLATFORM="${OS}-${ARCH}"
    info "Detected platform: $PLATFORM"
}

check_dependencies() {
    if ! command -v curl &> /dev/null; then
        error "curl is required but not installed"
    fi
}

get_latest_version() {
    local version

    info "Fetching latest version..."

    version=$(curl -fsSL "https://api.github.com/repos/$REPO/releases/latest" 2>/dev/null | \
        grep '"tag_name"' | \
        sed -E 's/.*"tag_name": *"([^"]+)".*/\1/')

    if [[ -z "$version" ]]; then
        error "Failed to fetch latest version. Please check your internet connection."
    fi

    echo "$version"
}

download_and_install() {
    local version="$1"
    local install_dir="$2"
    local asset_name="jarvis-${PLATFORM}"
    local download_url="https://github.com/$REPO/releases/download/${version}/${asset_name}"
    local checksum_url="${download_url}.sha256"
    local tmp_dir

    tmp_dir=$(mktemp -d)
    trap 'rm -rf "$tmp_dir"' EXIT

    info "Downloading Jarvis ${version} for ${PLATFORM}..."

    # Download binary
    if ! curl -fsSL "$download_url" -o "$tmp_dir/$asset_name"; then
        error "Failed to download binary. The release may not exist for this platform."
    fi

    # Download and verify checksum
    info "Verifying checksum..."
    if curl -fsSL "$checksum_url" -o "$tmp_dir/${asset_name}.sha256" 2>/dev/null; then
        local expected_checksum actual_checksum
        expected_checksum=$(awk '{print $1}' "$tmp_dir/${asset_name}.sha256")

        if command -v sha256sum &> /dev/null; then
            actual_checksum=$(sha256sum "$tmp_dir/$asset_name" | awk '{print $1}')
        elif command -v shasum &> /dev/null; then
            actual_checksum=$(shasum -a 256 "$tmp_dir/$asset_name" | awk '{print $1}')
        else
            warn "Neither sha256sum nor shasum found, skipping checksum verification"
            expected_checksum=""
        fi

        if [[ -n "$expected_checksum" && "$expected_checksum" != "$actual_checksum" ]]; then
            error "Checksum verification failed! Expected: $expected_checksum, Got: $actual_checksum"
        fi

        if [[ -n "$expected_checksum" ]]; then
            success "Checksum verified"
        fi
    else
        warn "Could not download checksum file, skipping verification"
    fi

    # Create install directory if needed
    mkdir -p "$install_dir"

    # Install binary
    info "Installing to $install_dir/$BINARY_NAME..."
    cp "$tmp_dir/$asset_name" "$install_dir/$BINARY_NAME"
    chmod +x "$install_dir/$BINARY_NAME"

    success "Jarvis ${version} installed successfully!"
}

check_path() {
    local install_dir="$1"

    if [[ ":$PATH:" != *":$install_dir:"* ]]; then
        echo ""
        warn "$install_dir is not in your PATH"
        echo ""
        echo "Add it to your shell configuration:"
        echo ""
        echo "  For bash (~/.bashrc):"
        echo "    export PATH=\"$install_dir:\$PATH\""
        echo ""
        echo "  For zsh (~/.zshrc):"
        echo "    export PATH=\"$install_dir:\$PATH\""
        echo ""
        echo "  For fish (~/.config/fish/config.fish):"
        echo "    fish_add_path $install_dir"
        echo ""
    fi
}

main() {
    echo ""
    echo "  Jarvis Installer"
    echo "  ================"
    echo ""

    check_dependencies
    detect_platform

    # Use VERSION env var or fetch latest
    local version="${VERSION:-}"
    if [[ -z "$version" ]]; then
        version=$(get_latest_version)
    fi

    # Use INSTALL_DIR env var or default to ~/.local/bin
    local install_dir="${INSTALL_DIR:-$HOME/.local/bin}"

    download_and_install "$version" "$install_dir"
    check_path "$install_dir"

    echo ""
    echo "Run 'jarvis --help' to get started!"
    echo ""
}

main "$@"
