#!/usr/bin/env bash

# Example script showing how to use Jarvis
# All functions defined in this script will be automatically discovered
# Function names will be formatted for display (e.g., hello_world -> "Hello World")
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

long_output_test() {
	echo "📜 Long Output Test"
	echo "==================="
	echo ""
	echo "This function generates many lines of output to test scrolling."
	echo ""
	
	for i in {1..50}; do
		echo "Line $i: This is a test line to demonstrate scrolling in the output pane."
		echo "        You can use j/k or arrow keys to scroll when the output pane is focused."
	done
	
	echo ""
	echo "✅ End of output. Try pressing Tab to focus the output pane and scroll!"
}
