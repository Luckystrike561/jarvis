{
  description = "A beautiful TUI for managing and executing bash scripts with zero configuration";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, nixpkgs, flake-utils, rust-overlay }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
          inherit system overlays;
        };
        
        rustToolchain = pkgs.rust-bin.stable.latest.default.override {
          extensions = [ "rust-src" "rustfmt" "clippy" ];
        };

        jarvis = pkgs.rustPlatform.buildRustPackage rec {
          pname = "jarvis-tui";
          version = "0.1.0";

          src = ./.;

          cargoLock = {
            lockFile = ./Cargo.lock;
          };

          nativeBuildInputs = with pkgs; [
            pkg-config
          ];

          buildInputs = with pkgs; [
            bash
          ];

          # Tests require a valid terminal
          doCheck = false;

          meta = with pkgs.lib; {
            description = "A beautiful TUI for managing and executing bash scripts with zero configuration";
            homepage = "https://github.com/Luckystrike561/jarvis";
            license = licenses.mit;
            maintainers = [ ];
            mainProgram = "jarvis";
            platforms = platforms.unix;
          };
        };

      in
      {
        packages = {
          default = jarvis;
          jarvis-tui = jarvis;
        };

        apps.default = {
          type = "app";
          program = "${jarvis}/bin/jarvis";
        };

        devShells.default = pkgs.mkShell {
          buildInputs = with pkgs; [
            rustToolchain
            cargo
            rustc
            rustfmt
            clippy
            rust-analyzer
            pkg-config
            bash
            shellcheck
            shfmt
            fzf
          ];

          shellHook = ''
            echo "🤖 Jarvis Nix development environment loaded"
            echo ""
            echo "Available commands:"
            echo "  cargo build       - Build the project"
            echo "  cargo run         - Run Jarvis"
            echo "  cargo test        - Run tests"
            echo "  cargo clippy      - Run linter"
            echo "  cargo fmt         - Format code"
            echo "  shellcheck        - Lint bash scripts"
            echo "  nix build         - Build with Nix"
            echo "  nix run           - Run with Nix"
            echo ""
          '';
        };
      }
    );
}
