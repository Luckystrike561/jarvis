#!/usr/bin/env bash

# Example script showing how to use Jarvis
# This script will be automatically discovered by Jarvis

# Define your functions in an array with format: "Display Name:function_name"
# shellcheck disable=SC2034  # This array is used by Jarvis for function discovery
example_functions=(
	"Hello World:hello_world"
	"System Info:show_system_info"
	"Interactive Demo:interactive_demo"
)

# Implement your functions below
hello_world() {
	echo "👋 Hello from Jarvis!"
	echo ""
	echo "This is an example function."
	echo "You can add any bash commands here."
}

show_system_info() {
	echo "💻 System Information"
	echo "===================="
	echo ""
	echo "Hostname: $(hostname)"
	echo "OS: $(uname -s)"
	echo "Kernel: $(uname -r)"
	echo "Architecture: $(uname -m)"
	echo ""
	echo "Current User: $USER"
	echo "Home Directory: $HOME"
	echo "Shell: $SHELL"
}

interactive_demo() {
	echo "🎯 Interactive Demo"
	echo "==================="
	echo ""
	echo "This function shows that you can use interactive commands!"
	echo ""

	# Simple read for input
	read -r -p "Enter your name: " name
	echo ""
	echo "Nice to meet you, $name! 🎉"
	echo ""
	echo "Jarvis supports full terminal access, so you can use:"
	echo "  - read for input"
	echo "  - gum for beautiful prompts"
	echo "  - fzf for selections"
	echo "  - dialog for UI elements"
	echo "  - Any interactive CLI tool!"
}
