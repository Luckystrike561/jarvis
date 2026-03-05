# Example outputs for testing Jarvis Terraform/OpenTofu integration

output "environment" {
  description = "Current environment"
  value       = var.environment
}

output "project_name" {
  description = "Name of the project"
  value       = var.project_name
}
