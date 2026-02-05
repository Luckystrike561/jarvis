//! # Script Executor
//!
//! This module provides interactive execution of scripts and functions with full
//! terminal access. It handles the actual running of discovered scripts, ensuring
//! that interactive tools (like `gum`, `fzf`, etc.) work correctly.
//!
//! ## Supported Script Types
//!
//! | Type | Function | Command Pattern |
//! |------|----------|-----------------|
//! | Bash | `execute_function_interactive` | `source script.sh && function_name` |
//! | npm | `execute_npm_script_interactive` | `npm run script_name` |
//! | Devbox | `execute_devbox_script_interactive` | `devbox run script_name` |
//! | Task | `execute_task_interactive` | `task --taskfile path task_name` |
//!
//! ## Key Design Decisions
//!
//! ### Full Terminal Access
//!
//! All execution functions inherit stdin, stdout, and stderr from the parent process:
//!
//! ```ignore
//! Command::new("bash")
//!     .stdin(Stdio::inherit())
//!     .stdout(Stdio::inherit())
//!     .stderr(Stdio::inherit())
//! ```
//!
//! This allows scripts to:
//! - Read user input interactively
//! - Display colored output
//! - Use TUI tools like `gum`, `fzf`, `dialog`
//!
//! ### Working Directory
//!
//! Each executor changes to the script's directory before execution, ensuring
//! relative paths in scripts work correctly.
//!
//! ### Input Validation
//!
//! All executors validate:
//! - Path existence and type (file vs directory)
//! - Required config files (package.json, devbox.json, Taskfile)
//! - Valid identifiers (for bash function names)

use anyhow::{Context, Result};
use std::path::Path;
use std::process::{Command, Stdio};

/// Execute a bash function interactively with full terminal access
/// This allows the script to use stdin/stdout/stderr directly (for gum, etc)
pub fn execute_function_interactive(script_path: &Path, function_name: &str) -> Result<i32> {
    // Validate inputs
    if !script_path.exists() {
        anyhow::bail!("Script file not found: {}", script_path.display());
    }

    if !script_path.is_file() {
        anyhow::bail!("Path is not a file: {}", script_path.display());
    }

    // Get script directory and name
    let script_dir = script_path.parent().with_context(|| {
        format!(
            "Failed to get parent directory for: {}",
            script_path.display()
        )
    })?;

    let script_name = script_path
        .file_name()
        .and_then(|n| n.to_str())
        .with_context(|| format!("Invalid script filename: {}", script_path.display()))?;

    // Validate function name
    if function_name.is_empty() {
        anyhow::bail!("Function name cannot be empty");
    }

    if !is_valid_bash_identifier(function_name) {
        anyhow::bail!(
            "Invalid function name '{}'. Must be a valid bash identifier.",
            function_name
        );
    }

    // Create bash command to source file and call function
    let bash_command = format!(
        r#"cd "{}" && source "{}" && {}"#,
        script_dir.display(),
        script_name,
        function_name
    );

    // Execute with inherited stdin/stdout/stderr for full interactivity
    let status = Command::new("bash")
        .arg("-c")
        .arg(&bash_command)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .with_context(|| {
            format!(
                "Failed to execute function '{}' from script '{}'",
                function_name,
                script_path.display()
            )
        })?;

    Ok(status.code().unwrap_or(1))
}

/// Execute an npm script interactively with full terminal access
pub fn execute_npm_script_interactive(package_dir: &Path, script_name: &str) -> Result<i32> {
    // Validate inputs
    if !package_dir.exists() {
        anyhow::bail!("Directory not found: {}", package_dir.display());
    }

    if !package_dir.is_dir() {
        anyhow::bail!("Path is not a directory: {}", package_dir.display());
    }

    // Check if package.json exists
    let package_json = package_dir.join("package.json");
    if !package_json.exists() {
        anyhow::bail!("package.json not found in: {}", package_dir.display());
    }

    // Validate script name
    if script_name.is_empty() {
        anyhow::bail!("Script name cannot be empty");
    }

    // Execute npm run with inherited stdin/stdout/stderr for full interactivity
    let status = Command::new("npm")
        .arg("run")
        .arg(script_name)
        .current_dir(package_dir)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .with_context(|| {
            format!(
                "Failed to execute npm script '{}' in directory '{}'",
                script_name,
                package_dir.display()
            )
        })?;

    Ok(status.code().unwrap_or(1))
}

