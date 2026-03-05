# Example Terraform configuration for testing Jarvis Terraform/OpenTofu integration
# See: https://www.terraform.io/ or https://opentofu.org/
#
# This is a minimal local-only configuration that requires no cloud credentials.
# It uses the `local` provider to demonstrate Terraform commands without any real infrastructure.

terraform {
  required_version = ">= 1.0"

  required_providers {
    local = {
      source  = "hashicorp/local"
      version = "~> 2.0"
    }
  }
}

# Create a local file as a simple demonstration
resource "local_file" "hello" {
  content  = "Hello from Jarvis Terraform example!"
  filename = "${path.module}/output/hello.txt"
}

# Another local file resource
resource "local_file" "config" {
  content  = "# Auto-generated config\nenv = \"example\"\nversion = \"1.0.0\""
  filename = "${path.module}/output/config.txt"
}

# Output the file path
output "hello_file_path" {
  value       = local_file.hello.filename
  description = "Path to the hello file"
}

output "config_file_path" {
  value       = local_file.config.filename
  description = "Path to the config file"
}
