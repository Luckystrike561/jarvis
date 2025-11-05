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

## Code Style Guidelines

**Rust:** Use `rustfmt` defaults. Naming: `snake_case` (functions/vars), `PascalCase` (structs/enums), `SCREAMING_SNAKE_CASE` (constants). Error handling: prefer `anyhow::Result<T>` with `?` operator, avoid `.unwrap()` in production. Add doc comments (`///`) for public APIs. Use `.with_context()` for descriptive errors.

**Bash:** Shebang: `#!/usr/bin/env bash`. Naming: `snake_case` (functions/vars). Define arrays: `category_functions=("Display:function" ...)`. Format with `shfmt`, lint with `shellcheck`.

**Imports:** Group std > external crates > internal modules, separated by blank lines. Use explicit imports over wildcards.

**Core Principles:** Zero configuration (auto-discover scripts), universal design (works with ANY bash scripts), full terminal access (inherit stdio), beautiful UX.

**Commits:** Use Conventional Commits format: `feat:`, `fix:`, `docs:`, `refactor:`, `perf:`, `test:`. Examples: `feat: add fuzzy search`, `fix: handle multi-line arrays`.

## Key Patterns
- Script discovery: scan `scripts/` for `.sh` files, parse `*_functions` arrays
- Execution: always use `.stdin(Stdio::inherit()).stdout(Stdio::inherit()).stderr(Stdio::inherit())` 
- TUI states: `MainMenu` → `CategoryView` → execute → return
- Category mapping: customize in `src/script/discovery.rs`
