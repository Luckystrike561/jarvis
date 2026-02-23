//! # Script Discovery
//!
//! This module handles automatic discovery of script files in the project directory.
//!
//! ## Supported Script Types
//!
//! - **Bash scripts** (`.sh` files) - Functions are extracted by the parser
//! - **npm scripts** (`package.json`) - Scripts from the "scripts" section
//! - **Devbox scripts** (`devbox.json`) - Scripts from the "shell.scripts" section
//! - **Taskfiles** (`Taskfile.yml`, etc.) - Tasks defined in go-task format
//! - **Makefiles** (`Makefile`, etc.) - Targets defined in GNU Make format
//! - **Justfiles** (`justfile`, etc.) - Recipes defined in just format
//! - **Terraform / `OpenTofu`** (`*.tf`) — Terraform infrastructure-as-code commands
//! - **Bazel** (`WORKSPACE`, `BUILD`, `MODULE.bazel`) — Bazel build targets
//!
//! ## Discovery Locations
//!
//! Scripts are discovered from multiple locations:
//!
//! | Location | Depth | Description |
//! |----------|-------|-------------|
//! | `./` | 1 | Root directory (shallow scan) |
//! | `./script/` | 2 | Script subdirectory |
//! | `./scripts/` | 2 | Scripts subdirectory |
//! | `./jarvis/` | 2 | Jarvis-specific directory |
//!
//! ## Category Assignment
//!
//! Each discovered script is assigned a category based on its source:
//! - Root scripts use the filename (without extension) as category
//! - Subdirectory scripts use the subdirectory name as category
//!
//! ## Key Functions
//!
//! - [`discover_scripts`] - Full recursive discovery with depth 2
//! - [`discover_scripts_shallow`] - Shallow discovery with depth 1
//! - [`format_display_name`] - Converts `snake_case` to Title Case

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::OnceLock;
use walkdir::WalkDir;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum ScriptType {
    Bazel,
    Bash,
    CargoToml,
    DevboxJson,
    Gradle,
    Just,
    Makefile,
    NxJson,
    PackageJson,
    Task,
    Terraform,
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

/// Makefile names to detect
const MAKEFILE_NAMES: &[&str] = &["Makefile", "makefile", "GNUmakefile"];

/// Justfile names to detect
const JUSTFILE_NAMES: &[&str] = &["justfile", "Justfile", ".justfile"];

/// Cargo manifest names to detect
const CARGO_TOML_NAMES: &[&str] = &["Cargo.toml"];

/// Nx workspace config names to detect
const NX_JSON_NAMES: &[&str] = &["nx.json"];

/// Gradle build file names to detect
const GRADLE_BUILD_NAMES: &[&str] = &["build.gradle", "build.gradle.kts"];

/// Gradle settings file names to detect
const GRADLE_SETTINGS_NAMES: &[&str] = &["settings.gradle", "settings.gradle.kts"];

/// Bazel workspace config names to detect
const BAZEL_NAMES: &[&str] = &[
    "BUILD",
    "BUILD.bazel",
    "MODULE.bazel",
    "WORKSPACE",
    "WORKSPACE.bazel",
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

/// Pre-warm all tool availability checks in parallel.
///
/// This spawns threads to check each tool concurrently, so by the time
/// discovery needs the results, they're already cached in the `OnceLock` statics.
pub fn prewarm_tool_checks() {
    std::thread::spawn(is_devbox_available);
    std::thread::spawn(crate::script::bazel_parser::is_bazel_available);
    std::thread::spawn(crate::script::task_parser::is_task_available);
    std::thread::spawn(crate::script::makefile_parser::is_make_available);
    std::thread::spawn(crate::script::just_parser::is_just_available);
    std::thread::spawn(crate::script::cargo_parser::is_cargo_available);
    std::thread::spawn(crate::script::nx_parser::is_nx_available);
    std::thread::spawn(crate::script::terraform_parser::is_terraform_available);
    std::thread::spawn(crate::script::gradle_parser::is_gradle_available);
}

/// Formats a filename into a display-friendly name
/// - Replaces underscores and hyphens with spaces
/// - Capitalizes the first letter of each word
///
/// Examples:
///   - "example" -> "Example"
///   - "`example_file`" -> "Example File"
///   - "example-file" -> "Example File"
///   - "service.auth" -> "Service Auth"
///   - "🏠 homelab" -> "🏠 Homelab"
pub fn format_display_name(name: &str) -> String {
    name.replace(['_', '-', '.'], " ")
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

/// Discover a single script file and return its `ScriptFile` representation.
///
/// This function determines the script type from the file extension/name:
/// - `.sh` files → Bash
/// - `.tf` files → Terraform
/// - `package.json` → `PackageJson`
/// - `devbox.json` → `DevboxJson`
/// - `Taskfile.yml` (and variants) → Task
/// - `Makefile` (and variants) → Makefile
/// - `justfile` (and variants) → Just
/// - `Cargo.toml` → `CargoToml`
/// - `nx.json` → `NxJson`
/// - `build.gradle` / `build.gradle.kts` → Gradle
/// - `WORKSPACE` / `BUILD` / `MODULE.bazel` → Bazel
///
/// # Arguments
///
/// * `file_path` - Path to the script file
///
/// # Returns
///
/// * `Ok(ScriptFile)` - Successfully identified script file
/// * `Err` - File doesn't exist, is not a file, or is an unsupported type
///
/// # Example
///
/// ```ignore
/// let script = discover_single_file(Path::new("./deploy.sh"))?;
/// println!("Found: {} ({})", script.display_name, script.script_type);
/// ```
pub fn discover_single_file(file_path: &Path) -> Result<ScriptFile> {
    // Verify the file exists
    if !file_path.exists() {
        anyhow::bail!("File '{}' does not exist", file_path.display());
    }

    // Verify it's a file, not a directory
    if !file_path.is_file() {
        anyhow::bail!("Path '{}' is not a file", file_path.display());
    }

    // Get the filename
    let filename = file_path
        .file_name()
        .and_then(|f| f.to_str())
        .with_context(|| format!("Invalid filename: {}", file_path.display()))?;

    // Determine script type based on filename/extension
    let script_type = determine_script_type(filename, file_path)?;

    // Get the file stem (name without extension) for category/display name
    let name = match script_type {
        ScriptType::Bazel
        | ScriptType::PackageJson
        | ScriptType::DevboxJson
        | ScriptType::Task
        | ScriptType::Makefile
        | ScriptType::Just
        | ScriptType::CargoToml
        | ScriptType::NxJson
        | ScriptType::Terraform
        | ScriptType::Gradle => {
            // For JSON/YAML config files and Makefile, use the parent directory name or the filename
            if let Some(parent) = file_path.parent() {
                parent
                    .file_name()
                    .and_then(|s| s.to_str())
                    .unwrap_or(filename)
                    .to_string()
            } else {
                filename.to_string()
            }
        }
        ScriptType::Bash => {
            // For .sh files, use the file stem
            file_path
                .file_stem()
                .and_then(|s| s.to_str())
                .with_context(|| format!("Invalid filename: {}", file_path.display()))?
                .to_string()
        }
    };

    let category = name.clone();
    let display_name = match script_type {
        ScriptType::Bazel => format!("🌿 {}", format_display_name(&name)),
        ScriptType::Task => format!("📋 {}", format_display_name(&name)),
        ScriptType::Makefile => format!("🔨 {}", format_display_name(&name)),
        ScriptType::Just => format!("⚡ {}", format_display_name(&name)),
        ScriptType::CargoToml => format!("🦀 {}", format_display_name(&name)),
        ScriptType::NxJson => format!("🔷 {}", format_display_name(&name)),
        ScriptType::Terraform => format!("🏗️ {}", format_display_name(&name)),
        ScriptType::Gradle => format!("🐘 {}", format_display_name(&name)),
        _ => format_display_name(&name),
    };

    Ok(ScriptFile {
        path: file_path.to_path_buf(),
        name,
        category,
        display_name,
        script_type,
    })
}

/// Determine the script type from the filename
fn determine_script_type(filename: &str, file_path: &Path) -> Result<ScriptType> {
    // Check for specific filenames first
    if filename == "package.json" {
        return Ok(ScriptType::PackageJson);
    }

    if filename == "devbox.json" {
        if !is_devbox_available() {
            anyhow::bail!(
                "devbox.json found but 'devbox' is not installed or not in PATH. \
                Please install devbox to use this file."
            );
        }
        return Ok(ScriptType::DevboxJson);
    }

    if TASKFILE_NAMES.contains(&filename) {
        if !crate::script::task_parser::is_task_available() {
            anyhow::bail!(
                "Taskfile found but 'task' is not installed or not in PATH. \
                Please install go-task to use this file."
            );
        }
        return Ok(ScriptType::Task);
    }

    if MAKEFILE_NAMES.contains(&filename) {
        if !crate::script::makefile_parser::is_make_available() {
            anyhow::bail!(
                "Makefile found but 'make' is not installed or not in PATH. \
                Please install make to use this file."
            );
        }
        return Ok(ScriptType::Makefile);
    }

    if JUSTFILE_NAMES.contains(&filename) {
        if !crate::script::just_parser::is_just_available() {
            anyhow::bail!(
                "Justfile found but 'just' is not installed or not in PATH. \
                Please install just to use this file."
            );
        }
        return Ok(ScriptType::Just);
    }

    if CARGO_TOML_NAMES.contains(&filename) {
        if !crate::script::cargo_parser::is_cargo_available() {
            anyhow::bail!(
                "Cargo.toml found but 'cargo' is not installed or not in PATH. \
                Please install Rust/Cargo to use this file."
            );
        }
        return Ok(ScriptType::CargoToml);
    }

    if NX_JSON_NAMES.contains(&filename) {
        if !crate::script::nx_parser::is_nx_available() {
            anyhow::bail!(
                "nx.json found but 'nx' is not installed or not in PATH. \
                Please install Nx to use this file."
            );
        }
        return Ok(ScriptType::NxJson);
    }

    if GRADLE_BUILD_NAMES.contains(&filename) || GRADLE_SETTINGS_NAMES.contains(&filename) {
        if !crate::script::gradle_parser::is_gradle_available() {
            anyhow::bail!(
                "Gradle build file found but neither Gradle wrapper (gradlew) nor system Gradle is available. \
                 Please install Gradle or add a Gradle wrapper to use this file."
            );
        }
        return Ok(ScriptType::Gradle);
    }

    if BAZEL_NAMES.contains(&filename) {
        if !crate::script::bazel_parser::is_bazel_available() {
            anyhow::bail!(
                "Bazel workspace file found but neither 'bazel' nor 'bazelisk' is installed or in PATH. \
                 Please install Bazel or Bazelisk to use this file."
            );
        }
        return Ok(ScriptType::Bazel);
    }

    // Check file extension for .sh files
    if let Some(ext) = file_path.extension() {
        if ext == "sh" {
            return Ok(ScriptType::Bash);
        }
        if ext == "tf" {
            if !crate::script::terraform_parser::is_terraform_available() {
                anyhow::bail!(
                    "Terraform file found but neither 'terraform' nor 'tofu' (OpenTofu) is installed or in PATH. \
                    Please install Terraform or OpenTofu to use this file."
                );
            }
            return Ok(ScriptType::Terraform);
        }
    }

    // Unsupported file type
    anyhow::bail!(
        "Unsupported file type: '{}'. \
        Supported types: .sh (bash), .tf (terraform), package.json (npm), devbox.json (devbox), \
        Taskfile.yml (task), Makefile (make), justfile (just), Cargo.toml (cargo), nx.json (nx), \
        build.gradle (gradle), WORKSPACE/BUILD (bazel)",
        filename
    );
}

fn discover_scripts_with_depth(scripts_dir: &Path, max_depth: usize) -> Result<Vec<ScriptFile>> {
    let mut scripts = Vec::new();

    // Track directories that already have a Terraform ScriptFile registered.
    // Multiple .tf files in the same directory should produce only one entry.
    let mut terraform_dirs: std::collections::HashSet<PathBuf> = std::collections::HashSet::new();

    // Track directories that already have a Gradle ScriptFile registered.
    // Multiple gradle files (build.gradle, settings.gradle, etc.) in the same directory should produce only one entry.
    let mut gradle_dirs: std::collections::HashSet<PathBuf> = std::collections::HashSet::new();

    // Track directories that already have a Bazel ScriptFile registered.
    // Multiple bazel files (WORKSPACE, BUILD, MODULE.bazel, etc.) in the same directory should produce only one entry.
    let mut bazel_dirs: std::collections::HashSet<PathBuf> = std::collections::HashSet::new();

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

        // Check filename for package.json, devbox.json, Taskfile, or Makefile
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

            if MAKEFILE_NAMES.contains(&filename) {
                if !crate::script::makefile_parser::is_make_available() {
                    continue;
                }

                let name = if let Some(parent) = path.parent() {
                    parent
                        .file_name()
                        .and_then(|s| s.to_str())
                        .unwrap_or("make")
                        .to_string()
                } else {
                    "make".to_string()
                };

                let category = name.clone();
                let display_name = format!("🔨 {}", format_display_name(&name));

                scripts.push(ScriptFile {
                    path: path.to_path_buf(),
                    name,
                    category,
                    display_name,
                    script_type: ScriptType::Makefile,
                });
                continue;
            }

            if JUSTFILE_NAMES.contains(&filename) {
                if !crate::script::just_parser::is_just_available() {
                    continue;
                }

                let name = if let Some(parent) = path.parent() {
                    parent
                        .file_name()
                        .and_then(|s| s.to_str())
                        .unwrap_or("just")
                        .to_string()
                } else {
                    "just".to_string()
                };

                let category = name.clone();
                let display_name = format!("⚡ {}", format_display_name(&name));

                scripts.push(ScriptFile {
                    path: path.to_path_buf(),
                    name,
                    category,
                    display_name,
                    script_type: ScriptType::Just,
                });
                continue;
            }

            if CARGO_TOML_NAMES.contains(&filename) {
                if !crate::script::cargo_parser::is_cargo_available() {
                    continue;
                }

                let name = if let Some(parent) = path.parent() {
                    parent
                        .file_name()
                        .and_then(|s| s.to_str())
                        .unwrap_or("cargo")
                        .to_string()
                } else {
                    "cargo".to_string()
                };

                let category = name.clone();
                let display_name = format!("🦀 {}", format_display_name(&name));

                scripts.push(ScriptFile {
                    path: path.to_path_buf(),
                    name,
                    category,
                    display_name,
                    script_type: ScriptType::CargoToml,
                });
                continue;
            }

            if NX_JSON_NAMES.contains(&filename) {
                if !crate::script::nx_parser::is_nx_available() {
                    continue;
                }

                let name = if let Some(parent) = path.parent() {
                    parent
                        .file_name()
                        .and_then(|s| s.to_str())
                        .unwrap_or("nx")
                        .to_string()
                } else {
                    "nx".to_string()
                };

                let category = name.clone();
                let display_name = format!("🔷 {}", format_display_name(&name));

                scripts.push(ScriptFile {
                    path: path.to_path_buf(),
                    name,
                    category,
                    display_name,
                    script_type: ScriptType::NxJson,
                });
                continue;
            }

            if GRADLE_BUILD_NAMES.contains(&filename) || GRADLE_SETTINGS_NAMES.contains(&filename) {
                if !crate::script::gradle_parser::is_gradle_available() {
                    continue;
                }

                // Only register one ScriptFile per gradle directory
                let gradle_dir = path.parent().unwrap_or(scripts_dir).to_path_buf();

                if gradle_dirs.contains(&gradle_dir) {
                    continue;
                }
                gradle_dirs.insert(gradle_dir.clone());

                let name = if let Some(parent) = path.parent() {
                    parent
                        .file_name()
                        .and_then(|s| s.to_str())
                        .unwrap_or("gradle")
                        .to_string()
                } else {
                    "gradle".to_string()
                };

                let category = name.clone();
                let display_name = format!("🐘 {}", format_display_name(&name));

                scripts.push(ScriptFile {
                    path: gradle_dir,
                    name,
                    category,
                    display_name,
                    script_type: ScriptType::Gradle,
                });
                continue;
            }

            if BAZEL_NAMES.contains(&filename) {
                if !crate::script::bazel_parser::is_bazel_available() {
                    continue;
                }

                // Only register one ScriptFile per bazel directory
                let bazel_dir = path.parent().unwrap_or(scripts_dir).to_path_buf();

                if bazel_dirs.contains(&bazel_dir) {
                    continue;
                }
                bazel_dirs.insert(bazel_dir.clone());

                let name = if let Some(parent) = path.parent() {
                    parent
                        .file_name()
                        .and_then(|s| s.to_str())
                        .unwrap_or("bazel")
                        .to_string()
                } else {
                    "bazel".to_string()
                };

                let category = name.clone();
                let display_name = format!("🌿 {}", format_display_name(&name));

                scripts.push(ScriptFile {
                    path: bazel_dir,
                    name,
                    category,
                    display_name,
                    script_type: ScriptType::Bazel,
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
            // Check for Terraform .tf files
            if extension == "tf" {
                if !crate::script::terraform_parser::is_terraform_available() {
                    continue;
                }

                // Only register one ScriptFile per directory
                let tf_dir = path.parent().unwrap_or(scripts_dir).to_path_buf();

                if terraform_dirs.contains(&tf_dir) {
                    continue;
                }
                terraform_dirs.insert(tf_dir.clone());

                let name = tf_dir
                    .file_name()
                    .and_then(|s| s.to_str())
                    .unwrap_or("terraform")
                    .to_string();

                let category = name.clone();
                let display_name = format!("🏗️ {}", format_display_name(&name));

                scripts.push(ScriptFile {
                    path: tf_dir,
                    name,
                    category,
                    display_name,
                    script_type: ScriptType::Terraform,
                });
            }
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
        assert_eq!(format_display_name("service.auth"), "Service Auth");
        assert_eq!(
            format_display_name("service.api-gateway"),
            "Service Api Gateway"
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

    #[test]
    fn test_discover_makefile() {
        let temp_dir = TempDir::new().unwrap();
        let makefile_path = temp_dir.path().join("Makefile");

        let content = ".PHONY: build test\n\nbuild:\n\techo building\n\ntest:\n\techo testing\n";
        fs::write(&makefile_path, content).unwrap();

        let result = discover_scripts(temp_dir.path()).unwrap();
        // If make binary is installed we get 1 ScriptFile with Makefile type, else 0
        let make_files: Vec<_> = result
            .iter()
            .filter(|s| s.script_type == ScriptType::Makefile)
            .collect();
        assert!(
            make_files.len() <= 1,
            "should have at most one Makefile script file"
        );
        if let Some(sf) = make_files.first() {
            assert_eq!(sf.script_type, ScriptType::Makefile);
            assert!(sf.path.ends_with("Makefile"));
            assert!(sf.display_name.contains("🔨"));
        }
    }

    // Tests for discover_single_file

    #[test]
    fn test_discover_single_file_bash() {
        let temp_dir = TempDir::new().unwrap();
        let script_path = temp_dir.path().join("deploy.sh");
        fs::write(&script_path, "#!/bin/bash\necho 'deploy'").unwrap();

        let result = discover_single_file(&script_path).unwrap();
        assert_eq!(result.name, "deploy");
        assert_eq!(result.category, "deploy");
        assert_eq!(result.display_name, "Deploy");
        assert_eq!(result.script_type, ScriptType::Bash);
    }

    #[test]
    fn test_discover_single_file_bash_with_underscores() {
        let temp_dir = TempDir::new().unwrap();
        let script_path = temp_dir.path().join("my_deploy_script.sh");
        fs::write(&script_path, "#!/bin/bash\necho 'deploy'").unwrap();

        let result = discover_single_file(&script_path).unwrap();
        assert_eq!(result.name, "my_deploy_script");
        assert_eq!(result.category, "my_deploy_script");
        assert_eq!(result.display_name, "My Deploy Script");
        assert_eq!(result.script_type, ScriptType::Bash);
    }

    #[test]
    fn test_discover_single_file_package_json() {
        let temp_dir = TempDir::new().unwrap();
        let package_path = temp_dir.path().join("package.json");
        let content = r#"{"name": "test", "scripts": {"test": "jest"}}"#;
        fs::write(&package_path, content).unwrap();

        let result = discover_single_file(&package_path).unwrap();
        assert_eq!(result.script_type, ScriptType::PackageJson);
        // Category should be the parent directory name
        assert!(!result.category.is_empty());
    }

    #[test]
    fn test_discover_single_file_nonexistent() {
        let temp_dir = TempDir::new().unwrap();
        let nonexistent = temp_dir.path().join("nonexistent.sh");

        let result = discover_single_file(&nonexistent);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("does not exist"));
    }

    #[test]
    fn test_discover_single_file_directory() {
        let temp_dir = TempDir::new().unwrap();

        let result = discover_single_file(temp_dir.path());
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("is not a file"));
    }

    #[test]
    fn test_discover_single_file_unsupported_type() {
        let temp_dir = TempDir::new().unwrap();
        let txt_path = temp_dir.path().join("readme.txt");
        fs::write(&txt_path, "some text").unwrap();

        let result = discover_single_file(&txt_path);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Unsupported file type"));
    }

    #[test]
    fn test_discover_single_file_with_emoji() {
        let temp_dir = TempDir::new().unwrap();
        let script_path = temp_dir.path().join("🏠 homelab.sh");
        fs::write(&script_path, "#!/bin/bash\necho 'homelab'").unwrap();

        let result = discover_single_file(&script_path).unwrap();
        assert_eq!(result.name, "🏠 homelab");
        assert_eq!(result.category, "🏠 homelab");
        assert_eq!(result.display_name, "🏠 Homelab");
        assert_eq!(result.script_type, ScriptType::Bash);
    }

    #[test]
    fn test_discover_terraform_tf_files() {
        let temp_dir = TempDir::new().unwrap();

        // Create .tf files
        fs::write(temp_dir.path().join("main.tf"), "resource {}").unwrap();
        fs::write(temp_dir.path().join("variables.tf"), "variable {}").unwrap();

        let result = discover_scripts(temp_dir.path()).unwrap();
        // If terraform binary is installed we get 1 ScriptFile with Terraform type, else 0
        let tf_files: Vec<_> = result
            .iter()
            .filter(|s| s.script_type == ScriptType::Terraform)
            .collect();
        assert!(
            tf_files.len() <= 1,
            "should have at most one Terraform script file per directory"
        );
        if let Some(sf) = tf_files.first() {
            assert_eq!(sf.script_type, ScriptType::Terraform);
            assert!(sf.display_name.contains("🏗️"));
            // Path should be the directory, not a specific .tf file
            assert!(sf.path.is_dir());
        }
    }

    #[test]
    fn test_discover_terraform_single_tf_file_per_dir() {
        let temp_dir = TempDir::new().unwrap();

        // Create multiple .tf files in the same directory
        fs::write(temp_dir.path().join("main.tf"), "resource {}").unwrap();
        fs::write(temp_dir.path().join("outputs.tf"), "output {}").unwrap();
        fs::write(temp_dir.path().join("variables.tf"), "variable {}").unwrap();

        let result = discover_scripts(temp_dir.path()).unwrap();
        let tf_files: Vec<_> = result
            .iter()
            .filter(|s| s.script_type == ScriptType::Terraform)
            .collect();
        // Even with multiple .tf files, we should get at most one entry
        assert!(tf_files.len() <= 1);
    }

    #[test]
    fn test_discover_bazel_single_entry_per_dir() {
        let temp_dir = TempDir::new().unwrap();

        // Create multiple bazel files in the same directory
        fs::write(temp_dir.path().join("WORKSPACE"), "").unwrap();
        fs::write(temp_dir.path().join("BUILD"), "").unwrap();
        fs::write(temp_dir.path().join("MODULE.bazel"), "").unwrap();

        let result = discover_scripts(temp_dir.path()).unwrap();
        // If bazel binary is installed we get 1 ScriptFile with Bazel type, else 0
        let bazel_files: Vec<_> = result
            .iter()
            .filter(|s| s.script_type == ScriptType::Bazel)
            .collect();
        assert!(
            bazel_files.len() <= 1,
            "should have at most one Bazel script file per directory"
        );
        if let Some(sf) = bazel_files.first() {
            assert_eq!(sf.script_type, ScriptType::Bazel);
            assert!(sf.display_name.contains("🌿"));
            // Path should be the directory, not a specific bazel file
            assert!(sf.path.is_dir());
        }
    }

    #[test]
    fn test_discover_gradle_single_entry_per_dir() {
        let temp_dir = TempDir::new().unwrap();

        // Create multiple gradle files in the same directory
        fs::write(
            temp_dir.path().join("build.gradle"),
            "plugins { id 'java' }",
        )
        .unwrap();
        fs::write(
            temp_dir.path().join("settings.gradle"),
            "rootProject.name = 'test'",
        )
        .unwrap();

        let result = discover_scripts(temp_dir.path()).unwrap();
        let gradle_files: Vec<_> = result
            .iter()
            .filter(|s| s.script_type == ScriptType::Gradle)
            .collect();
        // Even with multiple gradle files, we should get at most one entry
        assert!(
            gradle_files.len() <= 1,
            "should have at most one Gradle script file per directory"
        );
    }
}
