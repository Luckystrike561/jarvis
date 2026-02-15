# Example variables for testing Jarvis Terraform/OpenTofu integration

variable "environment" {
  description = "The environment name (e.g., dev, staging, production)"
  type        = string
  default     = "dev"
}

variable "project_name" {
  description = "Name of the project"
  type        = string
  default     = "jarvis-example"
}
