# Example Makefile for testing Jarvis Make integration
# See: https://www.gnu.org/software/make/

.PHONY: all build test check clean help

# @emoji 🔨
# @description Build the project in debug mode
build:
	cargo build

# @emoji 🧪
# @description Run the test suite
test: build
	cargo test

# @emoji ✅
# @description Run clippy linter and format checker
check:
	cargo clippy -- -D warnings
	cargo fmt -- --check

# @emoji 🧹
# @description Clean build artifacts
clean:
	cargo clean

# @emoji 📦
# @description Build optimized release binary
release:
	cargo build --release

# @emoji ❓
# @description Show available make targets
help:
	@echo "Available targets:"
	@echo "  build   - Build the project"
	@echo "  test    - Run tests"
	@echo "  check   - Run linter and format check"
	@echo "  clean   - Clean build artifacts"
	@echo "  release - Build release binary"
	@echo "  help    - Show this help"

# @ignore
_internal_helper:
	@echo "This is an internal helper target (hidden from TUI)"
