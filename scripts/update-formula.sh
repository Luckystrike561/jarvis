#!/usr/bin/env bash
# Script to update Homebrew formula with latest release

set -eu

# Get the latest version from Cargo.toml
VERSION=$(grep -E '^version = ' Cargo.toml | head -1 | sed 's/version = "\(.*\)"/\1/')

# Validate version format (semver)
if ! [[ "$VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+(-[a-zA-Z0-9.]+)?$ ]]; then
    echo "ERROR: Invalid version format in Cargo.toml: $VERSION" >&2
    exit 1
fi

# Validate the homebrew directory exists
if [[ ! -d "homebrew" ]]; then
    echo "ERROR: homebrew directory not found. Run from project root." >&2
    exit 1
fi

# Get the SHA256 of the release tarball
TARBALL_URL="https://github.com/Luckystrike561/jarvis/archive/refs/tags/v${VERSION}.tar.gz"
echo "Downloading tarball from: $TARBALL_URL"

# Download to temp file and compute hash
TEMP_FILE=$(mktemp)
trap 'rm -f "$TEMP_FILE"' EXIT

HTTP_CODE=$(curl -sL -w "%{http_code}" -o "$TEMP_FILE" "$TARBALL_URL")
if [[ "$HTTP_CODE" != "200" ]]; then
    echo "ERROR: Failed to download tarball. HTTP status: $HTTP_CODE" >&2
    echo "Make sure the release v$VERSION exists on GitHub." >&2
    exit 1
fi

SHA256=$(sha256sum "$TEMP_FILE" | cut -d' ' -f1)

# Validate SHA256 format
if ! [[ "$SHA256" =~ ^[a-f0-9]{64}$ ]]; then
    echo "ERROR: Invalid SHA256 hash computed: $SHA256" >&2
    exit 1
fi

echo "Version: v$VERSION"
echo "SHA256: $SHA256"

# Update the formula
cat > homebrew/jarvis.rb << EOF
class Jarvis < Formula
  desc "Beautiful TUI for managing and executing scripts with zero configuration"
  homepage "https://github.com/Luckystrike561/jarvis"
  url "https://github.com/Luckystrike561/jarvis/archive/refs/tags/v${VERSION}.tar.gz"
  sha256 "${SHA256}"
  license "MIT"
  head "https://github.com/Luckystrike561/jarvis.git", branch: "main"

  depends_on "rust" => :build

  def install
    system "cargo", "install", *std_cargo_args
  end

  test do
    # Test that the binary exists and runs
    assert_match "jarvis", shell_output("#{bin}/jarvis --help")
  end
end
EOF

echo "Formula updated successfully!"
echo "Formula location: homebrew/jarvis.rb"
