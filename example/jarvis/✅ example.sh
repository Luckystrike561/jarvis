#!/usr/bin/env bash

# Example custom script for the jarvis/ directory
# This directory is gitignored, so you can add personal scripts here
# without committing them to the repository

# All functions defined in this script will be automatically discovered
# Function names will be formatted for display (e.g., custom_hello -> "Custom Hello")
custom_hello() {
	echo "👋 Hello from your personal jarvis/ directory!"
	echo ""
	echo "This is a custom function that won't be tracked by git."
	echo "Perfect for personal workflows and sensitive scripts."
}

my_workflow() {
	echo "🔧 My Custom Workflow"
	echo "====================="
	echo ""
	echo "This is where you can add your personal automation."
	echo "Examples:"
	echo "  - Deploy to your personal server"
	echo "  - Run your specific build process"
	echo "  - Manage your local development environment"
	echo ""
	echo "Add your commands here!"
}

personal_task() {
	echo "📋 Personal Task Runner"
	echo "======================"
	echo ""
	echo "Use this for tasks specific to your setup:"
	echo "  - Backup personal files"
	echo "  - Sync with cloud storage"
	echo "  - Run local database migrations"
	echo "  - Start your dev services"
	echo ""
	echo "Customize this function for your needs!"
}