/// Execute a Task (go-task) task interactively with full terminal access
pub fn execute_task_interactive(taskfile_path: &Path, task_name: &str) -> Result<i32> {
    if !taskfile_path.exists() {
        anyhow::bail!("Taskfile not found: {}", taskfile_path.display());
    }

    if !taskfile_path.is_file() {
        anyhow::bail!("Path is not a file: {}", taskfile_path.display());
    }

    if task_name.is_empty() {
        anyhow::bail!("Task name cannot be empty");
    }

    let dir = taskfile_path.parent().with_context(|| {
        format!(
            "Failed to get parent directory of: {}",
            taskfile_path.display()
        )
    })?;

    let status = Command::new("task")
        .arg("--taskfile")
        .arg(taskfile_path)
        .arg(task_name)
        .current_dir(dir)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .with_context(|| {
            format!(
                "Failed to execute task '{}' from {}",
                task_name,
                taskfile_path.display()
            )
        })?;

    Ok(status.code().unwrap_or(1))
}

/// Execute a devbox script interactively with full terminal access
pub fn execute_devbox_script_interactive(devbox_dir: &Path, script_name: &str) -> Result<i32> {
    // Validate inputs
    if !devbox_dir.exists() {
        anyhow::bail!("Directory not found: {}", devbox_dir.display());
    }

    if !devbox_dir.is_dir() {
        anyhow::bail!("Path is not a directory: {}", devbox_dir.display());
    }

    // Check if devbox.json exists
    let devbox_json = devbox_dir.join("devbox.json");
    if !devbox_json.exists() {
        anyhow::bail!("devbox.json not found in: {}", devbox_dir.display());
    }

    // Validate script name
    if script_name.is_empty() {
        anyhow::bail!("Script name cannot be empty");
    }

    // Execute devbox run with inherited stdin/stdout/stderr for full interactivity
    let status = Command::new("devbox")
        .arg("run")
        .arg(script_name)
        .current_dir(devbox_dir)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .with_context(|| {
            format!(
                "Failed to execute devbox script '{}' in directory '{}'",
                script_name,
                devbox_dir.display()
            )
        })?;

    Ok(status.code().unwrap_or(1))
}

