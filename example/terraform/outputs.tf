# Example outputs for testing Jarvis Terraform/OpenTofu integration

output "hello_file_path" {
  description = "Path to the generated hello file"
  value       = local_file.hello.filename
}

output "environment" {
  description = "Current environment"
  value       = var.environment
}
