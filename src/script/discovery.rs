use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::OnceLock;
use walkdir::WalkDir;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ScriptType {
    Bash,
    PackageJson,
    DevboxJson,
    Task,
}

#[derive(Debug, Clone)]
pub struct ScriptFile {
    pub path: PathBuf,
    #[allow(dead_code)]
    pub name: String,
    pub category: String,
    pub display_name: String,
    pub script_type: ScriptType,
}

/// Taskfile names to detect (all variants from taskfile.dev)
const TASKFILE_NAMES: &[&str] = &[
    "taskfile.dist.yaml",
    "Taskfile.dist.yaml",
    "taskfile.dist.yml",
    "Taskfile.dist.yml",
    "taskfile.yaml",
    "Taskfile.yaml",
    "taskfile.yml",
    "Taskfile.yml",
];

/// Cache for devbox availability check (checked once per process)
static DEVBOX_AVAILABLE: OnceLock<bool> = OnceLock::new();

/// Check if devbox is installed and available in PATH
fn is_devbox_available() -> bool {
    *DEVBOX_AVAILABLE.get_or_init(|| {
        Command::new("devbox")
            .arg("version")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|status| status.success())
            .unwrap_or(false)
    })
}

/// Formats a filename into a display-friendly name
/// - Replaces underscores and hyphens with spaces
/// - Capitalizes the first letter of each word
///
/// Examples:
///   - "example" -> "Example"
///   - "example_file" -> "Example File"
///   - "example-file" -> "Example File"
///   - "🏠 homelab" -> "🏠 Homelab"
pub fn format_display_name(name: &str) -> String {
    name.replace(['_', '-'], " ")
        .split_whitespace()
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => {
                    let mut result = first.to_uppercase().to_string();
                    result.push_str(&chars.as_str().to_lowercase());
                    result
                }
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

pub fn discover_scripts(scripts_dir: &Path) -> Result<Vec<ScriptFile>> {
    discover_scripts_with_depth(scripts_dir, 2)
}

pub fn discover_scripts_shallow(scripts_dir: &Path) -> Result<Vec<ScriptFile>> {
    discover_scripts_with_depth(scripts_dir, 1)
}

fn discover_scripts_with_depth(scripts_dir: &Path, max_depth: usize) -> Result<Vec<ScriptFile>> {
    let mut scripts = Vec::new();

    // Verify the directory exists and is readable
    if !scripts_dir.exists() {
        return Ok(scripts); // Return empty vec if directory doesn't exist
    }

    if !scripts_dir.is_dir() {
        anyhow::bail!(
            "Path '{}' exists but is not a directory",
            scripts_dir.display()
        );
    }

    // Walk the directory and collect scripts
    for entry in WalkDir::new(scripts_dir)
        .max_depth(max_depth)
        .into_iter()
        .filter_map(|e| match e {
            Ok(entry) => Some(entry),
            Err(err) => {
                eprintln!("Warning: Failed to read directory entry: {}", err);
                None
            }
        })
    {
        let path = entry.path();

        // Skip if not a file
        if !path.is_file() {
            continue;
        }

        // Check filename for package.json or devbox.json
        if let Some(filename) = path.file_name().and_then(|f| f.to_str()) {
            if filename == "package.json" {
                // Extract parent directory name for category
                let name = if let Some(parent) = path.parent() {
                    parent
                        .file_name()
                        .and_then(|s| s.to_str())
                        .unwrap_or("node")
                        .to_string()
                } else {
                    "node".to_string()
                };

                let category = name.clone();
                let display_name = format_display_name(&name);

                scripts.push(ScriptFile {
                    path: path.to_path_buf(),
                    name,
                    category,
                    display_name,
                    script_type: ScriptType::PackageJson,
                });
                continue;
            }

            if filename == "devbox.json" {
                // Skip devbox.json if devbox is not installed
                if !is_devbox_available() {
                    continue;
                }

                // Extract parent directory name for category
                let name = if let Some(parent) = path.parent() {
                    parent
                        .file_name()
                        .and_then(|s| s.to_str())
                        .unwrap_or("devbox")
                        .to_string()
                } else {
                    "devbox".to_string()
                };

                let category = name.clone();
                let display_name = format_display_name(&name);

                scripts.push(ScriptFile {
                    path: path.to_path_buf(),
                    name,
                    category,
                    display_name,
                    script_type: ScriptType::DevboxJson,
                });
                continue;
            }

            if TASKFILE_NAMES.contains(&filename) {
                if !crate::script::task_parser::is_task_available() {
                    continue;
                }

                let name = if let Some(parent) = path.parent() {
                    parent
                        .file_name()
                        .and_then(|s| s.to_str())
                        .unwrap_or("task")
                        .to_string()
                } else {
                    "task".to_string()
                };

                let category = name.clone();
                let display_name = format!("📋 {}", format_display_name(&name));

                scripts.push(ScriptFile {
                    path: path.to_path_buf(),
                    name,
                    category,
                    display_name,
                    script_type: ScriptType::Task,
                });
                continue;
            }
        }

        // Check file extension for .sh files
        let extension = match path.extension() {
            Some(ext) => ext,
            None => continue,
        };

        if extension != "sh" {
            continue;
        }

        // Extract filename (without extension) to use as the category
        let name = path
            .file_stem()
            .and_then(|s| s.to_str())
            .with_context(|| format!("Invalid filename for script: {}", path.display()))?
            .to_string();

        // Use the filename as category (key)
        // Format the display name for UI presentation
        let category = name.clone();
        let display_name = format_display_name(&name);

        scripts.push(ScriptFile {
            path: path.to_path_buf(),
            name,
            category,
            display_name,
            script_type: ScriptType::Bash,
        });
    }

    Ok(scripts)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_discover_scripts_empty_directory() {
        let temp_dir = TempDir::new().unwrap();
        let result = discover_scripts(temp_dir.path()).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn test_discover_scripts_nonexistent_directory() {
        let temp_dir = TempDir::new().unwrap();
        let non_existent = temp_dir.path().join("nonexistent");
        let result = discover_scripts(&non_existent).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn test_discover_scripts_single_script() {
        let temp_dir = TempDir::new().unwrap();
        let script_path = temp_dir.path().join("test.sh");
        fs::write(&script_path, "#!/bin/bash\necho 'test'").unwrap();

        let result = discover_scripts(temp_dir.path()).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].name, "test");
        assert_eq!(result[0].category, "test");
        assert_eq!(result[0].display_name, "Test");
        assert_eq!(result[0].script_type, ScriptType::Bash);
    }

    #[test]
    fn test_discover_scripts_categorization() {
        let temp_dir = TempDir::new().unwrap();

        // Create scripts with different names
        fs::write(temp_dir.path().join("fedora.sh"), "#!/bin/bash").unwrap();
        fs::write(temp_dir.path().join("homelab.sh"), "#!/bin/bash").unwrap();
        fs::write(temp_dir.path().join("util.sh"), "#!/bin/bash").unwrap();
        fs::write(temp_dir.path().join("custom.sh"), "#!/bin/bash").unwrap();

        let result = discover_scripts(temp_dir.path()).unwrap();
        assert_eq!(result.len(), 4);

        // Check categorization - uses filename as category, formatted as display name
        let fedora = result.iter().find(|s| s.name == "fedora").unwrap();
        assert_eq!(fedora.category, "fedora");
        assert_eq!(fedora.display_name, "Fedora");

        let homelab = result.iter().find(|s| s.name == "homelab").unwrap();
        assert_eq!(homelab.category, "homelab");
        assert_eq!(homelab.display_name, "Homelab");

        let util = result.iter().find(|s| s.name == "util").unwrap();
        assert_eq!(util.category, "util");
        assert_eq!(util.display_name, "Util");

        let custom = result.iter().find(|s| s.name == "custom").unwrap();
        assert_eq!(custom.category, "custom");
        assert_eq!(custom.display_name, "Custom");
    }

    #[test]
    fn test_discover_scripts_ignores_non_sh_files() {
        let temp_dir = TempDir::new().unwrap();

        fs::write(temp_dir.path().join("script.sh"), "#!/bin/bash").unwrap();
        fs::write(temp_dir.path().join("readme.txt"), "text file").unwrap();
        fs::write(temp_dir.path().join("data.json"), "{}").unwrap();

        let result = discover_scripts(temp_dir.path()).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].name, "script");
        assert_eq!(result[0].display_name, "Script");
    }

    #[test]
    fn test_discover_scripts_subdirectories() {
        let temp_dir = TempDir::new().unwrap();

        // Create script in root
        fs::write(temp_dir.path().join("root.sh"), "#!/bin/bash").unwrap();

        // Create subdirectory with script
        let sub_dir = temp_dir.path().join("subdir");
        fs::create_dir(&sub_dir).unwrap();
        fs::write(sub_dir.join("sub.sh"), "#!/bin/bash").unwrap();

        let result = discover_scripts(temp_dir.path()).unwrap();
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn test_discover_scripts_file_instead_of_directory() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("file.txt");
        fs::write(&file_path, "content").unwrap();

        let result = discover_scripts(&file_path);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not a directory"));
    }

    #[test]
    fn test_discover_scripts_with_emoji_filenames() {
        let temp_dir = TempDir::new().unwrap();

        // Create scripts with emoji in filenames
        fs::write(temp_dir.path().join("🏠 homelab.sh"), "#!/bin/bash").unwrap();
        fs::write(temp_dir.path().join("🛠️ utilities.sh"), "#!/bin/bash").unwrap();
        fs::write(temp_dir.path().join("🖥️ system.sh"), "#!/bin/bash").unwrap();

        let result = discover_scripts(temp_dir.path()).unwrap();
        assert_eq!(result.len(), 3);

        // Check that emoji is preserved and text is capitalized
        let homelab = result.iter().find(|s| s.name == "🏠 homelab").unwrap();
        assert_eq!(homelab.category, "🏠 homelab");
        assert_eq!(homelab.display_name, "🏠 Homelab");

        let utilities = result.iter().find(|s| s.name == "🛠️ utilities").unwrap();
        assert_eq!(utilities.category, "🛠️ utilities");
        assert_eq!(utilities.display_name, "🛠️ Utilities");

        let system = result.iter().find(|s| s.name == "🖥️ system").unwrap();
        assert_eq!(system.category, "🖥️ system");
        assert_eq!(system.display_name, "🖥️ System");
    }

    #[test]
    fn test_discover_scripts_with_underscores_and_hyphens() {
        let temp_dir = TempDir::new().unwrap();

        // Create scripts with underscores and hyphens
        fs::write(temp_dir.path().join("example_file.sh"), "#!/bin/bash").unwrap();
        fs::write(temp_dir.path().join("example-file.sh"), "#!/bin/bash").unwrap();
        fs::write(temp_dir.path().join("my_test_script.sh"), "#!/bin/bash").unwrap();
        fs::write(
            temp_dir.path().join("another-test-script.sh"),
            "#!/bin/bash",
        )
        .unwrap();

        let result = discover_scripts(temp_dir.path()).unwrap();
        assert_eq!(result.len(), 4);

        // Check underscore handling
        let example_underscore = result.iter().find(|s| s.name == "example_file").unwrap();
        assert_eq!(example_underscore.category, "example_file");
        assert_eq!(example_underscore.display_name, "Example File");

        // Check hyphen handling
        let example_hyphen = result.iter().find(|s| s.name == "example-file").unwrap();
        assert_eq!(example_hyphen.category, "example-file");
        assert_eq!(example_hyphen.display_name, "Example File");

        // Check multiple underscores
        let my_test = result.iter().find(|s| s.name == "my_test_script").unwrap();
        assert_eq!(my_test.category, "my_test_script");
        assert_eq!(my_test.display_name, "My Test Script");

        // Check multiple hyphens
        let another_test = result
            .iter()
            .find(|s| s.name == "another-test-script")
            .unwrap();
        assert_eq!(another_test.category, "another-test-script");
        assert_eq!(another_test.display_name, "Another Test Script");
    }

    #[test]
    fn test_format_display_name() {
        assert_eq!(format_display_name("example"), "Example");
        assert_eq!(format_display_name("example_file"), "Example File");
        assert_eq!(format_display_name("example-file"), "Example File");
        assert_eq!(format_display_name("my_test_script"), "My Test Script");
        assert_eq!(
            format_display_name("another-test-script"),
            "Another Test Script"
        );
        assert_eq!(format_display_name("🏠 homelab"), "🏠 Homelab");
        assert_eq!(format_display_name("🛠️ utilities"), "🛠️ Utilities");
        assert_eq!(format_display_name("UPPERCASE"), "Uppercase");
        assert_eq!(
            format_display_name("mixed_CASE-example"),
            "Mixed Case Example"
        );
    }

    #[test]
    fn test_discover_package_json() {
        let temp_dir = TempDir::new().unwrap();
        let package_path = temp_dir.path().join("package.json");

        let content = r#"{
  "name": "test-project",
  "scripts": {
    "test": "jest"
  }
}"#;
        fs::write(&package_path, content).unwrap();

        let result = discover_scripts(temp_dir.path()).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].script_type, ScriptType::PackageJson);
        // Category should be the parent directory name
        assert!(result[0].category.len() > 0);
    }

    #[test]
    fn test_discover_mixed_scripts() {
        let temp_dir = TempDir::new().unwrap();

        // Create bash script
        fs::write(temp_dir.path().join("script.sh"), "#!/bin/bash").unwrap();

        // Create package.json
        let content = r#"{"name": "test", "scripts": {"test": "jest"}}"#;
        fs::write(temp_dir.path().join("package.json"), content).unwrap();

        let result = discover_scripts(temp_dir.path()).unwrap();
        assert_eq!(result.len(), 2);

        // Should find both types
        let bash_count = result
            .iter()
            .filter(|s| s.script_type == ScriptType::Bash)
            .count();
        let npm_count = result
            .iter()
            .filter(|s| s.script_type == ScriptType::PackageJson)
            .count();

        assert_eq!(bash_count, 1);
        assert_eq!(npm_count, 1);
    }

    #[test]
    fn test_discover_devbox_json() {
        let temp_dir = TempDir::new().unwrap();
        let devbox_path = temp_dir.path().join("devbox.json");

        let content = r#"{
  "shell": {
    "scripts": {
      "test": ["cargo test"]
    }
  }
}"#;
        fs::write(&devbox_path, content).unwrap();

        let result = discover_scripts(temp_dir.path()).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].script_type, ScriptType::DevboxJson);
        // Category should be the parent directory name
        assert!(result[0].category.len() > 0);
    }

    #[test]
    fn test_discover_all_script_types() {
        let temp_dir = TempDir::new().unwrap();

        // Create bash script
        fs::write(temp_dir.path().join("script.sh"), "#!/bin/bash").unwrap();

        // Create package.json
        let npm_content = r#"{"name": "test", "scripts": {"test": "jest"}}"#;
        fs::write(temp_dir.path().join("package.json"), npm_content).unwrap();

        // Create devbox.json
        let devbox_content = r#"{
  "shell": {
    "scripts": {
      "build": ["cargo build"]
    }
  }
}"#;
        fs::write(temp_dir.path().join("devbox.json"), devbox_content).unwrap();

        let result = discover_scripts(temp_dir.path()).unwrap();
        assert_eq!(result.len(), 3);

        // Should find all three types
        let bash_count = result
            .iter()
            .filter(|s| s.script_type == ScriptType::Bash)
            .count();
        let npm_count = result
            .iter()
            .filter(|s| s.script_type == ScriptType::PackageJson)
            .count();
        let devbox_count = result
            .iter()
            .filter(|s| s.script_type == ScriptType::DevboxJson)
            .count();

        assert_eq!(bash_count, 1);
        assert_eq!(npm_count, 1);
        assert_eq!(devbox_count, 1);
    }

    #[test]
    fn test_discover_taskfile() {
        let temp_dir = TempDir::new().unwrap();
        let taskfile_path = temp_dir.path().join("Taskfile.yml");

        let content = r#"version: '3'
tasks:
  default:
    desc: Default task
    cmds: [echo hello]
"#;
        fs::write(&taskfile_path, content).unwrap();

        let result = discover_scripts(temp_dir.path()).unwrap();
        // If task binary is installed we get 1 ScriptFile with Task type, else 0
        let task_files: Vec<_> = result
            .iter()
            .filter(|s| s.script_type == ScriptType::Task)
            .collect();
        assert!(
            task_files.len() <= 1,
            "should have at most one Task script file"
        );
        if let Some(sf) = task_files.first() {
            assert_eq!(sf.script_type, ScriptType::Task);
            assert!(sf.path.ends_with("Taskfile.yml"));
        }
    }
}
