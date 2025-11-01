use anyhow::Result;
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

    for entry in WalkDir::new(scripts_dir)
        .max_depth(2)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();

        if path.is_file() {
            if let Some(extension) = path.extension() {
                if extension == "sh" {
                    let name = path
                        .file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap_or("unknown")
                        .to_string();

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
            }
        }
    }

    Ok(scripts)
}
