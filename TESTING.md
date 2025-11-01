# Jarvis TUI Testing Guide

## Quick Start

```bash
cd /home/luckystrike561/shield/jarvis
./target/release/jarvis
```

## What to Test

### 1. Basic Navigation ✅
- **Arrow Keys** (↑/↓) or **j/k** to navigate through options
- **Enter** to select a script or function
- **Backspace/Esc** to go back to previous menu
- **Q** to quit

### 2. Script Discovery ✅
When you start Jarvis, you should see:
- **fedora.sh** - Fedora system setup functions
- **homelab.sh** - Homelab/K8s deployment functions
- **test.sh** - Interactive test functions (NEW!)
- **util.sh** - Utility functions

### 3. Non-Interactive Execution ✅
Navigate to: `test.sh` → `Simple echo test`
- Should display text output
- Press Enter to return to TUI

### 4. Interactive gum confirm ✅ (CRITICAL TEST)
Navigate to: `test.sh` → `Interactive gum confirm`
- TUI should **disappear** (suspend)
- You should see: "Do you like the Jarvis TUI?" with Yes/No buttons
- Use arrow keys to select, press Enter
- Should show result message
- Press Enter to return to TUI
- TUI should **reappear** (resume)

### 5. Interactive gum input ✅ (CRITICAL TEST)
Navigate to: `test.sh` → `Interactive gum input`
- TUI should suspend
- You should see: "Enter your name" input field
- Type your name and press Enter
- Should greet you by name
- Press Enter to return to TUI

### 6. Interactive bash read ✅ (CRITICAL TEST)
Navigate to: `test.sh` → `Interactive read command`
- TUI should suspend
- You should see: "Enter your favorite color: "
- Type a color and press Enter
- Should confirm your choice
- Press Enter to return to TUI

### 7. Real-World Test (Optional)
Navigate to: `homelab.sh` → `List all kubernetes resources for a namespace`
- Should prompt with gum input for namespace
- Type a namespace name (e.g., "default")
- Should execute kubectl commands
- Verify output is visible

## Expected Behavior

### ✅ Working Correctly
- TUI suspends cleanly (screen clears, cursor visible)
- Interactive prompts work (gum, read, etc.)
- You can type input and see what you're typing
- Script output is visible and readable
- "Press Enter to continue..." appears after script finishes
- TUI resumes cleanly after pressing Enter
- Navigation works smoothly

### ❌ Issues to Watch For
- Cannot type in gum prompts → stdin not working
- TUI artifacts remain on screen → suspend failed
- Cannot see script output → stdout/stderr not working
- Cursor not visible during prompts → terminal state issue
- Cannot return to TUI after script → resume failed

## Test Results Template

```
✅ Basic navigation: PASS/FAIL
✅ Script discovery: PASS/FAIL
✅ Simple echo test: PASS/FAIL
✅ gum confirm (interactive): PASS/FAIL
✅ gum input (interactive): PASS/FAIL
✅ bash read (interactive): PASS/FAIL
✅ TUI suspend/resume: PASS/FAIL
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
- Check the implementation in `src/script/executor.rs`
- Verify `Stdio::inherit()` is being used
- Confirm `suspend_tui()` and `resume_tui()` are called in `src/main.rs`

## Success Criteria

**All tests pass** = Interactive input support is working correctly! 🎉

The key indicator is: **You can type into gum prompts and see your input.**
