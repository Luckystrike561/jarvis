//! # Usage Storage
//!
//! Handles persistent storage of command usage data using XDG-compliant paths.
//!
//! ## Storage Location
//!
//! ```text
//! ~/.local/share/jarvis/usage/
//! ├── <project-hash-1>.json
//! ├── <project-hash-2>.json
//! └── ...
//! ```
//!
//! Each project has its own JSON file, identified by a hash of the project path.

use crate::script::ScriptType;
use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

/// Maximum number of frequently used commands to display
pub const MAX_FREQUENT_COMMANDS: usize = 5;

/// Reserved category name for frequently used commands
pub const FREQUENTLY_USED_CATEGORY: &str = "⭐ Frequently Used";

/// A single usage entry tracking how often a command is used
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageEntry {
    /// The function/script name
    pub function_name: String,
    /// The type of script (Bash, npm, etc.)
    pub script_type: ScriptType,
    /// The category this function belongs to
    pub category: String,
    /// Number of times this command has been executed
    pub count: u64,
    /// When this command was last used
    pub last_used: DateTime<Utc>,
}

/// Usage data for a specific project
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectUsage {
    /// The absolute path to the project directory
    pub project_path: PathBuf,
    /// Map of `function_name` -> usage entry
    pub entries: HashMap<String, UsageEntry>,
}

impl ProjectUsage {
    /// Create a new empty project usage tracker
    pub fn new(project_path: PathBuf) -> Self {
        Self {
            project_path,
            entries: HashMap::new(),
        }
    }

    /// Record a command execution
    pub fn record_usage(&mut self, function_name: &str, script_type: ScriptType, category: &str) {
        let entry = self
            .entries
            .entry(function_name.to_string())
            .or_insert_with(|| UsageEntry {
                function_name: function_name.to_string(),
                script_type,
                category: category.to_string(),
                count: 0,
                last_used: Utc::now(),
            });

        entry.count += 1;
        entry.last_used = Utc::now();
        // Update script type and category in case they changed
        entry.script_type = script_type;
        entry.category = category.to_string();
    }

    /// Get the most frequently used commands, sorted by count (descending)
    pub fn get_frequent(&self, limit: usize) -> Vec<&UsageEntry> {
        let mut entries: Vec<_> = self.entries.values().collect();
        entries.sort_by(|a, b| b.count.cmp(&a.count));
        entries.truncate(limit);
        entries
    }
}

/// Manages usage tracking with persistent storage
#[derive(Debug)]
pub struct UsageTracker {
    /// The project path being tracked
    project_path: PathBuf,
    /// The storage directory for usage files
    storage_dir: PathBuf,
    /// Current usage data
    usage: ProjectUsage,
}

impl UsageTracker {
    /// Create a new usage tracker for a project
    ///
    /// Automatically loads existing usage data if available.
    pub fn new(project_path: PathBuf) -> Result<Self> {
        let storage_dir = get_storage_dir()?;

        // Ensure storage directory exists
        fs::create_dir_all(&storage_dir).with_context(|| {
            format!(
                "Failed to create usage storage directory: {}",
                storage_dir.display()
            )
        })?;

        // Load existing usage data or create new
        let usage_file = get_usage_file_path(&storage_dir, &project_path);
        let usage = if usage_file.exists() {
            load_usage(&usage_file).unwrap_or_else(|_| ProjectUsage::new(project_path.clone()))
        } else {
            ProjectUsage::new(project_path.clone())
        };

        Ok(Self {
            project_path,
            storage_dir,
            usage,
        })
    }

    /// Create a usage tracker with a custom storage directory (for testing)
    #[cfg(test)]
    pub fn with_storage_dir(project_path: PathBuf, storage_dir: PathBuf) -> Result<Self> {
        fs::create_dir_all(&storage_dir)?;

        let usage_file = get_usage_file_path(&storage_dir, &project_path);
        let usage = if usage_file.exists() {
            load_usage(&usage_file).unwrap_or_else(|_| ProjectUsage::new(project_path.clone()))
        } else {
            ProjectUsage::new(project_path.clone())
        };

        Ok(Self {
            project_path,
            storage_dir,
            usage,
        })
    }

