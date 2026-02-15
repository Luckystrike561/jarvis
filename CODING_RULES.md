# Coding Rules

This document defines the coding standards enforced in the Jarvis codebase. These rules are
partially automated via tooling (`rustfmt`, `clippy`, `shellcheck`, `shfmt`, `editorconfig`) and
partially convention-based.

## Tooling Quick Reference

```bash
cargo fmt              # Format Rust code
cargo clippy           # Lint Rust code (must pass with zero warnings)
devbox run check       # Run both clippy + format check
devbox run lint        # Lint bash scripts with shellcheck
devbox run fmt         # Format everything (Rust + bash)
cargo test             # Run all tests
```

## Rust

### Formatting (rustfmt.toml)

- **Max line width**: 100 characters
- **Indentation**: 4 spaces, no tabs
- **Field init shorthand**: `Point { x, y }` not `Point { x: x, y: y }`
- **Try shorthand**: use `?` operator
- Run `cargo fmt` before committing. CI will reject unformatted code.

### Linting (Cargo.toml [lints])

The following lint rules are enforced in `Cargo.toml` under `[lints]`:

| Lint | Level | Rationale |
|------|-------|-----------|
| `unsafe_code` | **forbid** | No unsafe code anywhere in the codebase |
| `clippy::unwrap_used` | **deny** | Use `?`, `.with_context()`, or pattern matching instead |
| `clippy::redundant_closure_for_method_calls` | warn | Use method references: `.map(str::trim_end)` not `.map(\|l\| l.trim_end())` |
| `clippy::inefficient_to_string` | warn | Prefer `(*s).to_string()` over `s.to_string()` on `&&str` |
| `clippy::cloned_instead_of_copied` | warn | Use `.copied()` for `Copy` types |
| `clippy::needless_pass_by_value` | warn | Pass by reference when ownership isn't needed |
| `clippy::implicit_clone` | warn | Use explicit `.clone()` or `.to_owned()` |
| `clippy::semicolon_if_nothing_returned` | warn | Add semicolons to expressions that return `()` |
| `clippy::manual_string_new` | warn | Use `String::new()` not `"".to_string()` |
| `clippy::doc_markdown` | warn | Backtick-wrap code identifiers in doc comments |

### Naming

- `snake_case` for functions and variables
- `PascalCase` for types (structs, enums, traits)
- `SCREAMING_SNAKE_CASE` for constants
- Private helper functions: no prefix convention, just omit `pub`

### Error Handling

- Return `anyhow::Result<T>` from fallible functions
- Use `?` operator for propagation
- Use `.with_context(|| "descriptive message")` for adding context to errors
- **Never** use `.unwrap()` in production code (enforced by clippy deny)
- In tests, `.unwrap()` is acceptable

### Imports

- Group imports: std, external crates, internal modules (separated by blank lines in `main.rs`)
- Use explicit imports, not wildcards (`use super::*` is acceptable in test modules only)
- Prefer `use crate::module::Type` over `use crate::module::*`

### Documentation

- Add `///` doc comments on public API functions
- Wrap code identifiers in backticks in doc comments: `` `ScriptFile` `` not `ScriptFile`
- Module-level docs use `//!` at the top of the file

### Testing

- Place unit tests in a `#[cfg(test)] mod tests` block at the bottom of each source file
- Use `use super::*;` in test modules
- Test function names: `test_<function_name>_<scenario>` (e.g., `test_shell_escape_empty_string`)
- Use `tempfile::TempDir` for filesystem tests, never write to real paths
- All tests must pass: `cargo test`

### Architecture

- **No `unsafe` code** (enforced by `#[forbid(unsafe_code)]`)
- **No dead code** -- remove unused functions, fields, and modules promptly
- `pub` for public API; private by default for internal helpers. `pub(crate)` is not used.
- `#[allow(dead_code)]` is acceptable only on struct fields required for deserialization

## Bash Scripts

### Formatting and Linting

- **Formatter**: `shfmt` (run `devbox run fmt`)
- **Linter**: `shellcheck` (run `devbox run lint`)
- **Shebang**: always `#!/usr/bin/env bash`
- **Indentation**: 4 spaces

### Naming

- `snake_case` for functions and variables
- `ALL_CAPS` for constants and exported variables

### Function Annotations

Jarvis supports special comment annotations above function definitions:

```bash
# @emoji <emoji>         — Display emoji prefix in the TUI
# @description <text>    — Custom description in the details panel
# @ignore                — Hide helper functions from the TUI
```

Annotations must be in consecutive comment lines directly above the function definition.

## Editor Configuration (.editorconfig)

The `.editorconfig` file ensures consistent settings across editors:

- UTF-8 charset, LF line endings
- 4-space indentation for `.rs`, `.sh`
- 2-space indentation for `.yml`, `.yaml`, `.json`, `.toml`
- Tab indentation for `Makefile`
- Trailing whitespace trimmed (except in `.md`)
- Final newline inserted

## Commits

Follow [Conventional Commits](https://www.conventionalcommits.org/):

```
feat: add fuzzy search to function list
fix: handle missing scripts directory gracefully
docs: update installation instructions
refactor: extract command building into separate function
perf: cache parsed script results
test: add unit tests for shell escaping
```

## CI Enforcement

The following checks run in CI and must all pass:

1. `cargo fmt --check` -- formatting
2. `cargo clippy` -- linting (zero warnings)
3. `cargo test` -- all tests pass
4. `shellcheck` -- bash script linting
