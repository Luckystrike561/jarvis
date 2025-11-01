# JARVIS TUI - Next Generation Design

## Vision

A **Rust-based TUI application** that automatically discovers bash scripts and presents them in a beautiful, interactive interface with real-time output.

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│  JARVIS v2.0 - Just Another Rather Very Intelligent System  │
├──────────────┬──────────────────────────────────────────────┤
│              │                                              │
│  📁 Scripts  │  🖥️  Script Details                         │
│              │                                              │
│  System/     │  Name: Install Nix Package Manager          │
│  ▶ Complete  │  File: fedora.sh::install_nix               │
│    laptop    │  Description: Installs Nix using            │
│    Install   │  Determinate Systems installer              │
│    Nix       │                                              │
│    Docker    │  ────────────────────────────────────────   │
│              │                                              │
│  Homelab/    │  💬 Output:                                 │
│    K3S       │  ┌─────────────────────────────────────┐   │
│    ArgoCD    │  │ Downloading installer...            │   │
│              │  │ Installing Nix...                   │   │
│  Utilities/  │  │ ✅ Nix installed successfully!      │   │
│    S.M.A.R.T │  └─────────────────────────────────────┘   │
│              │                                              │
└──────────────┴──────────────────────────────────────────────┘
[↑↓] Navigate  [Enter] Execute  [Tab] Switch Pane  [Q] Quit
```