    /// Record a command execution and save to disk
    pub fn record(
        &mut self,
        function_name: &str,
        script_type: ScriptType,
        category: &str,
    ) -> Result<()> {
        self.usage
            .record_usage(function_name, script_type, category);
        self.save()
    }

    /// Get the most frequently used commands
    pub fn get_frequent(&self, limit: usize) -> Vec<&UsageEntry> {
        self.usage.get_frequent(limit)
    }

    /// Check if there are any recorded usages
    pub fn has_usage_data(&self) -> bool {
        !self.usage.entries.is_empty()
    }

    /// Save usage data to disk
    fn save(&self) -> Result<()> {
        let usage_file = get_usage_file_path(&self.storage_dir, &self.project_path);
        let json =
            serde_json::to_string_pretty(&self.usage).context("Failed to serialize usage data")?;

        fs::write(&usage_file, json)
            .with_context(|| format!("Failed to write usage file: {}", usage_file.display()))?;

        Ok(())
    }

    /// Get the project path this tracker is associated with
    pub fn project_path(&self) -> &Path {
        &self.project_path
    }
}

/// Get the XDG-compliant storage directory for usage data
fn get_storage_dir() -> Result<PathBuf> {
    let proj_dirs = directories::ProjectDirs::from("", "", "jarvis")
        .context("Failed to determine application data directory")?;

    Ok(proj_dirs.data_dir().join("usage"))
}

/// Generate a unique filename for a project based on its path
fn get_usage_file_path(storage_dir: &Path, project_path: &Path) -> PathBuf {
    // Use a simple hash of the project path for the filename
    let hash = simple_hash(project_path.to_string_lossy().as_ref());
    storage_dir.join(format!("{:016x}.json", hash))
}

/// Simple hash function for generating project file names
fn simple_hash(s: &str) -> u64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    s.hash(&mut hasher);
    hasher.finish()
}

