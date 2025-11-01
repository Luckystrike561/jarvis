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

Inspired by Tony Stark's legendary AI from Marvel, **Jarvis** is a powerful TUI (Terminal User Interface) application for managing and executing bash scripts. It automatically discovers your bash functions and presents them in a beautiful, organized interface - no configuration required!

### Why Jarvis?

- 🚀 **Zero Configuration** - Auto-discovers all available functions from bash scripts
- 🎨 **Beautiful TUI** - Modern, responsive interface built with Rust and Ratatui
- ⚡ **Real-Time Feedback** - Live stdout/stderr capture during command execution
- 📦 **Single Binary** - Compile once, run anywhere with zero dependencies
- 🔄 **Flexible** - Works with ANY bash scripts - automation, deployment, utilities, or custom workflows
- 🎯 **Universal** - Not limited to any specific use case - bring your own scripts!

---

## ✨ Features

### 🖥️ **Beautiful Terminal Interface**
- **Modern TUI** powered by Rust + Ratatui for smooth, responsive interactions
- **Category-based navigation** with intuitive keyboard shortcuts
- **Real-time output streaming** shows live command execution
- **Smart search** to quickly find any function

### 🔍 **Auto-Discovery System**
- Automatically scans and parses bash scripts in `scripts/` directory
- Discovers function definitions without manual configuration
- Organizes functions by category (System, Homelab, Utilities)
- Dynamic menu generation on every launch

### 🎯 **Universal Script Management**

Jarvis works with **any** bash scripts you provide! Simply place your `.sh` files in the `scripts/` directory and define function arrays.

**Example Use Cases:**
- 🖥️ **System Administration** - OS setup, package management, configuration
- 🏗️ **DevOps & Infrastructure** - Kubernetes, Docker, cloud deployments
- 🔧 **Development Workflows** - Build scripts, testing, deployment pipelines  
- 🏠 **Homelab Management** - Server maintenance, backups, monitoring
- 📊 **Data Processing** - ETL scripts, data analysis, reporting
- 🔐 **Security Tools** - Scanning, auditing, compliance checks
- 🎨 **Custom Automation** - Whatever workflow you need to automate!

Jarvis doesn't care what your scripts do - it just makes them easy to discover, organize, and execute.

---

## 🚀 Installation

### Prerequisites

- **Option 1 (Recommended): Devbox** - Reproducible dev environment
  - Install [Devbox](https://www.jetify.com/devbox/docs/installing_devbox/)
  - All dependencies managed automatically!
- **Option 2: Manual Setup**
  - Rust 1.70+ ([Install Rust](https://rustup.rs/))
  - Cargo (included with Rust)
  - `bash` 4.0+ (for executing scripts)

### Quick Start with Devbox (Recommended)

```bash
# Clone the repository
git clone https://github.com/yourusername/jarvis.git
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
git clone https://github.com/yourusername/jarvis.git
cd jarvis

# Build the optimized binary
cargo build --release

# Create a scripts directory (if it doesn't exist)
mkdir -p scripts

# Add your bash scripts to the scripts/ directory
# See "Adding Scripts" section below

# Run Jarvis
./target/release/jarvis
```

### System-Wide Installation

```bash
# With devbox
devbox run release
sudo cp target/release/jarvis /usr/local/bin/

# Or manually
cargo build --release
sudo cp target/release/jarvis /usr/local/bin/

# Run from anywhere (ensure you run from a directory with a scripts/ folder)
jarvis
```

**Note:** Jarvis looks for a `scripts/` directory in your current working directory. Make sure to run it from a directory containing your scripts.

---

## 📖 Usage

### Running Jarvis

```bash
# From the jarvis directory
./target/release/jarvis

# Or if installed system-wide (must be in a directory with scripts/)
jarvis
```

### Keyboard Navigation

- `↑`/`↓` or `j`/`k` - Navigate menu items
- `Enter` - Select category or execute function
- `Backspace`/`Esc` - Go back to previous menu
- `q` - Quit application

### Workflow

1. **Launch Jarvis** - It auto-discovers all bash scripts in `scripts/`
2. **Select Category** - Choose from automatically detected categories
3. **Select Function** - Pick the function you want to execute
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
│   │   ├── discovery.rs   # Finds bash scripts automatically
│   │   ├── parser.rs      # Parses function definitions
│   │   ├── executor.rs    # Executes scripts with full terminal access
│   │   └── mod.rs         # Module exports
│   └── ui/                # TUI rendering and state management
│       ├── app.rs         # Application state and event handling
│       ├── render.rs      # UI rendering logic
│       └── mod.rs         # Module exports
├── scripts/               # YOUR bash scripts go here
│   └── (add your .sh files)
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
- **Error Handling**: [Anyhow](https://github.com/dtolnay/anyhow)

---

## 🔧 Adding Your Own Scripts

Jarvis automatically discovers functions from your bash scripts using a simple convention.

### Step 1: Create a Script File

Create a `.sh` file in the `scripts/` directory:

```bash
# scripts/myproject.sh
#!/usr/bin/env bash

# Define a function array with format: "Display Name:function_name"
myproject_functions=(
    "Deploy Application:deploy_app"
    "Run Tests:run_tests"
    "Backup Database:backup_db"
)

# Implement your functions
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

### Step 2: Run Jarvis

That's it! Your functions will automatically appear in the TUI under a new category based on your script filename.

### Category Mapping

Jarvis automatically categorizes scripts based on filename:

| Filename | Category |
|----------|----------|
| `fedora.sh` | System Management |
| `homelab.sh` | Homelab Operations |
| `util.sh` | Utilities |
| `anything.sh` | Other |

You can customize category mapping in `src/script/discovery.rs`.

### Script Format Requirements

1. **Function Array**: Define `<name>_functions=()` array with entries in format `"Display:function"`
2. **Bash Functions**: Implement the functions referenced in your array
3. **Shell**: Must be valid bash scripts with `.sh` extension

### Tips

- Use `snake_case` for function names
- Provide clear, descriptive display names
- Use emoji prefixes for visual feedback (✅ ❌ 🚀 🔧 📦)
- Group related functionality in the same script
- Scripts can use interactive tools like `gum`, `fzf`, `dialog`, etc.

---

## 🎯 Example Use Case: Shield Homelab

Jarvis was originally built for the **Shield** homelab ecosystem, which demonstrates its capabilities:

- **System Management** (`scripts/fedora.sh`) - Complete laptop setup, package installation, dotfiles sync
- **Homelab Operations** (`scripts/homelab.sh`) - K3S cluster management, ArgoCD, Kubernetes resources
- **Utilities** (`scripts/util.sh`) - S.M.A.R.T. diagnostics, VPN management, system monitoring

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
