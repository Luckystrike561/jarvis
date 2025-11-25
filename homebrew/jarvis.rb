class Jarvis < Formula
  desc "Beautiful TUI for managing and executing scripts with zero configuration"
  homepage "https://github.com/Luckystrike561/jarvis"
  url "https://github.com/Luckystrike561/jarvis/archive/refs/tags/v0.1.4.tar.gz"
  sha256 "f9fa751103762efde68f1eb3e584ae8d87fa3a97b669968791221d6bd78f9dd4"
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
