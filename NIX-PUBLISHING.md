# Publishing Jarvis to nixpkgs

This guide explains how to make Jarvis available to the Nix/NixOS community and via Devbox.

## Quick Start - Using the Flake

Users can already try Jarvis with Nix flakes:

```bash
# Run directly
nix run github:Luckystrike561/jarvis

# Install to profile
nix profile install github:Luckystrike561/jarvis

# Use in devbox
devbox add github:Luckystrike561/jarvis
```

## Publishing Options

### 1. FlakeHub (Automatic - ✅ Configured)

FlakeHub provides automatic publishing when releases are created.

**Status:** ✅ Configured via `.github/workflows/flakehub.yml`

**When you create a release:**
1. Tag a release: `git tag v0.1.0 && git push origin v0.1.0`
2. The workflow automatically publishes to FlakeHub
3. Users can install: `nix profile install github:Luckystrike561/jarvis#v0.1.0`

**FlakeHub Setup (One-time):**
1. Visit https://flakehub.com
2. Sign in with GitHub
3. Enable the repository
4. Workflow handles the rest automatically!

### 2. nixpkgs (Manual - Requires PR)

To make Jarvis available via `nix-env -iA nixpkgs.jarvis-tui` and `devbox add jarvis-tui`:

#### Step 1: Wait for v0.1.0 Release

Create a proper release first:
```bash
git tag v0.1.0
git push origin v0.1.0
```

This triggers:
- GitHub Release creation
- Binary builds for all platforms
- FlakeHub publication

#### Step 2: Generate Nix Hash

```bash
# Get the tarball hash
nix-prefetch-url --unpack https://github.com/Luckystrike561/jarvis/archive/refs/tags/v0.1.0.tar.gz

# Or use nix-prefetch-github
nix-shell -p nix-prefetch-github --run "nix-prefetch-github Luckystrike561 jarvis --rev v0.1.0"
```

#### Step 3: Create nixpkgs Derivation

The derivation file for nixpkgs (save this for the PR):

```nix
# File: pkgs/by-name/ja/jarvis-tui/package.nix
{ lib
, rustPlatform
, fetchFromGitHub
, pkg-config
, bash
}:

rustPlatform.buildRustPackage rec {
  pname = "jarvis-tui";
  version = "0.1.0";

  src = fetchFromGitHub {
    owner = "Luckystrike561";
    repo = "jarvis";
    rev = "v${version}";
    hash = "sha256-AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA="; # Replace with actual hash
  };

  cargoHash = "sha256-BBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBB="; # Will be auto-generated

  nativeBuildInputs = [
    pkg-config
  ];

  buildInputs = [
    bash
  ];

  # Tests require a terminal
  doCheck = false;

  meta = with lib; {
    description = "A beautiful TUI for managing and executing bash scripts with zero configuration";
    homepage = "https://github.com/Luckystrike561/jarvis";
    license = licenses.mit;
    maintainers = with maintainers; [ ]; # Add your nixpkgs handle here
    mainProgram = "jarvis";
    platforms = platforms.unix;
  };
}
```

#### Step 4: Submit nixpkgs PR

1. Fork nixpkgs: https://github.com/NixOS/nixpkgs
2. Create branch: `git checkout -b jarvis-tui-init`
3. Add the package file: `pkgs/by-name/ja/jarvis-tui/package.nix`
4. Test it locally:
   ```bash
   nix-build -A jarvis-tui
   ./result/bin/jarvis
   ```
5. Commit and push:
   ```bash
   git add pkgs/by-name/ja/jarvis-tui/package.nix
   git commit -m "jarvis-tui: init at 0.1.0"
   git push origin jarvis-tui-init
   ```
6. Create PR to nixpkgs with description:
   ```markdown
   # jarvis-tui: init at 0.1.0
   
   A beautiful TUI for managing and executing bash scripts with zero configuration.
   
   ## Description
   Jarvis is a modern terminal user interface built with Rust and Ratatui that 
   automatically discovers bash functions in scripts and presents them in an 
   organized, easy-to-navigate interface.
   
   ## Features
   - Zero configuration
   - Auto-discovery of bash scripts
   - Beautiful TUI interface
   - Cross-platform (Linux, macOS, Windows)
   
   ## Testing
   - [x] Built successfully on x86_64-linux
   - [x] Ran `nix-build -A jarvis-tui`
   - [x] Tested basic functionality
   
   ## Links
   - Homepage: https://github.com/Luckystrike561/jarvis
   - License: MIT
   ```

#### Step 5: Respond to Review

nixpkgs maintainers will review (usually within 1-2 weeks):
- Respond to any feedback
- Make requested changes
- Once approved, it will be merged!

#### Step 6: Available in nixpkgs

After merge (next channel update):
```bash
# Via nix-env
nix-env -iA nixpkgs.jarvis-tui

# Via nix-shell
nix-shell -p jarvis-tui

# Via devbox
devbox add jarvis-tui

# Via flake
nix run nixpkgs#jarvis-tui
```

## Updating Versions

For future releases:

### Update Flake
1. Update version in `flake.nix`
2. Commit changes
3. Create new tag: `git tag v0.2.0 && git push origin v0.2.0`
4. FlakeHub auto-publishes

### Update nixpkgs
1. Fork nixpkgs again
2. Update `pkgs/by-name/ja/jarvis-tui/package.nix` version
3. Update hashes with new version
4. Submit PR: `jarvis-tui: 0.1.0 -> 0.2.0`

## Devbox Integration

Once in nixpkgs, users can add Jarvis to any project:

```bash
# Add to project
devbox add jarvis-tui

# Or use in devbox.json
{
  "packages": ["jarvis-tui@latest"]
}
```

## Benefits of Publishing

✅ **FlakeHub (Immediate)**:
- Users can install immediately via flakes
- Automatic updates on releases
- Works with `nix run`, `nix profile install`

✅ **nixpkgs (After PR Merge)**:
- Official NixOS/nixpkgs package
- Available via `devbox add jarvis-tui`
- Included in NixOS configurations
- Binary cache (no compilation needed)
- Part of official ecosystem

## Resources

- **Nix Flakes:** https://nixos.wiki/wiki/Flakes
- **FlakeHub:** https://flakehub.com
- **nixpkgs Contributing:** https://github.com/NixOS/nixpkgs/blob/master/CONTRIBUTING.md
- **Package Naming:** https://nixos.org/manual/nixpkgs/stable/#sec-package-naming
- **Rust in nixpkgs:** https://nixos.org/manual/nixpkgs/stable/#rust

## Current Status

- ✅ Nix flake created (`flake.nix`)
- ✅ FlakeHub workflow configured (`.github/workflows/flakehub.yml`)
- ✅ `Cargo.lock` tracked in git
- ⏳ Waiting for v0.1.0 release
- ⏳ nixpkgs PR (after first release)

## Quick Commands

```bash
# Test the flake locally
nix flake check
nix build
nix run

# Test in devbox
devbox shell
devbox run dev

# Create release
git tag v0.1.0
git push origin v0.1.0

# After release, users can:
nix run github:Luckystrike561/jarvis
devbox add github:Luckystrike561/jarvis
```
