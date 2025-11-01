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

## Features

### 1. **Auto-Discovery**
- Scans `jarvis/` folder structure
- Parses bash scripts for function definitions
- Extracts metadata from comments
- Builds tree structure automatically

### 2. **Smart Script Parsing**
```bash
# In your bash scripts, add metadata:

# @description: Install Nix package manager
# @category: System
# @requires: curl
# @dangerous: false
install_nix() {
    curl --proto '=https' ... | sh -s -- install
}
```

### 3. **Three-Pane Layout**
1. **Left Sidebar (25%)**: Script tree/list
2. **Center Content (50%)**: Description, params, and output
3. **Right Panel (25%)**: Logs, status, history (optional)

### 4. **Interactive Execution**
- Real-time stdout/stderr capture
- Progress indicators
- Ability to send input (for interactive scripts)
- Scrollable output
- Exit code display

### 5. **Advanced Features**
- **Search**: Fuzzy find scripts (like fzf)
- **History**: Recently run commands
- **Favorites**: Pin frequently used scripts
- **Themes**: Catppuccin support (matching your dotfiles)
- **Config**: TOML configuration file
- **Logging**: All executions logged to file

## Implementation Plan

### Tech Stack
- **Language**: Rust 🦀
- **TUI Framework**: [ratatui](https://github.com/ratatui-org/ratatui)
- **Script Parsing**: Custom parser + regex
- **Process Management**: tokio (async runtime)
- **Config**: serde + toml
- **Logging**: tracing

### Project Structure
```
jarvis-tui/
├── Cargo.toml
├── src/
│   ├── main.rs           # Entry point
│   ├── app.rs            # Main application state
│   ├── ui/
│   │   ├── mod.rs
│   │   ├── sidebar.rs    # Left menu
│   │   ├── content.rs    # Center pane
│   │   └── output.rs     # Output display
│   ├── script/
│   │   ├── mod.rs
│   │   ├── parser.rs     # Parse bash scripts
│   │   ├── discovery.rs  # Find scripts
│   │   └── executor.rs   # Run scripts
│   ├── config.rs         # Configuration
│   └── theme.rs          # Color schemes
└── config.toml           # User config
```

### Key Dependencies (Cargo.toml)
```toml
[dependencies]
ratatui = "0.28"
crossterm = "0.28"
tokio = { version = "1", features = ["full"] }
serde = { version = "1", features = ["derive"] }
toml = "0.8"
regex = "1"
walkdir = "2"
anyhow = "1"
tracing = "0.1"
```

## Script Metadata Format

### Option 1: Comment-based (Easy)
```bash
#!/usr/bin/env bash

# JARVIS-META: {
#   "name": "Install Nix",
#   "category": "System",
#   "description": "Install Nix package manager",
#   "dangerous": false,
#   "requires": ["curl"]
# }

install_nix() {
    # function body
}
```

### Option 2: YAML Frontmatter (Flexible)
```bash
#!/usr/bin/env bash
# ---
# name: Install Nix Package Manager
# category: System/Package Managers
# description: |
#   Installs Nix using the Determinate Systems installer.
#   Provides declarative package management.
# tags: [nix, packages, system]
# dangerous: false
# requires: [curl, bash]
# ---

install_nix() {
    curl --proto '=https' --tlsv1.2 -sSf -L \
        https://install.determinate.systems/nix | sh -s -- install
}
```

### Option 3: Separate Manifest (Structured)
```toml
# jarvis/scripts.toml

[[script]]
name = "Install Nix"
file = "fedora.sh"
function = "install_nix"
category = "System"
description = "Install Nix package manager with Determinate Systems installer"
tags = ["nix", "package-manager"]
dangerous = false
requires = ["curl"]

[[script]]
name = "K3S Cluster Setup"
file = "homelab.sh"
function = "install_k3s"
category = "Homelab"
description = "Deploy K3S cluster across multiple nodes"
dangerous = true
requires = ["k3sup", "ssh"]
```

## Comparison with Current Solutions

| Feature | Bash+FZF (Current) | Bash+Raw TUI | Rust+Ratatui (Proposed) |
|---------|-------------------|--------------|-------------------------|
| Speed | ⭐⭐⭐ | ⭐⭐⭐ | ⭐⭐⭐⭐⭐ |
| Reliability | ⭐⭐⭐⭐ | ⭐⭐ | ⭐⭐⭐⭐⭐ |
| Live Output | ❌ | ⚠️ Hard | ✅ Easy |
| Multi-pane | ⚠️ Preview only | ✅ | ✅ |
| Auto-discovery | ❌ Manual | ❌ Manual | ✅ Automatic |
| Search | ✅ Excellent | ⚠️ Manual | ✅ Custom |
| Maintainability | ⭐⭐⭐ | ⭐⭐ | ⭐⭐⭐⭐⭐ |
| Distribution | ⚠️ Multi-file | ⚠️ Multi-file | ✅ Single binary |

## Quick Start Guide

### 1. Bootstrap the Rust Project
```bash
cd /home/luckystrike561/shield/jarvis
cargo new --bin jarvis-tui
cd jarvis-tui
```

### 2. Add Dependencies
```bash
cargo add ratatui crossterm tokio serde toml regex walkdir anyhow
```

### 3. Basic Implementation
See the example code in the next sections...

## Alternative: Python + Textual (Faster Prototyping)

If you want to prototype quickly before committing to Rust:

### Python Version
```python
from textual.app import App, ComposeResult
from textual.widgets import Tree, Static, Log
from textual.containers import Container, Horizontal

class JarvisTUI(App):
    CSS = """
    Horizontal {
        height: 100%;
    }
    Tree {
        width: 30%;
        border-right: solid green;
    }
    #content {
        width: 70%;
        padding: 1;
    }
    """
    
    def compose(self) -> ComposeResult:
        with Horizontal():
            yield Tree("Scripts")
            with Container(id="content"):
                yield Static("Select a script...")
                yield Log()
```

Install with:
```bash
pip install textual
python jarvis_tui.py
```

## Recommended Approach

### Phase 1: Keep Current FZF Version ✅
- Your `jarvis-tui.sh` works well now
- Good for immediate use

### Phase 2: Prototype in Python (1-2 days)
- Use Textual to validate UX
- Test script discovery logic
- Verify output capture works

### Phase 3: Production in Rust (1 week)
- Port proven concepts from Python
- Build production-grade version
- Compile to single binary
- Add to your project

## Example: Minimal Rust TUI

Would you like me to create a minimal working Rust TUI example for your project? It would:
- Auto-discover your bash scripts
- Show them in a sidebar
- Execute them with live output
- ~200 lines of Rust code

## Decision Matrix

Choose based on your priorities:

| Priority | Recommendation |
|----------|---------------|
| **Quick solution** | Stick with jarvis-tui.sh (current) |
| **Learn & prototype** | Python + Textual |
| **Production quality** | Rust + Ratatui |
| **Best performance** | Rust + Ratatui |
| **Easiest maintenance** | Rust + Ratatui |
| **Single binary** | Rust or Go |

## My Final Recommendation

🎯 **Go with Rust + Ratatui** because:

1. ✅ It's what linutil uses (proven approach)
2. ✅ Single binary - no dependencies
3. ✅ Perfect for your use case
4. ✅ Great learning opportunity
5. ✅ Production-ready from day 1
6. ✅ Matches your homelab's technical level (K8s, ESP32, etc.)

Would you like me to:
1. Create a minimal Rust TUI example for your jarvis scripts?
2. Build a Python prototype first?
3. Enhance the current bash+fzf version further?

Let me know what you'd prefer! 🚀
