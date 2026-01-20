use anyhow::{Context, Result};
use serde::{Deserialize, Deserializer, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

use crate::script::discovery::format_display_name;

/// Represents a script value that can be either a string or an array of strings
#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
pub enum ScriptValue {
    Single(String),
    Multiple(Vec<String>),
}

impl<'de> Deserialize<'de> for ScriptValue {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        use serde::de::{self, Visitor};

        struct ScriptValueVisitor;

        impl<'de> Visitor<'de> for ScriptValueVisitor {
            type Value = ScriptValue;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a string or an array of strings")
            }

            fn visit_str<E>(self, value: &str) -> Result<ScriptValue, E>
            where
                E: de::Error,
            {
                Ok(ScriptValue::Single(value.to_string()))
            }

            fn visit_string<E>(self, value: String) -> Result<ScriptValue, E>
            where
                E: de::Error,
            {
                Ok(ScriptValue::Single(value))
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<ScriptValue, A::Error>
            where
                A: de::SeqAccess<'de>,
            {
                let mut commands = Vec::new();
                while let Some(cmd) = seq.next_element::<String>()? {
                    commands.push(cmd);
                }
                Ok(ScriptValue::Multiple(commands))
            }
        }

        deserializer.deserialize_any(ScriptValueVisitor)
    }
}

