# Agent Development Guide for Jarvis

## Build/Test/Lint Commands
```bash
cargo build                    # Debug build
cargo build --release          # Production build
cargo test                     # Run all tests
cargo test test_name           # Run single test by name
cargo clippy                   # Lint with clippy (must pass with no warnings)
cargo fmt                      # Format Rust code
devbox run check               # Run clippy + format check
devbox run lint                # Lint bash scripts with shellcheck
```

## Usage
```bash
jarvis                         # Use current directory
jarvis --path /path/to/dir     # Use specified directory
jarvis -p ~/projects           # Short form
```

## Code Style Guidelines

**Rust:** Use `rustfmt` defaults. Naming: `snake_case` (functions/vars), `PascalCase` (structs/enums), `SCREAMING_SNAKE_CASE` (constants). Error handling: prefer `anyhow::Result<T>` with `?` operator, avoid `.unwrap()` in production. Add doc comments (`///`) for public APIs. Use `.with_context()` for descriptive errors.

**Bash:** Shebang: `#!/usr/bin/env bash`. Naming: `snake_case` (functions/vars). All bash functions are automatically discovered - no arrays needed. Format with `shfmt`, lint with `shellcheck`.

**Function Annotations:** Customize function display in the TUI using special comment annotations:
- `@emoji <emoji>` - Add an emoji prefix before the function name (e.g., `# @emoji 🚀`)
- `@description <text>` - Provide a custom description for the details panel (e.g., `# @description Deploy to production`)
- `@ignore` - Hide utility/helper functions from the TUI (e.g., `# @ignore`)
- Place annotations in comments directly above the function definition (consecutive comment lines only)
- Example:
  ```bash
  # @emoji 🚀
  # @description Deploy the application to production
  deploy_app() {
      echo "Deploying..."
  }
  
  # @ignore
  _helper_function() {
      echo "Internal utility - hidden from TUI"
  }
  ```

**Imports:** Group std > external crates > internal modules, separated by blank lines. Use explicit imports over wildcards.

**Core Principles:** Zero configuration (auto-discover scripts), universal design (works with ANY bash scripts), full terminal access (inherit stdio), beautiful UX.

**Commits:** Use Conventional Commits format: `feat:`, `fix:`, `docs:`, `refactor:`, `perf:`, `test:`. Examples: `feat: add fuzzy search`, `fix: handle multi-line arrays`.

## Key Patterns
- Script discovery: Jarvis scans current directory (`./`) and optional subdirectories (`./script/`, `./scripts/`, `./jarvis/`) for `.sh` files, auto-detects all bash functions. For this repo, use `jarvis -p example` to test.
- Function naming: `my_function` becomes "My Function" in the UI
- Execution: always use `.stdin(Stdio::inherit()).stdout(Stdio::inherit()).stderr(Stdio::inherit())` 
- TUI states: `MainMenu` → `CategoryView` → execute → return
- Category mapping: customize in `src/script/discovery.rs`
- Discovery depth: root directory uses depth 1, subdirectories use depth 2
