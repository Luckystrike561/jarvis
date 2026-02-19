//! # Configuration Persistence
//!
//! Manages user configuration stored in `~/.config/jarvis/config.json`.
//!
//! ## Overview
//!
//! The [`Config`] struct is serialized to / deserialized from a JSON file in
//! the user's XDG config directory. Currently, the only persisted setting is
//! the selected theme name.
//!
//! ## File Location
//!
//! ```text
//! ~/.config/jarvis/config.json
//! ```
//!
//! The `directories` crate is used to resolve the platform-appropriate config
//! directory.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

/// Persisted user configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    /// The name of the selected theme (must match a built-in theme name).
    #[serde(default = "default_theme_name")]
    pub theme: String,
}

fn default_theme_name() -> String {
    "Catppuccin Mocha".to_string()
}

impl Default for Config {
    fn default() -> Self {
        Self {
            theme: default_theme_name(),
        }
    }
}

impl Config {
    /// Load configuration from disk. Returns `Config::default()` if the file
    /// does not exist or cannot be parsed.
    pub fn load() -> Self {
        Self::try_load().unwrap_or_default()
    }

    /// Try to load configuration, returning an error on failure.
    fn try_load() -> Result<Self> {
        let path = Self::config_path()?;
        Self::load_from(&path)
    }

    /// Load configuration from a specific path. Returns `Config::default()` if
    /// the file does not exist.
    pub fn load_from(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let contents = fs::read_to_string(path)
            .with_context(|| format!("Failed to read config file: {}", path.display()))?;
        let config: Self = serde_json::from_str(&contents)
            .with_context(|| format!("Failed to parse config file: {}", path.display()))?;
        Ok(config)
    }

    /// Save the current configuration to disk.
    pub fn save(&self) -> Result<()> {
        let path = Self::config_path()?;
        self.save_to(&path)
    }

    /// Save the current configuration to a specific path.
    pub fn save_to(&self, path: &Path) -> Result<()> {
        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).with_context(|| {
                format!("Failed to create config directory: {}", parent.display())
            })?;
        }

        let contents = serde_json::to_string_pretty(self).context("Failed to serialize config")?;
        fs::write(path, contents)
            .with_context(|| format!("Failed to write config file: {}", path.display()))?;

        Ok(())
    }

    /// Return the path to the config file.
    fn config_path() -> Result<PathBuf> {
        let dirs = directories::ProjectDirs::from("", "", "jarvis")
            .context("Could not determine config directory")?;
        Ok(dirs.config_dir().join("config.json"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.theme, "Catppuccin Mocha");
    }

    #[test]
    fn test_serialize_deserialize() {
        let config = Config {
            theme: "Dracula".to_string(),
        };
        let json = serde_json::to_string(&config).expect("serialize");
        let loaded: Config = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(loaded.theme, "Dracula");
    }

    #[test]
    fn test_deserialize_missing_theme_uses_default() {
        let json = "{}";
        let config: Config = serde_json::from_str(json).expect("deserialize");
        assert_eq!(config.theme, "Catppuccin Mocha");
    }

    #[test]
    fn test_save_and_load_roundtrip() {
        let temp_dir = TempDir::new().expect("create temp dir");
        let config_path = temp_dir.path().join("config.json");

        let config = Config {
            theme: "Nord".to_string(),
        };

        // Write directly to the temp path
        let contents = serde_json::to_string_pretty(&config).expect("serialize");
        fs::write(&config_path, contents).expect("write");

        // Read back
        let loaded_contents = fs::read_to_string(&config_path).expect("read");
        let loaded: Config = serde_json::from_str(&loaded_contents).expect("deserialize");
        assert_eq!(loaded.theme, "Nord");
    }

    #[test]
    fn test_save_to_load_from_roundtrip() {
        let temp_dir = TempDir::new().expect("create temp dir");
        let config_path = temp_dir.path().join("subdir").join("config.json");

        let config = Config {
            theme: "Dracula".to_string(),
        };

        // Use the actual save_to / load_from methods
        config.save_to(&config_path).expect("save_to");
        let loaded = Config::load_from(&config_path).expect("load_from");
        assert_eq!(loaded.theme, config.theme);
    }

    #[test]
    fn test_load_from_missing_file_returns_default() {
        let temp_dir = TempDir::new().expect("create temp dir");
        let config_path = temp_dir.path().join("does_not_exist.json");

        let loaded = Config::load_from(&config_path).expect("load_from");
        assert_eq!(loaded.theme, "Catppuccin Mocha");
    }

    #[test]
    fn test_deny_unknown_fields() {
        let json = r#"{"theme": "Nord", "unknown_field": true}"#;
        let result: Result<Config, _> = serde_json::from_str(json);
        assert!(result.is_err(), "should reject unknown fields");
    }
}
