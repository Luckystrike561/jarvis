use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

#[derive(Debug, Clone)]
pub struct ScriptFile {
    pub path: PathBuf,
    #[allow(dead_code)]
    pub name: String,
    pub category: String,
}

pub fn discover_scripts(scripts_dir: &Path) -> Result<Vec<ScriptFile>> {
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
        .max_depth(2)
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

        // Check file extension
        let extension = match path.extension() {
            Some(ext) => ext,
            None => continue,
        };

        if extension != "sh" {
            continue;
        }

        // Extract filename
        let name = path
            .file_stem()
            .and_then(|s| s.to_str())
            .with_context(|| format!("Invalid filename for script: {}", path.display()))?
            .to_string();

        // Categorize the script
        let category = match name.as_str() {
            "fedora" => "System Management",
            "homelab" => "Homelab Operations",
            "util" => "Utilities",
            _ => "Other",
        };

        scripts.push(ScriptFile {
            path: path.to_path_buf(),
            name,
            category: category.to_string(),
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
        assert_eq!(result[0].category, "Other");
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

        // Check categorization
        let fedora = result.iter().find(|s| s.name == "fedora").unwrap();
        assert_eq!(fedora.category, "System Management");

        let homelab = result.iter().find(|s| s.name == "homelab").unwrap();
        assert_eq!(homelab.category, "Homelab Operations");

        let util = result.iter().find(|s| s.name == "util").unwrap();
        assert_eq!(util.category, "Utilities");

        let custom = result.iter().find(|s| s.name == "custom").unwrap();
        assert_eq!(custom.category, "Other");
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
}
