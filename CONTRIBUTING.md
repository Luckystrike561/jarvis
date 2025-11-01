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
git clone https://github.com/yourusername/jarvis.git
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

### Rust

- **Format**: Use `rustfmt` (run `devbox run fmt` or `cargo fmt`)
- **Lint**: Pass `clippy` with no warnings (run `devbox run check`)
- **Naming**: 
  - `snake_case` for functions and variables
  - `PascalCase` for structs and enums
  - `SCREAMING_SNAKE_CASE` for constants
- **Error Handling**: Use `anyhow::Result<T>`, avoid `.unwrap()` in production code
- **Documentation**: Add doc comments for public APIs

Example:
```rust
/// Execute a bash function interactively
///
/// # Arguments
/// * `script_path` - Path to the bash script
/// * `function_name` - Name of the function to execute
pub fn execute_function(script_path: &Path, function_name: &str) -> anyhow::Result<()> {
    // Implementation
}
```

### Bash Scripts

- **Format**: Use `shfmt` (run `devbox run fmt`)
- **Lint**: Pass `shellcheck` (run `devbox run lint`)
- **Shebang**: Always use `#!/usr/bin/env bash`
- **Naming**:
  - `snake_case` for functions and variables
  - `ALL_CAPS` for constants and arrays
- **Documentation**: Add comments for complex logic

Example:
```bash
#!/usr/bin/env bash

# Example script showing Jarvis-compatible functions

# Function array for menu discovery
EXAMPLE_FUNCTIONS=(
    "Hello World:hello_world"
    "System Info:system_info"
)

hello_world() {
    echo "Hello from Jarvis!"
}

system_info() {
    echo "OS: $(uname -s)"
    echo "Kernel: $(uname -r)"
}
```

## 🏗️ Project Structure

```
jarvis/
├── src/
│   ├── main.rs           # Entry point
│   ├── ui/               # TUI components
│   │   ├── mod.rs
│   │   ├── app.rs        # Main app state and logic
│   │   └── render.rs     # UI rendering
│   └── script/           # Script discovery and execution
│       ├── mod.rs
│       ├── parser.rs     # Bash script parsing
│       ├── discovery.rs  # Script file discovery
│       └── executor.rs   # Script execution
├── scripts/              # Example bash scripts
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
# Run all tests
devbox run test

# Run specific test
cargo test test_name

# Run tests with output
cargo test -- --nocapture
```

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

- Open a [Discussion](https://github.com/yourusername/jarvis/discussions)
- Join our community chat (if available)
- Comment on relevant issues

Thank you for contributing to Jarvis! 🤖
