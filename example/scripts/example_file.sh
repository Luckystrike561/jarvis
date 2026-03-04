#!/usr/bin/env bash

# Example script for testing Jarvis TUI features
# Functions here are designed to test: scrolling, select, copy, navigation

# ────────────────────────────────────────────
# Basic functions
# ────────────────────────────────────────────

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
	echo "Hostname:      $(hostname)"
	echo "OS:            $(uname -s)"
	echo "Kernel:        $(uname -r)"
	echo "Architecture:  $(uname -m)"
	echo ""
	echo "Current User:  $USER"
	echo "Home Dir:      $HOME"
	echo "Shell:         $SHELL"
	echo "Date:          $(date)"
}

# ────────────────────────────────────────────
# Scrolling tests — long output
# ────────────────────────────────────────────

# @emoji 📜
# @description Generate 50 lines to test output scrolling (j/k or arrow keys)
long_output_test() {
	echo "📜 Long Output Test — 50 lines"
	echo "==============================="
	echo ""
	echo "Use j/k or ↑/↓ to scroll when the output pane is focused (Tab to focus)."
	echo ""

	for i in {1..50}; do
		printf "Line %02d: The quick brown fox jumps over the lazy dog. Lorem ipsum dolor sit amet.\n" "$i"
	done

	echo ""
	echo "✅ End of output (50 lines)."
}

# @emoji 📃
# @description Generate 100 lines to stress-test scrolling
very_long_output() {
	echo "📃 Very Long Output Test — 100 lines"
	echo "======================================"
	echo ""

	for i in {1..100}; do
		printf "Line %03d | %s\n" "$i" "$(date +%H:%M:%S) — item-$(printf '%03d' "$i") value=$(( i * 7 ))"
	done

	echo ""
	echo "✅ Done! Scrolled through 100 lines."
}

# @emoji 🔢
# @description Print numbered output to test copy selection
numbered_list_output() {
	echo "🔢 Numbered List — ideal for copy/select testing"
	echo "================================================="
	echo ""
	echo "  1. Apple"
	echo "  2. Banana"
	echo "  3. Cherry"
	echo "  4. Date"
	echo "  5. Elderberry"
	echo "  6. Fig"
	echo "  7. Grape"
	echo "  8. Honeydew"
	echo "  9. Kiwi"
	echo " 10. Lemon"
	echo " 11. Mango"
	echo " 12. Nectarine"
	echo " 13. Orange"
	echo " 14. Papaya"
	echo " 15. Quince"
	echo ""
	echo "✅ Select any line above and copy with your terminal!"
}

# @emoji 📋
# @description Print copy-friendly multiline text blocks
copy_test_blocks() {
	echo "📋 Copy Test — structured text blocks"
	echo "======================================="
	echo ""
	echo "=== Block 1: JSON-like ==="
	echo '{'
	echo '  "name": "jarvis",'
	echo '  "version": "1.0.0",'
	echo '  "description": "TUI script runner",'
	echo '  "author": "Luckystrike561"'
	echo '}'
	echo ""
	echo "=== Block 2: Table ==="
	echo "Name          | Status  | Value"
	echo "------------- | ------- | -----"
	echo "alpha         | active  | 100"
	echo "beta          | idle    | 42"
	echo "gamma         | error   | 0"
	echo "delta         | active  | 999"
	echo ""
	echo "=== Block 3: Shell commands ==="
	echo "  git status"
	echo "  git add ."
	echo "  git commit -m 'feat: add feature'"
	echo "  git push origin main"
	echo ""
	echo "✅ Try selecting and copying any block above!"
}

# ────────────────────────────────────────────
# Interactive input tests
# ────────────────────────────────────────────

# @emoji 🎯
# @description Prompt for name input — tests interactive stdin
interactive_demo() {
	echo "🎯 Interactive Demo"
	echo "==================="
	echo ""
	echo "This function uses 'read' to accept user input."
	echo ""

	read -r -p "Enter your name: " name
	echo ""
	echo "Nice to meet you, ${name:-stranger}! 🎉"
	echo ""
	echo "Jarvis supports full terminal access:"
	echo "  - read for input"
	echo "  - gum for beautiful prompts"
	echo "  - fzf for selections"
	echo "  - dialog for UI elements"
}

