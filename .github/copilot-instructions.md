# GitHub Copilot Instructions for Jarvis

## Project Overview

Jarvis is a **universal TUI (Terminal User Interface) for managing and executing bash scripts**. It's built with Rust and Ratatui, providing zero-configuration script discovery and execution.

## Development Environment

This project uses **Devbox** for reproducible development environments:

```bash
# Enter devbox shell (auto-installs dependencies)
devbox shell

# Available commands
devbox run build    # Build the project
devbox run dev      # Build and run
devbox run check    # Run clippy and format check
devbox run fmt      # Format code (Rust + bash)
devbox run lint     # Lint bash scripts
devbox run test     # Run tests
devbox run release  # Build optimized binary
```

**Dependencies managed by Devbox:**
- Rust toolchain (cargo, rustc)
- Bash (for script execution)
- shellcheck (bash linting)
- shfmt (bash formatting)
- fzf (for script features)

## Core Principles

1. **Zero Configuration** - Auto-discover scripts, no config files needed
2. **Universal Design** - Works with ANY bash scripts, not domain-specific
3. **Full Terminal Access** - Scripts run with inherited stdin/stdout for interactivity
4. **Beautiful UX** - Modern TUI with intuitive navigation

## Code Style Guidelines

### Rust Code

**Formatting:**
- Use `rustfmt` defaults (run `cargo fmt` before committing)
- Use `clippy` for linting (run `cargo clippy`)
- Maximum line length: 100 characters

**Naming Conventions:**
```rust
// Structs: PascalCase
pub struct ScriptFunction { }

// Enums: PascalCase, variants: PascalCase
pub enum AppState {
    MainMenu,
    CategoryView,
}

// Functions and variables: snake_case
fn execute_function_interactive() { }
let script_path = path.to_path_buf();

// Constants: SCREAMING_SNAKE_CASE
const MAX_RETRIES: u32 = 3;
```

**Error Handling:**
- Use `anyhow::Result<T>` for functions that can fail
- Provide descriptive error messages
- Avoid unwrap() in production code - use `?` operator or proper error handling

**Documentation:**
```rust
/// Execute a bash function interactively with full terminal access
/// 
/// This allows the script to use stdin/stdout/stderr directly for tools like gum, fzf, etc.
/// 
/// # Arguments
/// * `script_path` - Path to the bash script file
/// * `function_name` - Name of the function to execute
/// 
/// # Returns
/// Exit code from the function execution
pub fn execute_function_interactive(
    script_path: &Path,
    function_name: &str,
) -> Result<i32> {
    // implementation
}
```

### Bash Scripts

**Format Requirements:**
```bash
#!/usr/bin/env bash

# Define function array with format: "Display Name:function_name"
example_functions=(
    "Deploy Application:deploy_app"
    "Run Tests:run_tests"
)

# Function implementations
deploy_app() {
    echo "🚀 Deploying application..."
    # Implementation
}
```

**Naming:**
- Functions: `snake_case`
- Arrays: `<category>_functions`
- Variables: `snake_case`
- Constants: `ALL_CAPS`

**Best Practices:**
- Use emoji prefixes for visual feedback (✅ ❌ 🚀 🔧 📦 🧪)
- Check command availability before use
- Provide clear error messages
- Support interactive tools (gum, fzf, dialog)

## Project Structure

```
jarvis/
├── src/
│   ├── main.rs           # Application entry, terminal setup, event loop
│   ├── script/
│   │   ├── discovery.rs  # Find .sh files, map to categories
│   │   ├── parser.rs     # Parse function arrays with regex
│   │   ├── executor.rs   # Execute with full terminal access
│   │   └── mod.rs        # Module exports
│   └── ui/
│       ├── app.rs        # App state, navigation logic
│       ├── render.rs     # Ratatui rendering logic
│       └── mod.rs        # Module exports
└── scripts/              # User's bash scripts
```

## Key Patterns

### Script Discovery
```rust
// Always scan scripts/ directory relative to CWD
let scripts_dir = std::env::current_dir()?.join("scripts");

// Filter for .sh files only
if extension == "sh" { }

// Map filename to category (customize in discovery.rs)
let category = match name.as_str() {
    "fedora" => "System Management",
    _ => "Other",
};
```

### Script Parsing
```rust
// Look for arrays: `name_functions=("Display:function" ...)`
let array_re = Regex::new(r#"(\w+_functions)=\(\s*([^)]+)\s*\)"#)?;

// Parse items: "Display Name:function_name"
let item_re = Regex::new(r#""([^:]+):([^"]+)""#)?;
```

### Interactive Execution
```rust
// IMPORTANT: Always inherit stdio for interactive scripts
Command::new("bash")
    .stdin(Stdio::inherit())
    .stdout(Stdio::inherit())
    .stderr(Stdio::inherit())
    .status()?;
```

### TUI State Management
```rust
pub enum AppState {
    MainMenu,      // Show categories
    CategoryView,  // Show functions in selected category
    Executing,     // (Future: show live output)
    ViewingOutput, // (Future: show execution results)
}
```

## Feature Development Guidelines

### Adding New Features