/// Check if a string is a valid bash identifier
fn is_valid_bash_identifier(name: &str) -> bool {
    if name.is_empty() {
        return false;
    }

    // First character must be letter or underscore
    let first_char = name.chars().next().unwrap();
    if !first_char.is_ascii_alphabetic() && first_char != '_' {
        return false;
    }

    // Remaining characters must be alphanumeric or underscore
    name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_execute_function_interactive_success() {
        let temp_dir = TempDir::new().unwrap();
        let script_path = temp_dir.path().join("test.sh");

        let content = r#"#!/bin/bash
test_function() {
    echo "Test successful"
    exit 0
}
"#;
        fs::write(&script_path, content).unwrap();

        let result = execute_function_interactive(&script_path, "test_function");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0);
    }

    #[test]
    fn test_execute_function_interactive_failure() {
        let temp_dir = TempDir::new().unwrap();
        let script_path = temp_dir.path().join("test.sh");

        let content = r#"#!/bin/bash
failing_function() {
    echo "This will fail"
    exit 1
}
"#;
        fs::write(&script_path, content).unwrap();

        let result = execute_function_interactive(&script_path, "failing_function");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 1);
    }

    #[test]
    fn test_execute_function_interactive_nonexistent_file() {
        let temp_dir = TempDir::new().unwrap();
        let script_path = temp_dir.path().join("nonexistent.sh");

        let result = execute_function_interactive(&script_path, "any_function");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
    }

    #[test]
    fn test_execute_function_interactive_invalid_function_name() {
        let temp_dir = TempDir::new().unwrap();
        let script_path = temp_dir.path().join("test.sh");

        let content = "#!/bin/bash\necho 'test'";
        fs::write(&script_path, content).unwrap();

        // Test with invalid function name (starts with number)
        let result = execute_function_interactive(&script_path, "123invalid");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Invalid function name"));
    }

    #[test]
    fn test_execute_function_interactive_empty_function_name() {
        let temp_dir = TempDir::new().unwrap();
        let script_path = temp_dir.path().join("test.sh");

        let content = "#!/bin/bash\necho 'test'";
        fs::write(&script_path, content).unwrap();

        let result = execute_function_interactive(&script_path, "");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("cannot be empty"));
    }

    #[test]
    fn test_execute_function_interactive_directory_instead_of_file() {
        let temp_dir = TempDir::new().unwrap();

        let result = execute_function_interactive(temp_dir.path(), "any_function");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not a file"));
    }

    #[test]
    fn test_execute_function_interactive_undefined_function() {
        let temp_dir = TempDir::new().unwrap();
        let script_path = temp_dir.path().join("test.sh");

        let content = r#"#!/bin/bash
defined_function() {
    echo "I exist"
}
"#;
        fs::write(&script_path, content).unwrap();

        // Try to call a function that doesn't exist
        let result = execute_function_interactive(&script_path, "undefined_function");
        // Should fail because bash will error on undefined function
        assert!(result.is_ok()); // Command executes but should return non-zero
        assert_ne!(result.unwrap(), 0);
    }

    #[test]
    fn test_is_valid_bash_identifier_valid_names() {
        assert!(is_valid_bash_identifier("valid_name"));
        assert!(is_valid_bash_identifier("_underscore"));
        assert!(is_valid_bash_identifier("name123"));
        assert!(is_valid_bash_identifier("CamelCase"));
        assert!(is_valid_bash_identifier("UPPERCASE"));
        assert!(is_valid_bash_identifier("mixed_Case_123"));
    }

    #[test]
    fn test_is_valid_bash_identifier_invalid_names() {
        assert!(!is_valid_bash_identifier(""));
        assert!(!is_valid_bash_identifier("123start"));
        assert!(!is_valid_bash_identifier("has-dash"));
        assert!(!is_valid_bash_identifier("has space"));
        assert!(!is_valid_bash_identifier("has.dot"));
        assert!(!is_valid_bash_identifier("has$dollar"));
        assert!(!is_valid_bash_identifier("has@at"));
        assert!(!is_valid_bash_identifier("has!bang"));
    }

    #[test]
    fn test_execute_function_with_exit_code() {
        let temp_dir = TempDir::new().unwrap();
        let script_path = temp_dir.path().join("test.sh");

        let content = r#"#!/bin/bash
custom_exit() {
    exit 42
}
"#;
        fs::write(&script_path, content).unwrap();

        let result = execute_function_interactive(&script_path, "custom_exit");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
    }

    #[test]
    fn test_execute_npm_script_success() {
        let temp_dir = TempDir::new().unwrap();
        let package_json = temp_dir.path().join("package.json");

        let content = r#"{
  "name": "test-project",
  "scripts": {
    "test": "echo 'npm test success'"
  }
}"#;
        fs::write(&package_json, content).unwrap();

        let result = execute_npm_script_interactive(temp_dir.path(), "test");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0);
    }

    #[test]
    fn test_execute_npm_script_nonexistent_directory() {
        let temp_dir = TempDir::new().unwrap();
        let nonexistent = temp_dir.path().join("nonexistent");

        let result = execute_npm_script_interactive(&nonexistent, "test");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
    }

    #[test]
    fn test_execute_npm_script_directory_is_file() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("file.txt");
        fs::write(&file_path, "content").unwrap();

        let result = execute_npm_script_interactive(&file_path, "test");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not a directory"));
    }

    #[test]
    fn test_execute_npm_script_no_package_json() {
        let temp_dir = TempDir::new().unwrap();

        let result = execute_npm_script_interactive(temp_dir.path(), "test");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("package.json not found"));
    }

    #[test]
    fn test_execute_npm_script_empty_name() {
        let temp_dir = TempDir::new().unwrap();
        let package_json = temp_dir.path().join("package.json");

        let content = r#"{
  "name": "test-project",
  "scripts": {
    "test": "echo 'test'"
  }
}"#;
        fs::write(&package_json, content).unwrap();

        let result = execute_npm_script_interactive(temp_dir.path(), "");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("cannot be empty"));
    }

    #[test]
    fn test_execute_npm_script_nonexistent_script() {
        let temp_dir = TempDir::new().unwrap();
        let package_json = temp_dir.path().join("package.json");

        let content = r#"{
  "name": "test-project",
  "scripts": {
    "test": "echo 'test'"
  }
}"#;
        fs::write(&package_json, content).unwrap();

        // Try to run a script that doesn't exist - npm will return non-zero exit code
        let result = execute_npm_script_interactive(temp_dir.path(), "nonexistent");
        // npm run will execute but return non-zero
        assert!(result.is_ok());
        assert_ne!(result.unwrap(), 0);
    }

    #[test]
    fn test_execute_devbox_script_success() {
        let temp_dir = TempDir::new().unwrap();
        let devbox_json = temp_dir.path().join("devbox.json");

        let content = r#"{
  "shell": {
    "scripts": {
      "test": ["echo 'devbox test success'"]
    }
  }
}"#;
        fs::write(&devbox_json, content).unwrap();

        // Note: This test will only pass if devbox is installed
        // In CI/CD, devbox should be available
        let result = execute_devbox_script_interactive(temp_dir.path(), "test");
        // We expect this to succeed if devbox is installed
        if result.is_ok() {
            assert_eq!(result.unwrap(), 0);
        }
    }

    #[test]
    fn test_execute_devbox_script_nonexistent_directory() {
        let temp_dir = TempDir::new().unwrap();
        let nonexistent = temp_dir.path().join("nonexistent");

        let result = execute_devbox_script_interactive(&nonexistent, "test");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
    }

    #[test]
    fn test_execute_devbox_script_directory_is_file() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("file.txt");
        fs::write(&file_path, "content").unwrap();

        let result = execute_devbox_script_interactive(&file_path, "test");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not a directory"));
    }

    #[test]
    fn test_execute_devbox_script_no_devbox_json() {
        let temp_dir = TempDir::new().unwrap();

        let result = execute_devbox_script_interactive(temp_dir.path(), "test");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("devbox.json not found"));
    }

    #[test]
    fn test_execute_devbox_script_empty_name() {
        let temp_dir = TempDir::new().unwrap();
        let devbox_json = temp_dir.path().join("devbox.json");

        let content = r#"{
  "shell": {
    "scripts": {
      "test": ["echo 'test'"]
    }
  }
}"#;
        fs::write(&devbox_json, content).unwrap();

        let result = execute_devbox_script_interactive(temp_dir.path(), "");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("cannot be empty"));
    }

    #[test]
    fn test_execute_task_interactive_nonexistent_path() {
        let temp_dir = TempDir::new().unwrap();
        let taskfile = temp_dir.path().join("Taskfile.yml");

        let result = execute_task_interactive(&taskfile, "build");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
    }

    #[test]
    fn test_execute_task_interactive_empty_name() {
        let temp_dir = TempDir::new().unwrap();
        let taskfile = temp_dir.path().join("Taskfile.yml");
        fs::write(&taskfile, "version: '3'\ntasks: {}").unwrap();

        let result = execute_task_interactive(&taskfile, "");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("cannot be empty"));
    }
}
