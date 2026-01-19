<div align="center">

# Jarvis

**Just Another Rather Very Intelligent System**

A beautiful TUI for managing and executing scripts with zero configuration.

[![CI](https://github.com/Luckystrike561/jarvis/actions/workflows/build.yml/badge.svg)](https://github.com/Luckystrike561/jarvis/actions/workflows/build.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/Rust-1.70+-orange.svg)](https://www.rust-lang.org/)

<!-- Add a screenshot after capturing one -->
<!-- ![Jarvis Screenshot](assets/screenshot.png) -->

[Features](#features) • [Installation](#installation) • [Usage](#usage) • [Contributing](#contributing)

</div>

---

## Features

- **Zero Configuration** - Auto-discovers bash functions, npm scripts, and devbox scripts
- **Beautiful TUI** - Modern terminal interface built with Ratatui
- **Multi-Language Support** - Works with `.sh` files, `package.json`, and `devbox.json`
- **Single Binary** - Compile once, run anywhere

## Installation

### Homebrew (macOS & Linux)

```bash
brew install https://raw.githubusercontent.com/Luckystrike561/jarvis/main/homebrew/jarvis.rb
```

### Nix Flakes

```bash
nix run github:Luckystrike561/jarvis
```

### Build from Source

```bash
git clone https://github.com/Luckystrike561/jarvis.git
cd jarvis
cargo build --release
sudo cp target/release/jarvis /usr/local/bin/
```

## Usage

```bash
# Run in current directory
jarvis

# Run in a specific directory
jarvis --path /path/to/project
```

### Keyboard Shortcuts

| Key | Action |
|-----|--------|
| `j/k` or `↑/↓` | Navigate |
| `h/l` or `←/→` | Collapse/Expand |
| `Enter` | Select/Execute |
| `/` | Search |
| `Tab` | Switch panes |
| `q` | Quit |

## Script Discovery

Jarvis automatically discovers scripts in these locations:

- `./` - Current directory
- `./script/` or `./scripts/` - Script subdirectories  
- `./jarvis/` - Jarvis-specific scripts

### Supported Script Types

**Bash Functions** - Any `.sh` file with function definitions:

```bash
# deploy.sh
deploy_app() {
    echo "Deploying..."
}
```

**npm Scripts** - From `package.json`:

```json
{
  "scripts": {
    "build": "npm run build:app",
    "test": "jest"
  }
}
```

### Function Annotations

Customize how functions appear in the TUI:

```bash
# @emoji 🚀
# @description Deploy to production
deploy_production() {
    echo "Deploying..."
}

# @ignore
_helper_function() {
    # Hidden from TUI
}
```

## Development

```bash
# Using Devbox (recommended)
devbox shell
devbox run dev      # Build and run
devbox run test     # Run tests
devbox run check    # Lint + format check

# Manual
cargo build
cargo test
cargo clippy
```

## Contributing

Contributions are welcome! Please see [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

## License

MIT License - see [LICENSE](LICENSE) for details.

---

<div align="center">

Built with [Ratatui](https://github.com/ratatui-org/ratatui)

</div>
