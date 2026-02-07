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

**Function Annotations:** Customize function/task display in the TUI using special comment annotations. Supported in bash scripts, Taskfile.yml, and Makefile:
- `@emoji <emoji>` - Add an emoji prefix before the function name (e.g., `# @emoji 🚀`)
- `@description <text>` - Provide a custom description for the details panel (e.g., `# @description Deploy to production`)
- `@ignore` - Hide utility/helper functions from the TUI (e.g., `# @ignore`)
- Place annotations in comments directly above the function/task definition (consecutive comment lines only)
- Bash example:
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
- Taskfile example:
  ```yaml
  tasks:
    # @emoji 🚀
    # @description Deploy the application to production
    deploy:
      cmds:
        - ./deploy.sh

    # @ignore
    _internal_helper:
      cmds:
        - echo "helper"
  ```
- Makefile example:
  ```makefile
  # @emoji 🚀
  # @description Deploy the application to production
  deploy:
  	./deploy.sh

  # @ignore
  _internal_helper:
  	@echo "helper"
  ```

**Imports:** Group std > external crates > internal modules, separated by blank lines. Use explicit imports over wildcards.

**Core Principles:** Zero configuration (auto-discover scripts), universal design (works with ANY bash scripts), full terminal access (inherit stdio), beautiful UX.

**Commits:** Use Conventional Commits format: `feat:`, `fix:`, `docs:`, `refactor:`, `perf:`, `test:`. Examples: `feat: add fuzzy search`, `fix: handle multi-line arrays`.

## Key Patterns
- Script discovery: Jarvis scans current directory (`./`) and optional subdirectories (`./script/`, `./scripts/`, `./jarvis/`) for `.sh` files, `package.json`, `devbox.json`, `Taskfile.yml`, and `Makefile`, auto-detects all bash functions, npm scripts, devbox scripts, task targets, and make targets. For this repo, use `jarvis -p example` to test.
- Function naming: `my_function` becomes "My Function" in the UI
- Execution: always use `.stdin(Stdio::inherit()).stdout(Stdio::inherit()).stderr(Stdio::inherit())` 
- TUI states: `MainMenu` → `CategoryView` → execute → return
- Category mapping: customize in `src/script/discovery.rs`
- Discovery depth: root directory uses depth 1, subdirectories use depth 2
