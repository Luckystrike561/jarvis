use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

use crate::script::discovery::format_display_name;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageJson {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub scripts: HashMap<String, String>,
}

#[derive(Debug, Clone)]
pub struct NpmScript {
    pub name: String,
    pub display_name: String,
    pub category: String,
    pub description: String,
    #[allow(dead_code)] // Used for display/debug, may be used in future features
    pub command: String,
}

/// Parse a package.json file and extract npm scripts
pub fn parse_package_json(path: &Path, category: &str) -> Result<Vec<NpmScript>> {
    // Read package.json file
    let content = fs::read_to_string(path)
        .with_context(|| format!("Failed to read package.json: {}", path.display()))?;

    // Parse JSON
    let package: PackageJson = serde_json::from_str(&content)
        .with_context(|| format!("Failed to parse package.json: {}", path.display()))?;

    let mut scripts = Vec::new();

    // Convert each npm script to a NpmScript
    for (script_name, script_command) in package.scripts {
        // Auto-generate display name from script name
        let display_name = format_display_name(&script_name);

        // Create description with command preview
        let description = if script_command.len() > 60 {
            format!("npm run {} - {}...", script_name, &script_command[..57])
        } else {
            format!("npm run {} - {}", script_name, script_command)
        };

        scripts.push(NpmScript {
            name: script_name.clone(),
            display_name,
            category: category.to_string(),
            description,
            command: script_command,
        });
    }

    // Sort scripts alphabetically by name for consistent display
    scripts.sort_by(|a, b| a.name.cmp(&b.name));

    Ok(scripts)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_parse_package_json_valid() {
        let temp_dir = TempDir::new().unwrap();
        let package_path = temp_dir.path().join("package.json");

        let content = r#"{
  "name": "test-project",
  "scripts": {
    "start": "node index.js",
    "test": "jest",
    "build": "webpack --mode production"
  }
}"#;
        fs::write(&package_path, content).unwrap();

        let result = parse_package_json(&package_path, "Test").unwrap();
        assert_eq!(result.len(), 3);

        // Check that scripts are sorted
        assert_eq!(result[0].name, "build");
        assert_eq!(result[1].name, "start");
        assert_eq!(result[2].name, "test");

        // Verify display names
        assert_eq!(result[0].display_name, "Build");
        assert_eq!(result[1].display_name, "Start");
        assert_eq!(result[2].display_name, "Test");

        // Verify descriptions
        assert!(result[0].description.contains("npm run build"));
        assert!(result[0].description.contains("webpack --mode production"));
    }

    #[test]
    fn test_parse_package_json_empty_scripts() {
        let temp_dir = TempDir::new().unwrap();
        let package_path = temp_dir.path().join("package.json");

        let content = r#"{
  "name": "test-project",
  "scripts": {}
}"#;
        fs::write(&package_path, content).unwrap();

        let result = parse_package_json(&package_path, "Test").unwrap();
        assert_eq!(result.len(), 0);
    }

    #[test]
    fn test_parse_package_json_no_scripts() {
        let temp_dir = TempDir::new().unwrap();
        let package_path = temp_dir.path().join("package.json");

        let content = r#"{
  "name": "test-project"
}"#;
        fs::write(&package_path, content).unwrap();

        let result = parse_package_json(&package_path, "Test").unwrap();
        assert_eq!(result.len(), 0);
    }

    #[test]
    fn test_parse_package_json_complex_script_names() {
        let temp_dir = TempDir::new().unwrap();
        let package_path = temp_dir.path().join("package.json");

        let content = r#"{
  "name": "test-project",
  "scripts": {
    "test:unit": "jest --testPathPattern=unit",
    "test:integration": "jest --testPathPattern=integration",
    "build:dev": "webpack --mode development",
    "deploy:prod": "node scripts/deploy.js"
  }
}"#;
        fs::write(&package_path, content).unwrap();

        let result = parse_package_json(&package_path, "Test").unwrap();
        assert_eq!(result.len(), 4);

        // Check display name formatting (colons become spaces with title case)
        let build_dev = result.iter().find(|s| s.name == "build:dev").unwrap();
        assert_eq!(build_dev.display_name, "Build:dev");

        let test_unit = result.iter().find(|s| s.name == "test:unit").unwrap();
        assert_eq!(test_unit.display_name, "Test:unit");
    }

    #[test]
    fn test_parse_package_json_long_commands() {
        let temp_dir = TempDir::new().unwrap();
        let package_path = temp_dir.path().join("package.json");

        let content = r#"{
  "name": "test-project",
  "scripts": {
    "long": "echo 'This is a very long command that should be truncated in the description because it exceeds the maximum length'"
  }
}"#;
        fs::write(&package_path, content).unwrap();

        let result = parse_package_json(&package_path, "Test").unwrap();
        assert_eq!(result.len(), 1);

        // Verify long command is truncated
        assert!(result[0].description.len() < result[0].command.len() + 20);
        assert!(result[0].description.ends_with("..."));
    }

    #[test]
    fn test_parse_package_json_file_not_found() {
        let temp_dir = TempDir::new().unwrap();
        let package_path = temp_dir.path().join("nonexistent.json");

        let result = parse_package_json(&package_path, "Test");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_package_json_invalid_json() {
        let temp_dir = TempDir::new().unwrap();
        let package_path = temp_dir.path().join("package.json");

        let content = r#"{
  "name": "test-project",
  "scripts": {
    "invalid": "missing closing brace"
}"#;
        fs::write(&package_path, content).unwrap();

        let result = parse_package_json(&package_path, "Test");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_package_json_category() {
        let temp_dir = TempDir::new().unwrap();
        let package_path = temp_dir.path().join("package.json");

        let content = r#"{
  "name": "test-project",
  "scripts": {
    "test": "jest"
  }
}"#;
        fs::write(&package_path, content).unwrap();

        let result = parse_package_json(&package_path, "MyCategory").unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].category, "MyCategory");
    }
}
