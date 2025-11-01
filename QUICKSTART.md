# Jarvis Quick Start Guide

## What is Jarvis?

Jarvis is a **beautiful TUI (Terminal User Interface) for managing bash scripts**. It automatically discovers your bash functions and presents them in an organized, easy-to-navigate interface.

## Key Features

- 🎯 **Zero Configuration** - Just add bash scripts, no config files needed
- 🎨 **Beautiful Interface** - Modern TUI built with Rust and Ratatui
- ⚡ **Universal** - Works with ANY bash scripts (DevOps, automation, utilities, etc.)
- 📦 **Single Binary** - 3.8MB executable with no runtime dependencies
- 🔄 **Interactive** - Full terminal access for gum, fzf, dialog, and more

## Installation (30 seconds)

```bash
# 1. Clone and build
git clone https://github.com/yourusername/jarvis.git
cd jarvis
cargo build --release

# 2. Run it!
./target/release/jarvis
```

## Adding Your Scripts

Create a bash script in `scripts/` with this format:

```bash
#!/usr/bin/env bash

# Define function array: "Display Name:function_name"
myproject_functions=(
    "Deploy App:deploy_app"
    "Run Tests:run_tests"
)

deploy_app() {
    echo "🚀 Deploying..."
    # Your code here
}

run_tests() {
    echo "🧪 Testing..."
    # Your code here
}
```

That's it! Jarvis will automatically discover and display your functions.

## Usage

```bash
./target/release/jarvis
```

**Navigation:**
- `↑`/`↓` or `j`/`k` - Move up/down
- `Enter` - Select category or execute function
- `Backspace`/`Esc` - Go back
- `q` - Quit

## Example Scripts

Check out `scripts/example.sh` for a working example with:
- Hello World function
- System info display
- Interactive input demo

## Project Structure

```
jarvis/
├── src/              # Rust TUI code
├── scripts/          # YOUR bash scripts go here!
├── target/release/   # Built binary (jarvis)
└── README.md         # Full documentation
```

## Category Customization

Jarvis auto-categorizes scripts by filename. Edit `src/script/discovery.rs` to customize:

```rust
let category = match name.as_str() {
    "myproject" => "My Project",
    "devops" => "DevOps Tools",
    _ => "Other",
};
```

## Tips

- Use emoji in display names for visual appeal (🚀 ✅ 🔧 📦)
- Scripts can use interactive tools (gum, fzf, dialog)
- Group related functions in the same script
- Use `snake_case` for function names

## What Can You Use Jarvis For?

- 🖥️ System administration and setup
- 🏗️ DevOps deployments and CI/CD
- 🔧 Development workflows
- 🏠 Homelab management
- 📊 Data processing scripts
- 🔐 Security tools
- 🎨 Any bash automation!

## Need Help?

- Full docs: `README.md`
- Design docs: `JARVIS-TUI-DESIGN.md`
- Issues: Open a GitHub issue

## License

MIT - Use it for anything!
