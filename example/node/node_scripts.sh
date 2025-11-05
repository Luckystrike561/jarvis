#!/usr/bin/env bash

# Node.js project management script for Jarvis
# This script wraps common npm commands for easy access through the TUI

# @emoji 🚀
# @description Start the application in production mode
start_server() {
    echo "🚀 Starting server..."
    npm start
}

# @emoji 🔧
# @description Start development server with hot reload
start_dev_server() {
    echo "🔧 Starting development server with hot reload..."
    npm run dev
}

# @emoji 📦
# @description Build the project for production
build_production() {
    echo "📦 Building for production..."
    npm run build
    echo "✅ Build complete! Output in ./dist/"
}

# @emoji 🔨
# @description Build the project for development
build_development() {
    echo "🔨 Building for development..."
    npm run build:dev
    echo "✅ Development build complete!"
}

# @emoji 🧪
# @description Run all tests with coverage report
run_tests() {
    echo "🧪 Running test suite with coverage..."
    npm test
}

# @emoji 👀
# @description Run tests in watch mode for development
run_tests_watch() {
    echo "👀 Starting test watcher..."
    npm run test:watch
}

# @emoji 🎯
# @description Run only unit tests
run_unit_tests() {
    echo "🎯 Running unit tests..."
    npm run test:unit
}

# @emoji 🔗
# @description Run only integration tests
run_integration_tests() {
    echo "🔗 Running integration tests..."
    npm run test:integration
}

# @emoji 🔍
# @description Lint code and check for issues
lint_code() {
    echo "🔍 Linting code..."
    npm run lint
}

# @emoji ✨
# @description Lint and automatically fix issues
lint_and_fix() {
    echo "✨ Linting and fixing code..."
    npm run lint:fix
    echo "✅ Linting complete!"
}

# @emoji 💅
# @description Format all code with Prettier
format_code() {
    echo "💅 Formatting code..."
    npm run format
    echo "✅ Code formatted!"
}

# @emoji ✅
# @description Check code formatting without modifying files
check_formatting() {
    echo "✅ Checking code formatting..."
    npm run format:check
}

# @emoji 🔤
# @description Run TypeScript type checking
check_types() {
    echo "🔤 Running TypeScript type checker..."
    npm run type-check
}

# @emoji 🧹
# @description Clean build artifacts and dependencies
clean_project() {
    echo "🧹 Cleaning project..."
    npm run clean
    echo "✅ Project cleaned!"
}

# @emoji 🔄
# @description Clean install all dependencies
clean_install() {
    echo "🔄 Performing clean installation..."
    npm run install:clean
    echo "✅ Clean install complete!"
}

# @emoji 🔒
# @description Run security audit
security_audit() {
    echo "🔒 Running security audit..."
    npm run audit
}

# @emoji 🛡️
# @description Run security audit and auto-fix issues
security_audit_fix() {
    echo "🛡️ Running security audit with auto-fix..."
    npm run audit:fix
}

# @emoji 🚢
# @description Deploy to production
deploy_production() {
    echo "🚢 Deploying to production..."
    echo "⚠️  This will deploy to the production environment!"
    read -p "Continue? (y/n) " -n 1 -r
    echo
    if [[ $REPLY =~ ^[Yy]$ ]]; then
        npm run deploy:prod
        echo "✅ Deployed to production!"
    else
        echo "❌ Deployment cancelled"
    fi
}

# @emoji 🎭
# @description Deploy to staging environment
deploy_staging() {
    echo "🎭 Deploying to staging..."
    npm run deploy:staging
    echo "✅ Deployed to staging!"
}

# @emoji 💾
# @description Run database migrations
migrate_database() {
    echo "💾 Running database migrations..."
    npm run db:migrate
    echo "✅ Migrations complete!"
}

# @emoji 🌱
# @description Seed database with sample data
seed_database() {
    echo "🌱 Seeding database..."
    npm run db:seed
    echo "✅ Database seeded!"
}

# @emoji 🔄
# @description Reset database and run migrations with seed data
reset_database() {
    echo "🔄 Resetting database..."
    echo "⚠️  This will delete all data!"
    read -p "Continue? (y/n) " -n 1 -r
    echo
    if [[ $REPLY =~ ^[Yy]$ ]]; then
        npm run db:reset
        echo "✅ Database reset complete!"
    else
        echo "❌ Database reset cancelled"
    fi
}

# @emoji 🐳
# @description Build Docker image
docker_build() {
    echo "🐳 Building Docker image..."
    npm run docker:build
    echo "✅ Docker image built!"
}

# @emoji 🏃
# @description Run Docker container
docker_run() {
    echo "🏃 Running Docker container..."
    npm run docker:run
}

# @emoji 📦
# @description Start services with Docker Compose
docker_compose_up() {
    echo "📦 Starting Docker Compose services..."
    npm run docker:compose
    echo "✅ Services started!"
}

# @emoji 🛑
# @description Stop Docker Compose services
docker_compose_down() {
    echo "🛑 Stopping Docker Compose services..."
    npm run docker:down
    echo "✅ Services stopped!"
}

# @emoji 📊
# @description View application logs
view_logs() {
    echo "📊 Viewing application logs (Ctrl+C to exit)..."
    npm run logs
}

# @emoji ❌
# @description View error logs
view_error_logs() {
    echo "❌ Viewing error logs (Ctrl+C to exit)..."
    npm run logs:error
}

# @emoji ⚡
# @description Run performance benchmarks
run_benchmarks() {
    echo "⚡ Running performance benchmarks..."
    npm run benchmark
}

# @emoji 📈
# @description Analyze bundle size
analyze_bundle() {
    echo "📈 Analyzing bundle size..."
    npm run analyze
}

# @emoji 📋
# @description Display project information and available scripts
show_project_info() {
    echo "📋 Project Information"
    echo "====================="
    echo ""
    echo "Name: $(node -p "require('./package.json').name")"
    echo "Version: $(node -p "require('./package.json').version")"
    echo "Description: $(node -p "require('./package.json').description")"
    echo ""
    echo "Available npm scripts:"
    npm run
}

# @ignore
_check_node_modules() {
    # Helper function to check if node_modules exists
    if [ ! -d "node_modules" ]; then
        echo "⚠️  node_modules not found. Run 'npm install' first."
        return 1
    fi
    return 0
}

# @ignore
_check_package_json() {
    # Helper function to verify package.json exists
    if [ ! -f "package.json" ]; then
        echo "❌ package.json not found in current directory"
        return 1
    fi
    return 0
}
