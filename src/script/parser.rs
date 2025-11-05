use anyhow::{Context, Result};
use regex::Regex;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct ScriptFunction {
    pub name: String,
    pub display_name: String,
    pub category: String,
    pub description: String,
}

pub fn parse_script(path: &Path, category: &str) -> Result<Vec<ScriptFunction>> {
    // Read script file with proper error context
    let content = fs::read_to_string(path)
        .with_context(|| format!("Failed to read script file: {}", path.display()))?;

    let mut functions = Vec::new();

    // Compile regex patterns with error handling
    // Note: We use a more flexible pattern to handle parentheses within quotes
    let array_re = Regex::new(r#"(\w+_functions)=\(\s*([\s\S]*?)\s*\)(?:\s|$)"#)
        .context("Failed to compile function array regex pattern")?;
    
    let item_re = Regex::new(r#""([^:]+):([^"]+)""#)
        .context("Failed to compile function item regex pattern")?;

    // Track if we found any function arrays
    let mut found_arrays = false;

    for cap in array_re.captures_iter(&content) {
        found_arrays = true;
        let array_name = &cap[1];
        let items = &cap[2];

        // Track if we found any valid items in this array
        let mut found_items = false;

        for item_cap in item_re.captures_iter(items) {
            let display_name = item_cap[1].trim();
            let func_name = item_cap[2].trim();

            // Validate that neither field is empty
            if display_name.is_empty() {
                eprintln!(
                    "Warning: Empty display name in {} ({})",
                    array_name,
                    path.display()
                );
                continue;
            }

            if func_name.is_empty() {
                eprintln!(
                    "Warning: Empty function name for '{}' in {} ({})",
                    display_name,
                    array_name,
                    path.display()
                );
                continue;
            }

            // Validate function name format (should be valid bash identifier)
            if !is_valid_bash_identifier(func_name) {
                eprintln!(
                    "Warning: Invalid function name '{}' in {} ({}). Must be a valid bash identifier.",
                    func_name,
                    array_name,
                    path.display()
                );
                continue;
            }

            found_items = true;
            functions.push(ScriptFunction {
                name: func_name.to_string(),
                display_name: display_name.to_string(),
                category: category.to_string(),
                description: format!("Execute: {}", display_name),
            });
        }

        if !found_items {
            eprintln!(
                "Warning: No valid function items found in {} ({})",
                array_name,
                path.display()
            );
        }
    }

    if !found_arrays {
        eprintln!(
            "Warning: No function arrays found in script: {}",
            path.display()
        );
        eprintln!("Expected format: script_functions=(\"Display:function\" ...)");
    }

    Ok(functions)
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
    fn test_parse_script_valid_single_function() {
        let temp_dir = TempDir::new().unwrap();
        let script_path = temp_dir.path().join("test.sh");
        
        let content = r#"#!/bin/bash
test_functions=(
    "Hello World:hello_world"
)

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
test_functions=(
    "Function One:func_one"
    "Function Two:func_two"
    "Function Three:func_three"
)
"#;
        fs::write(&script_path, content).unwrap();

        let result = parse_script(&script_path, "Test").unwrap();
        assert_eq!(result.len(), 3);
        assert_eq!(result[0].name, "func_one");
        assert_eq!(result[1].name, "func_two");
        assert_eq!(result[2].name, "func_three");
    }

    #[test]
    fn test_parse_script_multiple_arrays() {
        let temp_dir = TempDir::new().unwrap();
        let script_path = temp_dir.path().join("test.sh");
        
        let content = r#"#!/bin/bash
first_functions=(
    "First:first"
)

second_functions=(
    "Second:second"
)
"#;
        fs::write(&script_path, content).unwrap();

        let result = parse_script(&script_path, "Test").unwrap();
        assert_eq!(result.len(), 2);
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
    fn test_parse_script_empty_array() {
        let temp_dir = TempDir::new().unwrap();
        let script_path = temp_dir.path().join("test.sh");
        
        let content = r#"#!/bin/bash
test_functions=()
"#;
        fs::write(&script_path, content).unwrap();

        let result = parse_script(&script_path, "Test").unwrap();
        assert_eq!(result.len(), 0);
    }

    #[test]
    fn test_parse_script_whitespace_handling() {
        let temp_dir = TempDir::new().unwrap();
        let script_path = temp_dir.path().join("test.sh");
        
        let content = r#"#!/bin/bash
test_functions=(
    "  Trimmed Name  :  trimmed_func  "
)
"#;
        fs::write(&script_path, content).unwrap();

        let result = parse_script(&script_path, "Test").unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].display_name, "Trimmed Name");
        assert_eq!(result[0].name, "trimmed_func");
    }

    #[test]
    fn test_parse_script_file_not_found() {
        let temp_dir = TempDir::new().unwrap();
        let script_path = temp_dir.path().join("nonexistent.sh");

        let result = parse_script(&script_path, "Test");
        assert!(result.is_err());
    }

    #[test]
    fn test_is_valid_bash_identifier_valid() {
        assert!(is_valid_bash_identifier("valid_name"));
        assert!(is_valid_bash_identifier("_underscore"));
        assert!(is_valid_bash_identifier("name123"));
        assert!(is_valid_bash_identifier("CamelCase"));
        assert!(is_valid_bash_identifier("snake_case_123"));
    }

    #[test]
    fn test_is_valid_bash_identifier_invalid() {
        assert!(!is_valid_bash_identifier(""));
        assert!(!is_valid_bash_identifier("123start"));
        assert!(!is_valid_bash_identifier("has-dash"));
        assert!(!is_valid_bash_identifier("has space"));
        assert!(!is_valid_bash_identifier("has.dot"));
        assert!(!is_valid_bash_identifier("has$dollar"));
    }

    #[test]
    fn test_parse_script_invalid_function_names_skipped() {
        let temp_dir = TempDir::new().unwrap();
        let script_path = temp_dir.path().join("test.sh");
        
        let content = r#"#!/bin/bash
test_functions=(
    "Valid:valid_func"
    "Invalid:123invalid"
    "Also Valid:also_valid"
)
"#;
        fs::write(&script_path, content).unwrap();

        let result = parse_script(&script_path, "Test").unwrap();
        // Should only include the valid ones
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].name, "valid_func");
        assert_eq!(result[1].name, "also_valid");
    }

    #[test]
    fn test_parse_script_complex_display_names() {
        let temp_dir = TempDir::new().unwrap();
        let script_path = temp_dir.path().join("test.sh");
        
        let content = r#"#!/bin/bash
test_functions=(
    "🚀 Deploy to Production:deploy_prod"
    "Install System (Full):install_system"
)
"#;
        fs::write(&script_path, content).unwrap();

        let result = parse_script(&script_path, "Test").unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].display_name, "🚀 Deploy to Production");
        assert_eq!(result[1].display_name, "Install System (Full)");
    }
}
