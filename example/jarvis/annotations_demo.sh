#!/usr/bin/env bash

# Demo script showcasing @emoji and @description annotations
# This demonstrates how to customize function display in Jarvis TUI

# Example 1: Function with both emoji and description
# @emoji 🚀
# @description Deploy the application to production environment
deploy_to_production() {
    echo "🚀 Deploying to production..."
    echo "✓ Building application"
    echo "✓ Running tests"
    echo "✓ Deploying artifacts"
    echo "✓ Deployment complete!"
}

# Example 2: Function with only description (no emoji)
# @description Run the full test suite with coverage reports
run_test_suite() {
    echo "Running comprehensive test suite..."
    echo "✓ Unit tests"
    echo "✓ Integration tests"
    echo "✓ Generating coverage report"
    echo "Done!"
}

# Example 3: Function with only emoji (no description)
# @emoji 🧹
cleanup_build_artifacts() {
    echo "🧹 Cleaning up build artifacts..."
    echo "Removing: target/, dist/, *.log"
    echo "Cleanup complete!"
}

# Example 4: Function without any annotations
# This will use the default display name "Start Dev Server"
start_dev_server() {
    echo "Starting development server on port 3000..."
    echo "Server is running at http://localhost:3000"
}

# Example 5: Multiple annotations with extra spacing
# @emoji 📊
# @description Generate detailed analytics report for the last 30 days
generate_analytics_report() {
    echo "📊 Generating analytics report..."
    echo "Period: Last 30 days"
    echo "Metrics: Users, Sessions, Page Views, Conversions"
    echo "Report saved to ./reports/analytics.html"
}

# Example 6: Development workflow
# @emoji 🔧
# @description Set up local development environment with all dependencies
setup_dev_environment() {
    echo "🔧 Setting up development environment..."
    echo "✓ Installing dependencies"
    echo "✓ Configuring environment variables"
    echo "✓ Initializing database"
    echo "✓ Starting services"
    echo "Environment ready!"
}

# Example 7: Maintenance task
# @emoji 🔄
# @description Update all project dependencies and run security audit
update_dependencies() {
    echo "🔄 Updating dependencies..."
    echo "✓ Checking for updates"
    echo "✓ Installing updates"
    echo "✓ Running security audit"
    echo "✓ All dependencies up to date!"
}

# Example 8: Database operations
# @emoji 💾
# @description Create database backup with timestamp and compression
backup_database() {
    local timestamp=$(date +%Y%m%d_%H%M%S)
    echo "💾 Creating database backup..."
    echo "Backup file: backup_${timestamp}.sql.gz"
    echo "✓ Backup completed successfully"
}

# Example 9: Hidden utility function
# @ignore
format_timestamp() {
    # Internal utility function - hidden from TUI
    # This is used by other functions but shouldn't be called directly
    date +%Y-%m-%d_%H:%M:%S
}

# Example 10: Another hidden helper
# @ignore
validate_input() {
    # Validation helper - not meant for direct execution
    [[ -n "$1" ]] && return 0 || return 1
}

# Example 11: Hidden function with metadata (still hidden despite having emoji)
# @ignore
# @emoji 🔧
# @description Internal helper for string processing
_internal_string_processor() {
    # Even with emoji and description, this is hidden from the TUI
    echo "$1" | tr '[:lower:]' '[:upper:]'
}