impl ScriptValue {
    /// Convert to a vector of commands
    pub fn to_commands(&self) -> Vec<String> {
        match self {
            ScriptValue::Single(cmd) => vec![cmd.clone()],
            ScriptValue::Multiple(cmds) => cmds.clone(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DevboxShell {
    #[serde(default)]
    pub scripts: HashMap<String, ScriptValue>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DevboxJson {
    #[serde(default)]
    pub shell: Option<DevboxShell>,
}

#[derive(Debug, Clone)]
pub struct DevboxScript {
    pub name: String,
    pub display_name: String,
    pub category: String,
    pub description: String,
    #[allow(dead_code)] // Used for display/debug, may be used in future features
    pub commands: Vec<String>,
}

/// Parse a devbox.json file and extract devbox scripts
pub fn parse_devbox_json(path: &Path, category: &str) -> Result<Vec<DevboxScript>> {
    // Read devbox.json file
    let content = fs::read_to_string(path)
        .with_context(|| format!("Failed to read devbox.json: {}", path.display()))?;

    // Parse JSON
    let devbox: DevboxJson = serde_json::from_str(&content)
        .with_context(|| format!("Failed to parse devbox.json: {}", path.display()))?;

    let mut scripts = Vec::new();

    // Extract scripts from shell.scripts if present
    if let Some(shell) = devbox.shell {
        for (script_name, script_value) in shell.scripts {
            // Convert ScriptValue to Vec<String>
            let script_commands = script_value.to_commands();

            // Auto-generate display name from script name
            let display_name = format_display_name(&script_name);

            // Create description with command preview
            let command_preview = script_commands.join(" && ");
            let description = if command_preview.len() > 60 {
                format!("devbox run {} - {}...", script_name, &command_preview[..57])
            } else {
                format!("devbox run {} - {}", script_name, command_preview)
            };

            scripts.push(DevboxScript {
                name: script_name.clone(),
                display_name,
                category: category.to_string(),
                description,
                commands: script_commands,
            });
        }
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
    fn test_parse_devbox_json_valid() {
        let temp_dir = TempDir::new().unwrap();
        let devbox_path = temp_dir.path().join("devbox.json");

        let content = r#"{
  "shell": {
    "scripts": {
      "build": ["cargo build"],
      "test": ["cargo test"],
      "check": ["cargo clippy -- -D warnings", "cargo fmt -- --check"]
    }
  }
}"#;
        fs::write(&devbox_path, content).unwrap();

        let result = parse_devbox_json(&devbox_path, "Test").unwrap();
        assert_eq!(result.len(), 3);

        // Check that scripts are sorted
        assert_eq!(result[0].name, "build");
        assert_eq!(result[1].name, "check");
        assert_eq!(result[2].name, "test");

        // Verify display names
        assert_eq!(result[0].display_name, "Build");
        assert_eq!(result[1].display_name, "Check");
        assert_eq!(result[2].display_name, "Test");

        // Verify descriptions
        assert!(result[0].description.contains("devbox run build"));
        assert!(result[0].description.contains("cargo build"));
    }

    #[test]
    fn test_parse_devbox_json_empty_scripts() {
        let temp_dir = TempDir::new().unwrap();
        let devbox_path = temp_dir.path().join("devbox.json");

        let content = r#"{
  "shell": {
    "scripts": {}
  }
}"#;
        fs::write(&devbox_path, content).unwrap();

        let result = parse_devbox_json(&devbox_path, "Test").unwrap();
        assert_eq!(result.len(), 0);
    }

    #[test]
    fn test_parse_devbox_json_no_shell() {
        let temp_dir = TempDir::new().unwrap();
        let devbox_path = temp_dir.path().join("devbox.json");

        let content = r#"{
  "packages": ["cargo"]
}"#;
        fs::write(&devbox_path, content).unwrap();

        let result = parse_devbox_json(&devbox_path, "Test").unwrap();
        assert_eq!(result.len(), 0);
    }

    #[test]
    fn test_parse_devbox_json_no_scripts() {
        let temp_dir = TempDir::new().unwrap();
        let devbox_path = temp_dir.path().join("devbox.json");

        let content = r#"{
  "shell": {
    "init_hook": ["echo 'hello'"]
  }
}"#;
        fs::write(&devbox_path, content).unwrap();

        let result = parse_devbox_json(&devbox_path, "Test").unwrap();
        assert_eq!(result.len(), 0);
    }

    #[test]
    fn test_parse_devbox_json_complex_script_names() {
        let temp_dir = TempDir::new().unwrap();
        let devbox_path = temp_dir.path().join("devbox.json");

        let content = r#"{
  "shell": {
    "scripts": {
      "test:unit": ["jest --testPathPattern=unit"],
      "test:integration": ["jest --testPathPattern=integration"],
      "build:dev": ["webpack --mode development"],
      "deploy:prod": ["node scripts/deploy.js"]
    }
  }
}"#;
        fs::write(&devbox_path, content).unwrap();

        let result = parse_devbox_json(&devbox_path, "Test").unwrap();
        assert_eq!(result.len(), 4);

        // Check display name formatting (colons become spaces with title case)
        let build_dev = result.iter().find(|s| s.name == "build:dev").unwrap();
        assert_eq!(build_dev.display_name, "Build:dev");

        let test_unit = result.iter().find(|s| s.name == "test:unit").unwrap();
        assert_eq!(test_unit.display_name, "Test:unit");
    }

    #[test]
    fn test_parse_devbox_json_multi_command_scripts() {
        let temp_dir = TempDir::new().unwrap();
        let devbox_path = temp_dir.path().join("devbox.json");

        let content = r#"{
  "shell": {
    "scripts": {
      "check": ["cargo clippy -- -D warnings", "cargo fmt -- --check"]
    }
  }
}"#;
        fs::write(&devbox_path, content).unwrap();

        let result = parse_devbox_json(&devbox_path, "Test").unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].commands.len(), 2);
        assert!(result[0].description.contains("cargo clippy"));
        assert!(result[0].description.contains("&&"));
    }

    #[test]
    fn test_parse_devbox_json_long_commands() {
        let temp_dir = TempDir::new().unwrap();
        let devbox_path = temp_dir.path().join("devbox.json");

        let content = r#"{
  "shell": {
    "scripts": {
      "long": ["echo 'This is a very long command that should be truncated in the description because it exceeds the maximum length'"]
    }
  }
}"#;
        fs::write(&devbox_path, content).unwrap();

        let result = parse_devbox_json(&devbox_path, "Test").unwrap();
        assert_eq!(result.len(), 1);

        // Verify long command is truncated
        assert!(result[0].description.ends_with("..."));
    }

    #[test]
    fn test_parse_devbox_json_file_not_found() {
        let temp_dir = TempDir::new().unwrap();
        let devbox_path = temp_dir.path().join("nonexistent.json");

        let result = parse_devbox_json(&devbox_path, "Test");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_devbox_json_invalid_json() {
        let temp_dir = TempDir::new().unwrap();
        let devbox_path = temp_dir.path().join("devbox.json");

        let content = r#"{
  "shell": {
    "scripts": {
      "invalid": ["missing closing brace"
}"#;
        fs::write(&devbox_path, content).unwrap();

        let result = parse_devbox_json(&devbox_path, "Test");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_devbox_json_category() {
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

        let result = parse_devbox_json(&devbox_path, "MyCategory").unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].category, "MyCategory");
    }

    #[test]
    fn test_parse_devbox_json_hyphenated_names() {
        let temp_dir = TempDir::new().unwrap();
        let devbox_path = temp_dir.path().join("devbox.json");

        let content = r#"{
  "shell": {
    "scripts": {
      "deploy-homebrew": ["bash scripts/update-formula.sh"]
    }
  }
}"#;
        fs::write(&devbox_path, content).unwrap();

        let result = parse_devbox_json(&devbox_path, "Test").unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].display_name, "Deploy Homebrew");
    }

    #[test]
    fn test_parse_devbox_json_string_scripts() {
        let temp_dir = TempDir::new().unwrap();
        let devbox_path = temp_dir.path().join("devbox.json");

        // Scripts as plain strings (not arrays) - common devbox.json format
        let content = r#"{
  "shell": {
    "scripts": {
      "help": "jq .shell.scripts devbox.json",
      "build": "cargo build --release",
      "test": "cargo test"
    }
  }
}"#;
        fs::write(&devbox_path, content).unwrap();

        let result = parse_devbox_json(&devbox_path, "Test").unwrap();
        assert_eq!(result.len(), 3);

        // Check that string scripts are parsed correctly
        let help = result.iter().find(|s| s.name == "help").unwrap();
        assert_eq!(help.commands.len(), 1);
        assert_eq!(help.commands[0], "jq .shell.scripts devbox.json");

        let build = result.iter().find(|s| s.name == "build").unwrap();
        assert_eq!(build.commands.len(), 1);
        assert_eq!(build.commands[0], "cargo build --release");
    }

    #[test]
    fn test_parse_devbox_json_mixed_string_and_array_scripts() {
        let temp_dir = TempDir::new().unwrap();
        let devbox_path = temp_dir.path().join("devbox.json");

        // Mix of string and array scripts
        let content = r#"{
  "shell": {
    "scripts": {
      "simple": "echo hello",
      "complex": ["cargo clippy", "cargo fmt --check"]
    }
  }
}"#;
        fs::write(&devbox_path, content).unwrap();

        let result = parse_devbox_json(&devbox_path, "Test").unwrap();
        assert_eq!(result.len(), 2);

        let simple = result.iter().find(|s| s.name == "simple").unwrap();
        assert_eq!(simple.commands.len(), 1);
        assert_eq!(simple.commands[0], "echo hello");

        let complex = result.iter().find(|s| s.name == "complex").unwrap();
        assert_eq!(complex.commands.len(), 2);
        assert_eq!(complex.commands[0], "cargo clippy");
        assert_eq!(complex.commands[1], "cargo fmt --check");
    }
}
