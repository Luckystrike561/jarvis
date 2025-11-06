<div align="center">

# 🤖 Jarvis

<img src="https://github.com/Luckystrike561/shield/raw/main/asset/jarvis.png" width="256" height="256" alt="Jarvis Logo">

### Just Another Rather Very Intelligent System

*Your trusted AI assistant for automating OS setup and homelab management*

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/Rust-1.70+-orange.svg)](https://www.rust-lang.org/)
[![Built with Ratatui](https://img.shields.io/badge/TUI-Ratatui-green.svg)](https://github.com/ratatui-org/ratatui)

[Features](#-features) • [Installation](#-installation) • [Usage](#-usage) • [Documentation](#-documentation)

</div>

---

## 🌟 Overview

Inspired by Tony Stark's legendary AI from Marvel, **Jarvis** is a powerful TUI (Terminal User Interface) application for managing and executing bash scripts and npm scripts. It automatically discovers your bash functions and npm scripts, presenting them in a beautiful, organized interface - no configuration required!

### Why Jarvis?

- 🚀 **Zero Configuration** - Auto-discovers all available functions from bash scripts
- 🎨 **Beautiful TUI** - Modern, responsive interface built with Rust and Ratatui
- ⚡ **Real-Time Feedback** - Live stdout/stderr capture during command execution
- 📦 **Single Binary** - Compile once, run anywhere with zero dependencies
- 🔄 **Flexible** - Works with ANY bash scripts AND npm scripts - automation, deployment, utilities, or custom workflows
- 🎯 **Universal** - Not limited to any specific use case - bring your own scripts!
- 📦 **Multi-Language** - Supports both bash functions and npm package.json scripts

---

## ✨ Features

### 🖥️ **Beautiful Terminal Interface**
- **Modern TUI** powered by Rust + Ratatui for smooth, responsive interactions
- **Category-based navigation** with intuitive keyboard shortcuts
- **Real-time output streaming** shows live command execution
- **Smart search** to quickly find any function

### 🔍 **Auto-Discovery System**
- Automatically scans and parses bash scripts AND npm package.json files in your current directory and optional subdirectories
- Discovers `.sh` files in: `./` (root), `./script/`, `./scripts/`, and `./jarvis/`
- Discovers `package.json` files with scripts in the same directories
- Discovers all bash function definitions and npm scripts without manual configuration
- Auto-generates display names from function names (e.g., `my_function` → "My Function", `build:prod` → "Build Prod")
- Organizes functions by script filename as category
- Dynamic menu generation on every launch
- Place scripts anywhere: root directory or in `script/`, `scripts/`, or `jarvis/` folders

### 🎯 **Universal Script Management**

Jarvis works with **any** bash scripts AND npm scripts you provide! Simply place your `.sh` files or `package.json` files in your project directory.

**What Jarvis Discovers:**
- **Bash Scripts** (`.sh` files) - All bash function definitions
- **npm Scripts** (`package.json` files) - All scripts defined in the "scripts" section

**Where to place scripts:**
- **`./` (root directory)** - Scripts in your current directory are discovered
- **`./script/` or `./scripts/`** - Optional subdirectories for organization
- **`./jarvis/`** - Optional directory for Jarvis-specific scripts (can be gitignored)

**Example Use Cases:**
- 🖥️ **System Administration** - OS setup, package management, configuration
- 🏗️ **DevOps & Infrastructure** - Kubernetes, Docker, cloud deployments
- 🔧 **Development Workflows** - Build scripts, testing, deployment pipelines
- 📦 **JavaScript/Node.js Projects** - npm scripts, build processes, testing suites
- 🏠 **Homelab Management** - Server maintenance, backups, monitoring
- 📊 **Data Processing** - ETL scripts, data analysis, reporting
- 🔐 **Security Tools** - Scanning, auditing, compliance checks
- 🎨 **Custom Automation** - Whatever workflow you need to automate!

Jarvis doesn't care what your scripts do - it just makes them easy to discover, organize, and execute.

**Note:** This repository uses `example/` as the test directory. Run `jarvis -p example` to test in this repo.

---

## 🚀 Installation

### Prerequisites

- **Option 1 (Recommended): Nix/Devbox** - Reproducible environment
  - Install [Nix](https://nixos.org/download.html) or [Devbox](https://www.jetify.com/devbox/docs/installing_devbox/)
  - All dependencies managed automatically!
- **Option 2: Manual Setup**
  - Rust 1.70+ ([Install Rust](https://rustup.rs/))
  - Cargo (included with Rust)
  - `bash` 4.0+ (for executing scripts)

### Quick Install with Nix Flakes

```bash
# Run directly (no installation needed!)
nix run github:Luckystrike561/jarvis

# Install to your profile
nix profile install github:Luckystrike561/jarvis

# Try it in a shell
nix shell github:Luckystrike561/jarvis
```

### Quick Start with Devbox (Recommended for Development)

```bash
# Clone the repository
git clone https://github.com/Luckystrike561/jarvis.git
cd jarvis

# Start devbox shell (installs all dependencies automatically)
devbox shell

# Build and run
devbox run dev

# Or build release binary
devbox run release
```

**Available Devbox Commands:**
- `devbox run build` - Build the project
- `devbox run dev` - Build and run Jarvis
- `devbox run check` - Run clippy and format check
- `devbox run fmt` - Format Rust and bash code
- `devbox run lint` - Lint bash scripts with shellcheck
- `devbox run test` - Run all tests
- `devbox run release` - Build optimized release binary

### Quick Start (Manual)

```bash
# Clone the repository
git clone https://github.com/Luckystrike561/jarvis.git
cd jarvis

# Build the optimized binary
cargo build --release

# Run Jarvis (it will discover scripts in current directory)
./target/release/jarvis
```

### System-Wide Installation

```bash
# With Nix (recommended)
nix profile install github:Luckystrike561/jarvis

# With devbox
devbox run release
sudo cp target/release/jarvis /usr/local/bin/

# Or manually
cargo build --release
sudo cp target/release/jarvis /usr/local/bin/

# Run from anywhere
jarvis                    # Searches current directory and optional subdirectories
jarvis --path ~/projects  # Searches in specified directory
```

**Note:** Jarvis looks for `.sh` script files and `package.json` files in your current working directory and optional subdirectories (`script/`, `scripts/`, `jarvis/`) by default, or in the directory specified with `--path`. For this repository, use `jarvis -p example` to test the example scripts.

---

## 📖 Usage

### Running Jarvis

```bash
# From the jarvis directory (uses current directory)
./target/release/jarvis

# Specify a custom directory to search for scripts
./target/release/jarvis --path /path/to/project

# Or if installed system-wide
jarvis                      # Uses current directory
jarvis --path ~/projects    # Uses specified directory
jarvis -p /opt/scripts      # Short form
```

### Keyboard Navigation

- `↑`/`↓` or `j`/`k` - Navigate menu items
- `Enter` - Select category or execute function
- `Backspace`/`Esc` - Go back to previous menu
- `q` - Quit application

### Workflow

1. **Launch Jarvis** - It auto-discovers all bash scripts and npm scripts in your directory
2. **Select Category** - Choose from automatically detected categories
3. **Select Function/Script** - Pick the function or npm script you want to execute
4. **Execute** - Jarvis exits TUI mode and runs your script with full terminal access
5. **Return** - Press Enter after execution to return to Jarvis

---

## 🏗️ Architecture

### Project Structure

```
jarvis/
├── src/                    # Rust TUI source code
│   ├── main.rs            # Application entry point
│   ├── script/            # Script discovery, parsing, and execution
│   │   ├── discovery.rs   # Finds bash scripts and package.json files automatically
│   │   ├── parser.rs      # Parses bash function definitions
│   │   ├── npm_parser.rs  # Parses package.json files
│   │   ├── executor.rs    # Executes scripts with full terminal access
│   │   └── mod.rs         # Module exports
│   └── ui/                # TUI rendering and state management
│       ├── app.rs         # Application state and event handling
│       ├── render.rs      # UI rendering logic
│       └── mod.rs         # Module exports
├── example/               # Example scripts for testing this repo
│   ├── scripts/          # Repository bash scripts (tracked in git)
│   │   └── (add your .sh files)
│   └── jarvis/           # User-specific custom scripts (gitignored)
│       └── (your personal .sh files)
├── Cargo.toml            # Rust project dependencies
├── README.md             # This file
└── LICENSE               # MIT License
```

### Technology Stack

- **Language**: Rust 🦀
- **TUI Framework**: [Ratatui](https://github.com/ratatui-org/ratatui) v0.28
- **Terminal Control**: [Crossterm](https://github.com/crossterm-rs/crossterm) v0.28
- **Async Runtime**: [Tokio](https://tokio.rs/) (full features)
- **Script Parsing**: Regex + custom parser
- **JSON Parsing**: [Serde](https://serde.rs/) + [serde_json](https://github.com/serde-rs/json) (for package.json)
- **Error Handling**: [Anyhow](https://github.com/dtolnay/anyhow)

---

## 🔧 Adding Your Own Scripts

Jarvis automatically discovers bash functions and npm scripts from your project. Just define them and they'll appear in the TUI!

### Option 1: Bash Scripts

Create a `.sh` file in your project directory (or in optional `script/`, `scripts/`, or `jarvis/` subdirectories):

```bash
# myproject.sh  (place in current directory or subdirectories)
#!/usr/bin/env bash

# All functions are automatically discovered by Jarvis
# Function names are formatted for display (e.g., deploy_app -> "Deploy App")

deploy_app() {
    echo "🚀 Deploying application..."
    # Your deployment logic here
}

run_tests() {
    echo "🧪 Running test suite..."
    # Your test commands here
}

backup_db() {
    echo "💾 Backing up database..."
    # Your backup logic here
}
```

### Option 2: npm Scripts

Create a `package.json` file in your project directory (or in optional `script/`, `scripts/`, or `jarvis/` subdirectories):

```json
{
  "name": "my-project",
  "version": "1.0.0",
  "scripts": {
    "build": "npm run build:app",
    "build:prod": "NODE_ENV=production webpack --mode production",
    "test": "jest --coverage",
    "test:watch": "jest --watch",
    "deploy": "npm run build:prod && ./deploy.sh",
    "lint": "eslint src/",
    "dev": "webpack-dev-server --mode development"
  }
}
```

All scripts in the `"scripts"` section will be automatically discovered by Jarvis!

### How It Works

### How It Works

Your scripts will automatically appear in the TUI:
- **Bash scripts**: Functions appear under a category based on the script filename (e.g., `myproject.sh` → "Myproject" category)
- **npm scripts**: Scripts appear under a "Package" category (e.g., `package.json` → "Package" category)

### Category Mapping

Jarvis automatically creates categories based on script filenames:

**Bash Scripts:**

| Filename | Category Display |
|----------|------------------|
| `example_file.sh` | Example File |
| `my_scripts.sh` | My Scripts |
| `homelab-setup.sh` | Homelab Setup |

**npm Scripts:**

| Filename | Category Display |
|----------|------------------|
| `package.json` | Package |

Display names are auto-generated with proper capitalization and spacing.

### Script Format Requirements

**Bash Scripts:**
1. **Bash Functions**: Simply define bash functions in your `.sh` files
2. **Shell**: Must be valid bash scripts with `.sh` extension
3. **Function Names**: Use valid bash identifiers (letters, numbers, underscores)

**npm Scripts:**
1. **package.json**: Must be valid JSON with a `"scripts"` section
2. **Script Names**: Can use any valid npm script naming (letters, numbers, colons, hyphens)
3. **Commands**: Any valid shell command or npm command

### Customizing Function Display

You can customize how functions appear in Jarvis using special comment annotations above your function definitions:

```bash
#!/usr/bin/env bash

# @emoji 🚀
# @description Deploy the application to production environment
deploy_to_production() {
    echo "Deploying to production..."
    # Your deployment logic here
}

# @description Run the full test suite with coverage reports
run_tests() {
    echo "Running tests..."
    # Your test commands here
}

# @emoji 💾
backup_database() {
    echo "Backing up database..."
    # Your backup logic here
}

# @ignore
format_string() {
    # Utility function - hidden from TUI
    echo "$1" | tr '[:lower:]' '[:upper:]'
}
```

**Available Annotations:**

- **`@emoji`** - Add an emoji prefix that appears before the function name in the TUI
- **`@description`** - Provide a custom description shown in the details panel (replaces the default "Execute: function_name")
- **`@ignore`** - Hide utility/helper functions from the TUI (useful for internal functions not meant to be called directly)

**Annotation Rules:**

- Place annotations in comments directly above the function definition
- Annotations must be on consecutive comment lines (no blank lines between them and the function)
- You can use multiple annotations together, or just one
- If no annotations are provided, Jarvis uses auto-generated defaults
- Functions marked with `@ignore` are parsed but filtered out from the TUI display

**Example Output in TUI:**

Without annotations:
```
  Deploy To Production
```

With emoji:
```
  🚀 Deploy To Production
```

See `example/jarvis/annotations_demo.sh` for a complete demonstration of all annotation combinations.

### Tips

- **Bash scripts**: Use `snake_case` for function names (e.g., `deploy_production`, `backup_database`)
- **npm scripts**: Use descriptive names with colons for namespacing (e.g., `build:prod`, `test:unit`)
- Function names are automatically formatted: `my_cool_function` becomes "My Cool Function"
- npm script names are formatted: `build:prod` becomes "Build Prod"
- Add emoji annotations to make functions visually distinctive in the TUI
- Use custom descriptions to provide clear context about what each function does
- Use emoji prefixes in echo output for visual feedback (✅ ❌ 🚀 🔧 📦)
- Group related functionality in the same script file
- Scripts can use interactive tools like `gum`, `fzf`, `dialog`, etc.

---

## 🎯 Example Use Case: Shield Homelab

Jarvis was originally built for the **Shield** homelab ecosystem, which demonstrates its capabilities:

- **System Management** (`fedora.sh`) - Complete laptop setup, package installation, dotfiles sync
- **Homelab Operations** (`homelab.sh`) - K3S cluster management, ArgoCD, Kubernetes resources
- **Utilities** (`util.sh`) - S.M.A.R.T. diagnostics, VPN management, system monitoring

But Jarvis is **not limited** to this use case - it's a general-purpose tool that works with any bash scripts you throw at it!

---

## 🛠️ Development

### Building from Source

```bash
# Install Rust toolchain
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Clone and build
git clone https://github.com/yourusername/jarvis.git
cd jarvis
cargo build --release

# Binary location: target/release/jarvis
```

### Development Mode

```bash
# Run with hot reload
cargo watch -x run

# Run tests
cargo test

# Check for issues
cargo clippy

# Format code
cargo fmt
```

### Dependencies

```toml
[dependencies]
ratatui = "0.28"      # Terminal UI framework
crossterm = "0.28"    # Terminal control
tokio = "1"           # Async runtime (full features)
anyhow = "1"          # Error handling
regex = "1"           # Pattern matching
walkdir = "2"         # Directory traversal
serde = "1"           # Serialization framework
serde_json = "1"      # JSON parsing for package.json
```

---

## 🤝 Contributing

Contributions are welcome! Here's how you can help:

1. **Report Bugs** - Open an issue with details
2. **Suggest Features** - Propose enhancements to the TUI or script discovery
3. **Submit PRs** - Follow the coding conventions below
4. **Improve Documentation** - Help others understand Jarvis better
5. **Share Examples** - Show off interesting ways you're using Jarvis!

### Coding Style

**Rust Code:**
- Follow `rustfmt` conventions (run `cargo fmt`)
- Use `clippy` for linting (run `cargo clippy`)
- Write descriptive error messages
- Keep functions focused and modular

**Bash Scripts:**
- Use `#!/usr/bin/env bash` shebang
- `snake_case` for functions and variables
- Clear, descriptive function and display names
- Test your scripts before committing

---

## 📚 Documentation

- [JARVIS-TUI-DESIGN.md](JARVIS-TUI-DESIGN.md) - TUI architecture and design decisions
- [TESTING.md](TESTING.md) - Testing procedures and guidelines
- [AGENTS.md](AGENTS.md) - Original development guide (for Shield ecosystem context)

---

## 📝 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

---

## 🙏 Acknowledgments

- Inspired by **Tony Stark's Jarvis** from Marvel's Iron Man
- Built with [Ratatui](https://github.com/ratatui-org/ratatui) - Excellent TUI framework for Rust
- Influenced by [linutil](https://github.com/ChrisTitusTech/linutil) - Chris Titus Tech's Linux utility
- Originally created for the **Shield** homelab project

---

<div align="center">

**[⬆ back to top](#-jarvis)**

Made with ❤️ for the homelab community

</div>