# @emoji ❓
# @description Ask a yes/no question — tests interactive confirmation
confirm_demo() {
	echo "❓ Confirmation Demo"
	echo "===================="
	echo ""
	read -r -p "Do you want to continue? [y/N] " answer
	echo ""
	case "${answer,,}" in
	y | yes)
		echo "✅ You chose: YES — continuing!"
		;;
	*)
		echo "❌ You chose: NO — stopping."
		;;
	esac
}

# ────────────────────────────────────────────
# Output format tests
# ────────────────────────────────────────────

# @emoji 🌈
# @description Print colored output using ANSI escape codes
color_output_test() {
	echo "🌈 ANSI Color Output Test"
	echo "=========================="
	echo ""
	echo -e "\033[31mRed text\033[0m"
	echo -e "\033[32mGreen text\033[0m"
	echo -e "\033[33mYellow text\033[0m"
	echo -e "\033[34mBlue text\033[0m"
	echo -e "\033[35mMagenta text\033[0m"
	echo -e "\033[36mCyan text\033[0m"
	echo -e "\033[1mBold text\033[0m"
	echo -e "\033[4mUnderlined text\033[0m"
	echo ""
	echo -e "\033[42m\033[30m SUCCESS \033[0m Green background"
	echo -e "\033[41m\033[37m  ERROR  \033[0m Red background"
	echo -e "\033[43m\033[30m WARNING \033[0m Yellow background"
	echo ""
	echo "✅ Colors should render if your terminal supports ANSI codes."
}

# @emoji ⏱️
# @description Simulate a progress bar over time
progress_bar_demo() {
	echo "⏱️  Progress Bar Demo"
	echo "======================"
	echo ""
	echo "Simulating a task with progress..."
	echo ""

	for i in {1..20}; do
		filled=$(printf '█%.0s' $(seq 1 "$i"))
		empty=$(printf '░%.0s' $(seq 1 $((20 - i))))
		pct=$(( i * 5 ))
		printf "\rProgress: [%s%s] %d%%" "$filled" "$empty" "$pct"
		sleep 0.1
	done

	echo ""
	echo ""
	echo "✅ Task complete!"
}

# @emoji 📊
# @description Print a wide table to test horizontal display
wide_table_test() {
	echo "📊 Wide Table Test"
	echo "==================="
	echo ""
	printf "%-20s %-10s %-10s %-12s %-15s %-10s\n" "Name" "Status" "CPU%" "Memory" "Uptime" "Requests"
	printf "%-20s %-10s %-10s %-12s %-15s %-10s\n" "--------------------" "----------" "----------" "------------" "---------------" "----------"
	printf "%-20s %-10s %-10s %-12s %-15s %-10s\n" "web-server-01"   "running" "12.4"  "512MB"   "2d 14h 32m"  "142301"
	printf "%-20s %-10s %-10s %-12s %-15s %-10s\n" "web-server-02"   "running" "8.1"   "489MB"   "2d 14h 31m"  "138922"
	printf "%-20s %-10s %-10s %-12s %-15s %-10s\n" "api-gateway"     "running" "23.7"  "1.2GB"   "5d 02h 17m"  "512043"
	printf "%-20s %-10s %-10s %-12s %-15s %-10s\n" "database-primary" "running" "45.2"  "8.0GB"   "30d 00h 05m" "2871204"
	printf "%-20s %-10s %-10s %-12s %-15s %-10s\n" "cache-server"    "running" "2.3"   "2.0GB"   "15d 08h 44m" "9923401"
	printf "%-20s %-10s %-10s %-12s %-15s %-10s\n" "worker-01"       "idle"    "0.1"   "256MB"   "1d 06h 02m"  "0"
	printf "%-20s %-10s %-10s %-12s %-15s %-10s\n" "worker-02"       "stopped" "0.0"   "0MB"     "-"           "0"
	echo ""
	echo "✅ Scroll horizontally if your terminal is narrow."
}

# ────────────────────────────────────────────
# Error / exit code tests
# ────────────────────────────────────────────

# @emoji ✅
# @description Exit with code 0 — success
exit_success() {
	echo "✅ This function exits successfully (code 0)."
	exit 0
}

# @emoji ❌
# @description Exit with code 1 — failure (tests error handling display)
exit_failure() {
	echo "❌ This function exits with an error (code 1)."
	echo "   Jarvis should show this ran but failed."
	exit 1
}

# @ignore
_internal_helper() {
	# Hidden utility — not visible in TUI
	echo "internal"
}
