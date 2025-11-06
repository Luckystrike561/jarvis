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
- Node.js/npm (for npm script execution)

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

**Jarvis uses auto-discovery** - all bash functions in `.sh` files are automatically detected. No arrays or special declarations needed!

**Format Requirements:**
```bash
#!/usr/bin/env bash

# Optional: Customize function display with annotations
# @emoji 🚀
# @description Deploy the application to production
deploy_app() {
    echo "Deploying application..."
    # Implementation
}

# Optional: Hide helper functions from TUI
# @ignore
_internal_helper() {
    echo "This won't appear in Jarvis"
}

# Regular functions appear automatically
run_tests() {
    echo "Running tests..."
}
```

**Function Annotations:**
- `@emoji <emoji>` - Add emoji prefix before function name
- `@description <text>` - Custom description for details panel
- `@ignore` - Hide utility/helper functions from TUI
- Place annotations in comments directly above function (consecutive comment lines only)

**Naming:**
- Functions: `snake_case` (auto-converted to "Title Case" in UI)
- Variables: `snake_case`
- Constants: `ALL_CAPS`

**Best Practices:**
- Functions are auto-discovered - no arrays needed
- Use annotations to customize display (`@emoji`, `@description`)
- Hide helper functions with `@ignore`
- Support interactive tools (gum, fzf, dialog)
- Provide clear error messages

## Project Structure

```
jarvis/
├── src/
│   ├── main.rs           # Application entry, terminal setup, event loop
│   ├── script/
│   │   ├── discovery.rs  # Find .sh files and package.json, map to categories
│   │   ├── parser.rs     # Parse bash functions with regex
│   │   ├── npm_parser.rs # Parse package.json scripts
│   │   ├── executor.rs   # Execute with full terminal access
│   │   └── mod.rs        # Module exports
│   └── ui/
│       ├── app.rs        # App state, navigation logic
│       ├── render.rs     # Ratatui rendering logic
│       └── mod.rs        # Module exports
├── example/              # Example scripts and projects for testing
│   ├── jarvis/           # Bash script examples
│   ├── node/             # npm/package.json example
│   └── scripts/          # Additional bash examples
└── scripts/              # User's bash scripts (scanned at CWD)
```

## Key Patterns

### Script Discovery

Jarvis automatically discovers:
1. **Bash scripts** in `.sh` files (current directory + optional subdirectories)
2. **npm scripts** in `package.json` files

```rust
// Scan current directory and optional subdirectories
let search_paths = vec![
    base_path.to_path_buf(),           // ./
    base_path.join("script"),          // ./script/
    base_path.join("scripts"),         // ./scripts/
    base_path.join("jarvis"),          // ./jarvis/
];

// Discovery depths
// Root directory: depth 1 (immediate files only)
// Subdirectories: depth 2 (recursive search)

// Filter for .sh files and package.json
if extension == "sh" || file_name == "package.json" { }

// Map to categories
let category = match name.as_str() {
    "fedora" => "System Management",
    "package.json" => "npm Scripts",
    _ => "Other",
};
```

### Bash Function Parsing

Auto-discovers all bash functions - no arrays needed!

```rust
// Find bash functions with regex
let function_re = Regex::new(r"^(\w+)\s*\(\)\s*\{")?;

// Parse annotations (@emoji, @description, @ignore)
let emoji_re = Regex::new(r"#\s*@emoji\s+(.+)")?;
let desc_re = Regex::new(r"#\s*@description\s+(.+)")?;
let ignore_re = Regex::new(r"#\s*@ignore")?;

// Convert function_name to "Function Name" for display
```

### npm Script Parsing

```rust
// Parse package.json with serde_json
let package_json: serde_json::Value = serde_json::from_str(&content)?;

// Extract scripts section
if let Some(scripts) = package_json.get("scripts") {
    // Each script becomes a ScriptFunction
}
```

### Interactive Execution

**Bash Functions:**
```rust
// IMPORTANT: Always inherit stdio for interactive scripts
Command::new("bash")
    .args(["-c", &format!("source {} && {}", script_path, function_name)])
    .stdin(Stdio::inherit())
    .stdout(Stdio::inherit())
    .stderr(Stdio::inherit())
    .status()?;
```

**npm Scripts:**
```rust
// Execute npm scripts in package.json directory
Command::new("npm")
    .args(["run", script_name])
    .current_dir(package_dir)
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
- [ ] Favorites/starred functions
- [ ] Execution history
- [ ] Script validation on discovery
- [ ] Keyboard shortcuts customization
- [ ] Theme support (Catppuccin, Nord, etc.)
- [ ] Support for more package managers (pnpm, yarn, bun)
- [ ] Parse JSDoc-style comments for npm script descriptions

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
