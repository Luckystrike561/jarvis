# Example Terraform configuration for testing Jarvis Terraform/OpenTofu integration
# See: https://www.terraform.io/ or https://opentofu.org/
#
# This is a minimal local-only configuration that requires no cloud credentials.
# It uses the `null_resource` and `local_file` providers to demonstrate
# Terraform commands without any real infrastructure.

terraform {
  required_version = ">= 1.0"

  required_providers {
    local = {
      source  = "hashicorp/local"
      version = "~> 2.0"
    }
    null = {
      source  = "hashicorp/null"
      version = "~> 3.0"
    }
  }
}

# Create a local file as a simple demonstration
resource "local_file" "hello" {
  content  = "Hello from Jarvis Terraform example!"
  filename = "${path.module}/output/hello.txt"
}

# A null resource that runs a local command
resource "null_resource" "echo" {
  provisioner "local-exec" {
    command = "echo 'Terraform apply completed successfully!'"
  }

  triggers = {
    always_run = timestamp()
  }
}

# Read the local file back as a data source (demonstrates data block targeting)
data "local_file" "hello_content" {
  filename = local_file.hello.filename
}

# A module reference (demonstrates module block targeting)
# Note: This module path doesn't need to exist for Jarvis to discover the block
module "example" {
  source = "./modules/example"
  name   = var.name
}
