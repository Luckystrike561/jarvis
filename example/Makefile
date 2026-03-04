# Example Makefile for testing Jarvis Make integration
# See: https://www.gnu.org/software/make/

.PHONY: all build test check clean help release deploy exit_success exit_failure

# @emoji 🔨
# @description Build the project in debug mode
build:
	@echo "🔨 Building project in debug mode..."
	@echo "   Compiling sources..."
	@echo "   Linking binaries..."
	@echo "✅ Build complete!"

# @emoji 🧪
# @description Run the test suite
test:
	@echo "🧪 Running test suite..."
	@echo "   Running unit tests... 42/42 passed"
	@echo "   Running integration tests... 8/8 passed"
	@echo "✅ All tests passed!"

# @emoji ✅
# @description Run linter and format checker
check:
	@echo "✅ Running linter..."
	@echo "   No warnings found."
	@echo "   Checking code format..."
	@echo "   All files are properly formatted."
	@echo "✅ Check passed!"

# @emoji 🧹
# @description Clean build artifacts
clean:
	@echo "🧹 Cleaning build artifacts..."
	@echo "   Removing target/ directory..."
	@echo "   Removing dist/ directory..."
	@echo "✅ Clean complete!"

# @emoji 📦
# @description Build optimized release binary
release:
	@echo "📦 Building optimized release binary..."
	@echo "   Optimizing with LTO..."
	@echo "   Stripping debug symbols..."
	@echo "✅ Release build complete! Binary at ./target/release/app"

# @emoji 🚀
# @description Deploy to the staging environment
deploy:
	@echo "🚀 Deploying to staging..."
	@echo "   Uploading artifacts..."
	@echo "   Restarting services..."
	@echo "✅ Deployed successfully!"

# @emoji ❓
# @description Show available make targets
help:
	@echo "Available targets:"
	@echo "  build   - Build the project"
	@echo "  test    - Run tests"
	@echo "  check   - Run linter and format check"
	@echo "  clean   - Clean build artifacts"
	@echo "  release - Build release binary"
	@echo "  deploy  - Deploy to staging"
	@echo "  help    - Show this help"

# @emoji ✅
# @description Exit with code 0 — test success display in UI
exit_success:
	@echo "✅ This target exits successfully (code 0)."
	@exit 0

# @emoji ❌
# @description Exit with code 1 — test error/failure display in UI
exit_failure:
	@echo "❌ This target exits with an error (code 1)."
	@echo "   Jarvis should indicate this target failed."
	@exit 1

# @ignore
_internal_helper:
	@echo "This is an internal helper target (hidden from TUI)"
