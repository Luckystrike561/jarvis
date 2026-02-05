use anyhow::{Context, Result};
use regex::Regex;
use std::fs;
use std::path::Path;

use crate::script::discovery::{format_display_name, ScriptType};
use crate::script::utils::is_valid_bash_identifier;

#[derive(Debug, Clone)]
pub struct ScriptFunction {
    pub name: String,
    pub display_name: String,
    pub category: String,
    pub description: String,
    pub emoji: Option<String>,
    pub ignored: bool,
    pub script_type: ScriptType,
}

pub fn parse_script(path: &Path, category: &str) -> Result<Vec<ScriptFunction>> {
    // Read script file with proper error context
    let content = fs::read_to_string(path)
        .with_context(|| format!("Failed to read script file: {}", path.display()))?;

    let mut functions = Vec::new();

    // Split content into lines for easier processing
    let lines: Vec<&str> = content.lines().collect();

    // Regex pattern to match bash function definitions
    // Matches both formats:
    // 1. function_name() {
    // 2. function function_name() {
    let func_re = Regex::new(r"^(?:function\s+)?([a-zA-Z_][a-zA-Z0-9_]*)\s*\(\s*\)\s*\{")
        .context("Failed to compile function regex pattern")?;

    // Regex patterns for metadata comments
    let emoji_re =
        Regex::new(r"^\s*#\s*@emoji\s+(.+)$").context("Failed to compile emoji regex pattern")?;
    let desc_re = Regex::new(r"^\s*#\s*@description\s+(.+)$")
        .context("Failed to compile description regex pattern")?;
    let ignore_re =
        Regex::new(r"^\s*#\s*@ignore\s*$").context("Failed to compile ignore regex pattern")?;
    let comment_re = Regex::new(r"^\s*#").context("Failed to compile comment regex pattern")?;

    // Iterate through lines to find function definitions
    for (line_idx, line) in lines.iter().enumerate() {
        if let Some(cap) = func_re.captures(line) {
            let func_name = &cap[1];

            // Validate function name format (should be valid bash identifier)
            if !is_valid_bash_identifier(func_name) {
                continue;
            }

            // Extract metadata from preceding comment lines
            let mut emoji: Option<String> = None;
            let mut description: Option<String> = None;
            let mut ignored = false;

            // Look backwards from the function line, but only through consecutive comment lines
            let mut check_idx = line_idx.saturating_sub(1);
            loop {
                if check_idx >= line_idx {
                    break; // Underflow protection
                }

                let prev_line = lines[check_idx];

                // If we hit a non-comment, non-empty line, stop looking back
                if !prev_line.trim().is_empty() && !comment_re.is_match(prev_line) {
                    break;
                }

                // Check for ignore annotation
                if ignore_re.is_match(prev_line) {
                    ignored = true;
                }

                // Check for emoji annotation
                if let Some(emoji_cap) = emoji_re.captures(prev_line) {
                    emoji = Some(emoji_cap[1].trim().to_string());
                }

                // Check for description annotation
                if let Some(desc_cap) = desc_re.captures(prev_line) {
                    description = Some(desc_cap[1].trim().to_string());
                }

                if check_idx == 0 {
                    break;
                }
                check_idx -= 1;
            }

            // Auto-generate display name from function name
            let display_name = format_display_name(func_name);

            // Use custom description if provided, otherwise generate default
            let final_description =
                description.unwrap_or_else(|| format!("Execute: {}", display_name));

            functions.push(ScriptFunction {
                name: func_name.to_string(),
                display_name: display_name.clone(),
                category: category.to_string(),
                description: final_description,
                emoji,
                ignored,
                script_type: ScriptType::Bash,
            });
        }
    }

    Ok(functions)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_parse_script_valid_single_function() {
        let temp_dir = TempDir::new().unwrap();
        let script_path = temp_dir.path().join("test.sh");

        let content = r#"#!/bin/bash

hello_world() {
    echo "Hello"
}
"#;
        fs::write(&script_path, content).unwrap();

        let result = parse_script(&script_path, "Test").unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].name, "hello_world");
        assert_eq!(result[0].display_name, "Hello World");
        assert_eq!(result[0].category, "Test");
    }

    #[test]
    fn test_parse_script_multiple_functions() {
        let temp_dir = TempDir::new().unwrap();
        let script_path = temp_dir.path().join("test.sh");

        let content = r#"#!/bin/bash

func_one() {
    echo "one"
}

func_two() {
    echo "two"
}

func_three() {
    echo "three"
}
"#;
        fs::write(&script_path, content).unwrap();

        let result = parse_script(&script_path, "Test").unwrap();
        assert_eq!(result.len(), 3);
        assert_eq!(result[0].name, "func_one");
        assert_eq!(result[0].display_name, "Func One");
        assert_eq!(result[1].name, "func_two");
        assert_eq!(result[1].display_name, "Func Two");
        assert_eq!(result[2].name, "func_three");
        assert_eq!(result[2].display_name, "Func Three");
    }

    #[test]
    fn test_parse_script_function_keyword() {
        let temp_dir = TempDir::new().unwrap();
        let script_path = temp_dir.path().join("test.sh");

        let content = r#"#!/bin/bash

function first() {
    echo "first"
}

function second() {
    echo "second"
}
"#;
        fs::write(&script_path, content).unwrap();

        let result = parse_script(&script_path, "Test").unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].name, "first");
        assert_eq!(result[0].display_name, "First");
        assert_eq!(result[1].name, "second");
        assert_eq!(result[1].display_name, "Second");
    }

    #[test]
    fn test_parse_script_no_functions() {
        let temp_dir = TempDir::new().unwrap();
        let script_path = temp_dir.path().join("test.sh");

        let content = r#"#!/bin/bash
echo "Just a script"
"#;
        fs::write(&script_path, content).unwrap();

        let result = parse_script(&script_path, "Test").unwrap();
        assert_eq!(result.len(), 0);
    }

    #[test]
    fn test_parse_script_empty_file() {
        let temp_dir = TempDir::new().unwrap();
        let script_path = temp_dir.path().join("test.sh");

        let content = "";
        fs::write(&script_path, content).unwrap();

        let result = parse_script(&script_path, "Test").unwrap();
        assert_eq!(result.len(), 0);
    }

    #[test]
    fn test_parse_script_file_not_found() {
        let temp_dir = TempDir::new().unwrap();
        let script_path = temp_dir.path().join("nonexistent.sh");

        let result = parse_script(&script_path, "Test");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_script_underscore_functions() {
        let temp_dir = TempDir::new().unwrap();
        let script_path = temp_dir.path().join("test.sh");

        let content = r#"#!/bin/bash

my_custom_function() {
    echo "custom"
}

another_test_func() {
    echo "test"
}
"#;
        fs::write(&script_path, content).unwrap();

        let result = parse_script(&script_path, "Test").unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].name, "my_custom_function");
        assert_eq!(result[0].display_name, "My Custom Function");
        assert_eq!(result[1].name, "another_test_func");
        assert_eq!(result[1].display_name, "Another Test Func");
    }

    #[test]
    fn test_parse_script_mixed_styles() {
        let temp_dir = TempDir::new().unwrap();
        let script_path = temp_dir.path().join("test.sh");

        let content = r#"#!/bin/bash

# Function with 'function' keyword
function with_keyword() {
    echo "keyword"
}

# Function without 'function' keyword
without_keyword() {
    echo "no keyword"
}

# Another with keyword
function another_with_keyword() {
    echo "another"
}
"#;
        fs::write(&script_path, content).unwrap();

        let result = parse_script(&script_path, "Test").unwrap();
        assert_eq!(result.len(), 3);
        assert_eq!(result[0].name, "with_keyword");
        assert_eq!(result[1].name, "without_keyword");
        assert_eq!(result[2].name, "another_with_keyword");
    }

    #[test]
    fn test_parse_script_whitespace_variations() {
        let temp_dir = TempDir::new().unwrap();
        let script_path = temp_dir.path().join("test.sh");

        let content = r#"#!/bin/bash

# Various whitespace patterns
func1(){
    echo "no spaces"
}

func2() {
    echo "space before brace"
}

func3()  {
    echo "multiple spaces"
}

function func4(){
    echo "keyword no spaces"
}
"#;
        fs::write(&script_path, content).unwrap();

        let result = parse_script(&script_path, "Test").unwrap();
        assert_eq!(result.len(), 4);
        assert_eq!(result[0].name, "func1");
        assert_eq!(result[1].name, "func2");
        assert_eq!(result[2].name, "func3");
        assert_eq!(result[3].name, "func4");
    }

    #[test]
    fn test_parse_script_with_emoji_annotation() {
        let temp_dir = TempDir::new().unwrap();
        let script_path = temp_dir.path().join("test.sh");

        let content = r#"#!/bin/bash

# @emoji 🚀
# @description Deploy the application to production
deploy_app() {
    echo "Deploying..."
}

# No annotations
simple_func() {
    echo "Simple"
}

# @emoji 🧪
test_suite() {
    echo "Testing..."
}
"#;
        fs::write(&script_path, content).unwrap();

        let result = parse_script(&script_path, "Test").unwrap();

        assert_eq!(result.len(), 3);

        // First function with both emoji and description
        assert_eq!(result[0].name, "deploy_app");
        assert_eq!(result[0].emoji, Some("🚀".to_string()));
        assert_eq!(
            result[0].description,
            "Deploy the application to production"
        );

        // Second function without annotations
        assert_eq!(result[1].name, "simple_func");
        assert_eq!(result[1].emoji, None);
        assert_eq!(result[1].description, "Execute: Simple Func");

        // Third function with emoji only
        assert_eq!(result[2].name, "test_suite");
        assert_eq!(result[2].emoji, Some("🧪".to_string()));
        assert_eq!(result[2].description, "Execute: Test Suite");
    }

    #[test]
    fn test_parse_script_description_only() {
        let temp_dir = TempDir::new().unwrap();
        let script_path = temp_dir.path().join("test.sh");

        let content = r#"#!/bin/bash

# @description This is a custom description
my_function() {
    echo "Hello"
}
"#;
        fs::write(&script_path, content).unwrap();

        let result = parse_script(&script_path, "Test").unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].name, "my_function");
        assert_eq!(result[0].emoji, None);
        assert_eq!(result[0].description, "This is a custom description");
    }

    #[test]
    fn test_parse_script_annotations_with_spacing() {
        let temp_dir = TempDir::new().unwrap();
        let script_path = temp_dir.path().join("test.sh");

        let content = r#"#!/bin/bash

#@emoji    💾  
#    @description    Backup database with extra spaces   
backup_db() {
    echo "Backing up..."
}
"#;
        fs::write(&script_path, content).unwrap();

        let result = parse_script(&script_path, "Test").unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].name, "backup_db");
        assert_eq!(result[0].emoji, Some("💾".to_string()));
        assert_eq!(result[0].description, "Backup database with extra spaces");
    }

    #[test]
    fn test_parse_script_ignore_annotation() {
        let temp_dir = TempDir::new().unwrap();
        let script_path = temp_dir.path().join("test.sh");

        let content = r#"#!/bin/bash

# @ignore
format_string() {
    echo "Utility function"
}

public_function() {
    echo "Public function"
}
"#;
        fs::write(&script_path, content).unwrap();

        let result = parse_script(&script_path, "Test").unwrap();
        assert_eq!(result.len(), 2);

        // First function should be ignored
        let format_func = result.iter().find(|f| f.name == "format_string").unwrap();
        assert_eq!(format_func.ignored, true);

        // Second function should not be ignored
        let public_func = result.iter().find(|f| f.name == "public_function").unwrap();
        assert_eq!(public_func.ignored, false);
    }

    #[test]
    fn test_parse_script_ignore_with_other_annotations() {
        let temp_dir = TempDir::new().unwrap();
        let script_path = temp_dir.path().join("test.sh");

        let content = r#"#!/bin/bash

# @ignore
# @emoji 🔧
# @description Helper function for string formatting
_helper_function() {
    echo "Helper"
}

# @emoji 🚀
# @description Main deployment function
deploy() {
    echo "Deploying..."
}
"#;
        fs::write(&script_path, content).unwrap();

        let result = parse_script(&script_path, "Test").unwrap();
        assert_eq!(result.len(), 2);

        // Helper function should be ignored but still have metadata
        let helper = result
            .iter()
            .find(|f| f.name == "_helper_function")
            .unwrap();
        assert_eq!(helper.ignored, true);
        assert_eq!(helper.emoji, Some("🔧".to_string()));
        assert_eq!(helper.description, "Helper function for string formatting");

        // Deploy function should not be ignored
        let deploy = result.iter().find(|f| f.name == "deploy").unwrap();
        assert_eq!(deploy.ignored, false);
        assert_eq!(deploy.emoji, Some("🚀".to_string()));
        assert_eq!(deploy.description, "Main deployment function");
    }

    #[test]
    fn test_parse_script_multiple_ignored_functions() {
        let temp_dir = TempDir::new().unwrap();
        let script_path = temp_dir.path().join("test.sh");

        let content = r#"#!/bin/bash

# @ignore
_private_helper() {
    echo "Private"
}

# @ignore
validate_input() {
    echo "Validate"
}

main_function() {
    echo "Main"
}

# @ignore
another_helper() {
    echo "Helper"
}
"#;
        fs::write(&script_path, content).unwrap();

        let result = parse_script(&script_path, "Test").unwrap();
        assert_eq!(result.len(), 4);

        // Count ignored functions
        let ignored_count = result.iter().filter(|f| f.ignored).count();
        assert_eq!(ignored_count, 3);

        // Verify main_function is not ignored
        let main = result.iter().find(|f| f.name == "main_function").unwrap();
        assert_eq!(main.ignored, false);
    }
}
