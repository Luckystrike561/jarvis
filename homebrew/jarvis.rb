class Jarvis < Formula
  desc "Beautiful TUI for managing and executing scripts with zero configuration"
  homepage "https://github.com/Luckystrike561/jarvis"
  url "https://github.com/Luckystrike561/jarvis/archive/refs/tags/v0.1.7.tar.gz"
  sha256 ""
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
