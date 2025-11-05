# User-Specific Jarvis Scripts

This directory is for your **personal custom scripts** that are specific to your workflow and are not tracked in git.

## Purpose

- Store personal automation scripts
- Keep private/sensitive scripts separate from the repository
- Customize Jarvis for your specific needs without affecting the shared codebase

## Usage

Create bash scripts following the same format as the `scripts/` directory:

```bash
#!/usr/bin/env bash

# Define function array
my_custom_functions=(
    "My Function:my_func"
)

# Implement functions
my_func() {
    echo "Hello from my custom script!"
}
```

## Examples

### Personal Workflow Script

```bash
# jarvis/personal.sh
#!/usr/bin/env bash

personal_functions=(
    "Sync Dotfiles:sync_dots"
    "Deploy Personal Site:deploy_site"
)

sync_dots() {
    echo "🔄 Syncing dotfiles..."
    # Your dotfiles sync logic
}

deploy_site() {
    echo "🚀 Deploying personal website..."
    # Your deployment logic
}
```

## Notes

- This directory is ignored by git (see `.gitignore`)
- Scripts here follow the same format as `scripts/` directory
- Jarvis automatically discovers scripts from both directories
- Perfect for company-specific, private, or experimental scripts
