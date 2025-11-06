#!/usr/bin/env bash
# Script to update Homebrew formula with latest release

set -e

# Get the latest version from Cargo.toml
VERSION=$(grep -E '^version = ' Cargo.toml | head -1 | sed 's/version = "\(.*\)"/\1/')

# Get the SHA256 of the release tarball
TARBALL_URL="https://github.com/Luckystrike561/jarvis/archive/refs/tags/v${VERSION}.tar.gz"
echo "Downloading tarball from: $TARBALL_URL"
SHA256=$(curl -sL "$TARBALL_URL" | sha256sum | cut -d' ' -f1)

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

echo "✅ Formula updated successfully!"
echo "Formula location: homebrew/jarvis.rb"
