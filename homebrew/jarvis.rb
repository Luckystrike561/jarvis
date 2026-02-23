class Jarvis < Formula
  desc "Beautiful TUI for managing and executing scripts with zero configuration"
  homepage "https://github.com/Luckystrike561/jarvis"
  url "https://github.com/Luckystrike561/jarvis/archive/refs/tags/v0.1.14.tar.gz"
  sha256 "07d10ae09490bb8f6331182ba21d2df5ef7a2edce4a163cedf59110cfc594449"
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
