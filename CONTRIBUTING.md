# Contributing to Jarvis

Thank you for your interest in contributing to Jarvis! This guide will help you set up your development environment and understand the project structure.

## 🚀 Getting Started

### Prerequisites

We use **Devbox** for reproducible development environments. This ensures all contributors have the same toolchain and dependencies.

1. **Install Devbox** - [Installation Guide](https://www.jetify.com/devbox/docs/installing_devbox/)
2. **Optional: Install direnv** - For automatic environment loading: [direnv](https://direnv.net/)

### Setting Up Your Development Environment

```bash
# Clone the repository
git clone https://github.com/Luckystrike561/jarvis.git
cd jarvis

# Start devbox shell (installs all dependencies)
devbox shell

# You should see: "🤖 Jarvis development environment loaded"
```

If you have `direnv` installed:
```bash
# Allow direnv to auto-load devbox
direnv allow

# Now the environment loads automatically when you cd into the directory!
```

## 🛠️ Development Workflow

### Available Commands

Devbox provides convenient commands for common tasks:

```bash
# Build the project
devbox run build

# Build and run Jarvis
devbox run dev

# Run code quality checks (clippy + format)
devbox run check

# Format all code (Rust + bash scripts)
devbox run fmt

# Lint bash scripts with shellcheck
devbox run lint

# Run all tests
devbox run test

# Build optimized release binary
devbox run release
```

### Manual Commands (if needed)

```bash
# Rust commands
cargo build                 # Debug build
cargo build --release       # Release build
cargo test                  # Run tests
cargo clippy                # Linting
cargo fmt                   # Format code

# Bash linting/formatting
shellcheck scripts/*.sh     # Lint bash scripts
shfmt -w scripts/          # Format bash scripts
```

## 📝 Code Style

See [CODING_RULES.md](CODING_RULES.md) for the full coding standards, including Rust lint rules,
naming conventions, error handling patterns, bash script guidelines, and commit format.

**Quick summary:**

- **Format**: `cargo fmt` (enforced by `rustfmt.toml`)
- **Lint**: `cargo clippy` with zero warnings (enforced by `Cargo.toml [lints]`)
- **No `unsafe` code** (forbidden project-wide)
- **No `.unwrap()`** in production code (denied by clippy)
- **Error handling**: `anyhow::Result<T>` with `?` operator and `.with_context()`

## 🏗️ Project Structure

```
jarvis/
├── src/
│   ├── main.rs           # Entry point
│   ├── lib.rs            # Module declarations
│   ├── ui/               # TUI components
│   │   ├── mod.rs
│   │   ├── app.rs            # Main app state and logic
│   │   ├── render.rs         # UI rendering
│   │   ├── pty_runner.rs     # PTY-based script execution
│   │   └── terminal_widget.rs # Terminal output widget
│   └── script/           # Script discovery and parsing
│       ├── mod.rs
│       ├── discovery.rs      # Script file discovery
│       ├── parser.rs         # Bash script parsing
│       ├── npm_parser.rs     # package.json parsing
│       ├── cargo_parser.rs   # Cargo.toml parsing
│       ├── devbox_parser.rs  # devbox.json parsing
│       ├── just_parser.rs    # Justfile parsing
│       ├── makefile_parser.rs # Makefile parsing
│       ├── nx_parser.rs      # Nx workspace parsing
│       ├── task_parser.rs    # Taskfile.yml parsing
│       └── utils.rs          # Shared parser utilities
│   └── usage/            # Usage tracking
│       ├── mod.rs
│       └── storage.rs        # Usage data persistence
├── example/              # Example scripts and test files
│   ├── jarvis/          # Bash script examples
│   ├── node/            # npm/package.json examples
│   └── scripts/         # Additional script examples
├── devbox.json           # Devbox configuration
├── .envrc                # direnv config (optional)
└── Cargo.toml            # Rust dependencies
```

## 🔍 Making Changes

### Before Submitting a PR

1. **Format your code**: `devbox run fmt`
2. **Run checks**: `devbox run check`
3. **Lint bash scripts**: `devbox run lint`
4. **Test your changes**: `devbox run test`
5. **Test manually**: `devbox run dev`

### Pull Request Guidelines

1. **Branch naming**: 
   - `feature/your-feature-name` for new features
   - `fix/bug-description` for bug fixes
   - `docs/what-changed` for documentation
2. **Commit messages**: Follow [Conventional Commits](https://www.conventionalcommits.org/)
   - `feat: add support for nested script directories`
   - `fix: handle missing scripts/ directory gracefully`
   - `docs: update installation instructions`
3. **Description**: Clearly explain what your PR does and why
4. **Tests**: Add tests for new functionality
5. **Documentation**: Update README.md or other docs if needed

## 🧪 Testing

```bash
# Run all 258 tests
devbox run test

# Run specific test
cargo test test_name

# Run tests with output
cargo test -- --nocapture
```

See [TESTING.md](TESTING.md) for comprehensive testing guide including manual TUI tests.

## 🐛 Reporting Issues

When reporting bugs, please include:
- Operating system and version
- Rust version (`rustc --version`)
- Steps to reproduce
- Expected vs actual behavior
- Error messages or logs

## 💡 Feature Requests

We welcome feature requests! Please:
- Check existing issues first
- Clearly describe the use case
- Explain how it fits with Jarvis's universal design philosophy
- Provide examples if possible

## 📜 Code of Conduct

- Be respectful and inclusive
- Focus on constructive feedback
- Help others learn and grow
- Have fun! 🎉

## 🎯 Good First Issues

Look for issues tagged with `good first issue` - these are beginner-friendly tasks that are a great way to get started.

## 📧 Questions?

- Open a [Discussion](https://github.com/Luckystrike561/jarvis/discussions)
- Join our community chat (if available)
- Comment on relevant issues

Thank you for contributing to Jarvis! 🤖