**Before implementing:**
1. Does it align with "zero configuration" principle?
2. Does it keep the tool universal (not domain-specific)?
3. Does it improve UX without adding complexity?

**Feature Ideas (Welcome PRs):**
- [ ] Fuzzy search/filter (press `/` to search)
- [ ] Custom category mapping via config file
- [ ] Parse script comments for descriptions (`# @description: ...`)
- [ ] Favorites/starred functions
- [ ] Execution history
- [ ] Multi-line function array parsing
- [ ] Script validation on discovery
- [ ] Keyboard shortcuts customization
- [ ] Theme support (Catppuccin, Nord, etc.)

### Testing Approach

**Manual Testing:**
```bash
# Build and test
cargo build --release
./target/release/jarvis

# Test with different script structures
# Test category navigation
# Test function execution
# Test interactive scripts (with gum/fzf)
```

**Unit Tests:**
```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_parse_function_array() {
        let content = r#"
            test_functions=(
                "Test One:test_one"
                "Test Two:test_two"
            )
        "#;
        let functions = parse_script_content(content, "Test").unwrap();
        assert_eq!(functions.len(), 2);
    }
}
```

## Common Tasks

### Adding a New Script Category

Edit `src/script/discovery.rs`:
```rust
let category = match name.as_str() {
    "myproject" => "My Project",
    "devops" => "DevOps Tools",
    "security" => "Security",
    _ => "Other",
};
```

### Modifying the TUI Layout

Edit `src/ui/render.rs`:
```rust
// Follow Ratatui patterns
// Use Layout for responsive sizing
// Keep it simple and readable
```

### Improving Script Parsing

Edit `src/script/parser.rs`:
```rust
// Use regex for robust parsing
// Handle multi-line arrays
// Extract metadata from comments
```

## Dependencies

**Required:**
- `ratatui = "0.28"` - TUI framework
- `crossterm = "0.28"` - Terminal control
- `tokio = "1"` - Async runtime
- `anyhow = "1"` - Error handling
- `regex = "1"` - Pattern matching
- `walkdir = "2"` - Directory traversal

**When adding dependencies:**
- Justify why it's needed
- Consider binary size impact
- Prefer well-maintained crates
- Check licensing compatibility (MIT preferred)

## Git Workflow

```bash
# Create feature branch
git checkout -b feature/your-feature

# Make changes, commit often
git add .
git commit -m "Add fuzzy search for functions"

# Format and lint before PR
cargo fmt
cargo clippy

# Push and create PR
git push -u origin feature/your-feature
```

## Commit Message Format

```
<type>: <description>

Examples:
feat: Add fuzzy search functionality
fix: Handle multi-line function arrays correctly
docs: Update README with new examples
refactor: Simplify script discovery logic
perf: Optimize regex compilation
test: Add unit tests for parser
```

## Questions to Ask When Developing

1. **Is it zero-config?** - Users shouldn't need to configure anything
2. **Is it universal?** - Works with any bash scripts, not just specific use cases
3. **Is it intuitive?** - Navigation should be obvious
4. **Is it fast?** - Script discovery and execution should be instant
5. **Is it maintainable?** - Code should be clear and well-documented

## Example Snippets

### Adding a New UI State
```rust
// 1. Add to AppState enum (ui/app.rs)
pub enum AppState {
    MainMenu,
    CategoryView,
    SearchMode,  // <- New state
}

// 2. Handle in event loop (main.rs)
AppState::SearchMode => match key.code {
    KeyCode::Esc => {
        app.state = AppState::MainMenu;
        app.search_query.clear();
    }
    KeyCode::Char(c) => {
        app.search_query.push(c);
        app.filter_functions();
    }
    _ => {}
}

// 3. Render in UI (ui/render.rs)
AppState::SearchMode => {
    // Render search input and filtered results
}
```

### Adding Category Metadata
```rust
pub struct Category {
    pub name: String,
    pub icon: String,
    pub description: String,
}

impl Category {
    pub fn from_script_name(name: &str) -> Self {
        match name {
            "devops" => Category {
                name: "DevOps".to_string(),
                icon: "🚀".to_string(),
                description: "Deployment and infrastructure".to_string(),
            },
            _ => Category::default(),
        }
    }
}
```

## Anti-Patterns to Avoid

❌ **Don't:**
- Hard-code paths or assume specific directory structure
- Make assumptions about script content beyond the function array format
- Add features that require configuration files
- Break the single binary distribution model
- Add domain-specific logic (homelab-only, devops-only, etc.)

✅ **Do:**
- Keep the tool generic and flexible
- Support any bash script that follows the simple convention
- Maintain zero-configuration philosophy
- Preserve full terminal access for scripts
- Keep the binary small and fast

## Support and Resources

- **Documentation**: See README.md, QUICKSTART.md, JARVIS-TUI-DESIGN.md
- **Ratatui Docs**: https://docs.rs/ratatui/
- **Crossterm Docs**: https://docs.rs/crossterm/
- **Rust Book**: https://doc.rust-lang.org/book/

## Summary

When working on Jarvis, remember:
- It's a **universal** tool for **any** bash scripts
- Zero configuration is key
- Beautiful UX matters
- Keep it simple and maintainable
- Full terminal access for script execution
