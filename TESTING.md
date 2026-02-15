# Jarvis TUI Testing Guide

This guide covers both manual TUI testing and automated unit/integration tests.

## Automated Tests

### Running Tests

```bash
# Run all 289 tests
cargo test

# Run specific test
cargo test test_name

# Run tests with output
cargo test -- --nocapture

# With devbox
devbox run test
```

### Test Coverage

Jarvis has comprehensive test coverage across all modules:

- **289 total tests** covering ~90% of testable code
- **Unit tests** for script discovery, parsing, npm parsing, PTY execution, and terminal widget
- **Integration tests** for application logic and edge cases
- **Mock-based tests** for TUI event handling and key input processing

**Coverage Breakdown:**
- `src/script/discovery.rs` - ~95% (script and package.json discovery)
- `src/script/parser.rs` - ~95% (bash function parsing with annotations)
- `src/script/npm_parser.rs` - ~90% (package.json parsing)
- `src/script/cargo_parser.rs` - ~90% (Cargo.toml parsing)
- `src/script/devbox_parser.rs` - ~90% (devbox.json parsing)
- `src/script/just_parser.rs` - ~90% (Justfile parsing)
- `src/script/makefile_parser.rs` - ~90% (Makefile parsing)
- `src/script/task_parser.rs` - ~90% (Taskfile.yml parsing)
- `src/script/nx_parser.rs` - ~90% (Nx workspace parsing)
- `src/script/terraform_parser.rs` - ~90% (Terraform/OpenTofu command discovery)
- `src/ui/app.rs` - ~90% (application state and navigation)
- `src/ui/pty_runner.rs` - ~85% (PTY execution, shell escaping, command building)
- `src/ui/terminal_widget.rs` - ~80% (selection, scrollback, color conversion)
- `src/main.rs` - ~85% (key event handling, integration tests)
- `src/ui/render.rs` - ~5% (UI rendering - difficult to test)

## Manual TUI Testing

### Quick Start

```bash
# From the project root, test with the example directory:
jarvis -p example
```

## What to Test

### 1. Basic Navigation ✅
- **Arrow Keys** (↑/↓) or **j/k** to navigate through options
- **Enter** to select a script or function
- **Backspace/Esc** to go back to previous menu
- **Q** to quit

### 2. Script Discovery ✅
When you start Jarvis with `jarvis -p example`, you should see:
- **example.sh** - Example bash functions
- **Taskfile.yml** - Task runner targets
- **Makefile** - Make targets
- **package.json** - npm scripts (from example/node/)
- **devbox.json** - Devbox scripts
- **justfile** - Just recipes
- **Cargo.toml** - Cargo commands (from example/cargo-demo/)
- **nx.json** - Nx project targets (from example/nx/)
- **terraform/** - Terraform/OpenTofu commands (from example/terraform/)

### 3. Non-Interactive Execution ✅
Select a simple echo function from any discovered script.
- Should display text output inline in the embedded terminal
- Press Esc or Backspace to return to the menu

### 4. Interactive Execution ✅ (CRITICAL TEST)
Select an interactive function (e.g., one that uses `gum` or `read`).
- The script should run in the embedded PTY terminal
- You should be able to type input directly
- Script output should be visible inline
- Press Esc or Backspace to return to the menu after completion

## Expected Behavior

### ✅ Working Correctly
- Scripts execute in the embedded PTY terminal
- Interactive prompts work (gum, read, etc.)
- You can type input and see what you're typing
- Script output is visible and readable
- Navigation works smoothly after returning from execution

### ❌ Issues to Watch For
- Cannot type in interactive prompts → PTY stdin not working
- No script output visible → PTY rendering issue
- Cursor not visible during prompts → terminal state issue
- Cannot return to menu after script → key handling issue

## Test Results Template

```
✅ Basic navigation: PASS/FAIL
✅ Script discovery: PASS/FAIL
✅ Non-interactive execution: PASS/FAIL
✅ Interactive execution (PTY): PASS/FAIL
✅ npm scripts (if available): PASS/FAIL
✅ Terraform/OpenTofu commands (if binary available): PASS/FAIL
```

## Troubleshooting

### If gum is not installed:
```bash
# Devbox should have it, but if not:
go install github.com/charmbracelet/gum@latest
```

### If TUI doesn't start:
- Make sure you're running in a real terminal (not background)
- Try running with: `cargo run --release`

### If interactive input doesn't work:
- Check the PTY implementation in `src/ui/pty_runner.rs`
- Verify the `build_command()` function generates correct shell commands
- Check that key events are being forwarded to the PTY in `src/main.rs`

## Success Criteria

**All manual TUI tests pass** = Interactive input support is working correctly! 🎉

**All 289 automated tests pass** = Code quality and functionality are maintained! 🎉

The key indicator for manual testing is: **You can type into gum prompts and see your input.**