/// Load usage data from a file
fn load_usage(path: &Path) -> Result<ProjectUsage> {
    let content = fs::read_to_string(path)
        .with_context(|| format!("Failed to read usage file: {}", path.display()))?;

    let usage: ProjectUsage = serde_json::from_str(&content)
        .with_context(|| format!("Failed to parse usage file: {}", path.display()))?;

    Ok(usage)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_project_usage_new() {
        let usage = ProjectUsage::new(PathBuf::from("/test/project"));
        assert_eq!(usage.project_path, PathBuf::from("/test/project"));
        assert!(usage.entries.is_empty());
    }

    #[test]
    fn test_project_usage_record_usage() {
        let mut usage = ProjectUsage::new(PathBuf::from("/test/project"));

        usage.record_usage("build", ScriptType::Bash, "Build");
        assert_eq!(usage.entries.len(), 1);
        assert_eq!(usage.entries.get("build").unwrap().count, 1);

        usage.record_usage("build", ScriptType::Bash, "Build");
        assert_eq!(usage.entries.get("build").unwrap().count, 2);

        usage.record_usage("test", ScriptType::PackageJson, "Test");
        assert_eq!(usage.entries.len(), 2);
        assert_eq!(usage.entries.get("test").unwrap().count, 1);
    }

    #[test]
    fn test_project_usage_get_frequent() {
        let mut usage = ProjectUsage::new(PathBuf::from("/test/project"));

        // Add some usage data
        for _ in 0..10 {
            usage.record_usage("build", ScriptType::Bash, "Build");
        }
        for _ in 0..5 {
            usage.record_usage("test", ScriptType::Bash, "Test");
        }
        for _ in 0..3 {
            usage.record_usage("lint", ScriptType::Bash, "Lint");
        }

        let frequent = usage.get_frequent(2);
        assert_eq!(frequent.len(), 2);
        assert_eq!(frequent[0].function_name, "build");
        assert_eq!(frequent[0].count, 10);
        assert_eq!(frequent[1].function_name, "test");
        assert_eq!(frequent[1].count, 5);
    }

    #[test]
    fn test_usage_tracker_new() {
        let temp_dir = TempDir::new().unwrap();
        let project_path = temp_dir.path().to_path_buf();
        let storage_dir = temp_dir.path().join("storage");

        let tracker = UsageTracker::with_storage_dir(project_path.clone(), storage_dir).unwrap();
        assert_eq!(tracker.project_path(), project_path);
        assert!(!tracker.has_usage_data());
    }

    #[test]
    fn test_usage_tracker_record_and_persist() {
        let temp_dir = TempDir::new().unwrap();
        let project_path = temp_dir.path().join("my-project");
        let storage_dir = temp_dir.path().join("storage");

        // Create tracker and record usage
        {
            let mut tracker =
                UsageTracker::with_storage_dir(project_path.clone(), storage_dir.clone()).unwrap();
            tracker.record("build", ScriptType::Bash, "Build").unwrap();
            tracker.record("build", ScriptType::Bash, "Build").unwrap();
            tracker
                .record("test", ScriptType::PackageJson, "Test")
                .unwrap();
        }

        // Create new tracker and verify data was persisted
        {
            let tracker = UsageTracker::with_storage_dir(project_path, storage_dir).unwrap();
            assert!(tracker.has_usage_data());

            let frequent = tracker.get_frequent(10);
            assert_eq!(frequent.len(), 2);

            let build = frequent
                .iter()
                .find(|e| e.function_name == "build")
                .unwrap();
            assert_eq!(build.count, 2);

            let test = frequent.iter().find(|e| e.function_name == "test").unwrap();
            assert_eq!(test.count, 1);
        }
    }

    #[test]
    fn test_usage_tracker_get_frequent() {
        let temp_dir = TempDir::new().unwrap();
        let project_path = temp_dir.path().to_path_buf();
        let storage_dir = temp_dir.path().join("storage");

        let mut tracker = UsageTracker::with_storage_dir(project_path, storage_dir).unwrap();

        for _ in 0..10 {
            tracker.record("cmd1", ScriptType::Bash, "Cat1").unwrap();
        }
        for _ in 0..5 {
            tracker.record("cmd2", ScriptType::Bash, "Cat2").unwrap();
        }
        for _ in 0..3 {
            tracker.record("cmd3", ScriptType::Bash, "Cat3").unwrap();
        }

        let frequent = tracker.get_frequent(2);
        assert_eq!(frequent.len(), 2);
        assert_eq!(frequent[0].function_name, "cmd1");
        assert_eq!(frequent[1].function_name, "cmd2");
    }

    #[test]
    fn test_simple_hash() {
        let hash1 = simple_hash("/home/user/project1");
        let hash2 = simple_hash("/home/user/project2");
        let hash3 = simple_hash("/home/user/project1");

        assert_ne!(hash1, hash2);
        assert_eq!(hash1, hash3);
    }

    #[test]
    fn test_corrupted_usage_file() {
        let temp_dir = TempDir::new().unwrap();
        let project_path = temp_dir.path().join("my-project");
        let storage_dir = temp_dir.path().join("storage");

        // Create storage dir and write corrupted file
        fs::create_dir_all(&storage_dir).unwrap();
        let usage_file = get_usage_file_path(&storage_dir, &project_path);
        fs::write(&usage_file, "not valid json").unwrap();

        // Should gracefully handle corrupted file
        let tracker = UsageTracker::with_storage_dir(project_path, storage_dir).unwrap();
        assert!(!tracker.has_usage_data());
    }

    #[test]
    fn test_usage_entry_serialization() {
        let entry = UsageEntry {
            function_name: "build".to_string(),
            script_type: ScriptType::Bash,
            category: "Build".to_string(),
            count: 42,
            last_used: Utc::now(),
        };

        let json = serde_json::to_string(&entry).unwrap();
        let parsed: UsageEntry = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.function_name, "build");
        assert_eq!(parsed.count, 42);
    }
}
