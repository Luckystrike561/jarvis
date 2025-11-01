# AGENTS.md - Development Guide for Jarvis Repository

## Build/Test Commands
- Main entry: `./jarvis.sh` or `bash main.sh` (interactive function selector)
- Test single function: `bash -c "source util.sh; smart"` (replace with function name)
- No automated tests or linting - manual testing required for system automation scripts

## Code Style - Bash Scripts
- **Shebang**: Always start with `#!/usr/bin/env bash`
- **Naming**: snake_case for functions/variables, ALL_CAPS for constants/arrays
- **Functions**: Declare as `function_name() { ... }` with descriptive names
- **Arrays**: Use descriptive array names ending in `_functions` for menu items
- **Error handling**: Use `&>/dev/null` for command suppression, check exit codes
- **Dependencies**: Use `is_command` utility function to check command availability

## Project Structure
- `main.sh` - Entry point with fzf-based function selector
- `util.sh` - Utility functions and S.M.A.R.T. diagnostics
- `fedora.sh` - Fedora-specific installation and system management functions  
- `homelab.sh` - Kubernetes cluster and homelab infrastructure functions
- Menu arrays follow format: `"Display Name:function_name"`

## Development Guidelines
- **Purpose**: System automation and OS setup assistant for Shield project
- **Dependencies**: Requires `fzf` for interactive selection, installs via DNF if missing
- **Function Discovery**: Add new functions to appropriate `*_functions` arrays for menu visibility
- **Error Messages**: Use emoji prefixes (❌ ERROR:, ✅ OK) for visual feedback
- **Interactive**: Use `gum` for user prompts, `fzf` for selections where available
- **Safety**: Always check command existence before execution, handle missing dependencies gracefully