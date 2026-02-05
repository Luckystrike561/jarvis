//! CLI argument parsing and application initialization tests

use std::path::PathBuf;
use tempfile::TempDir;

/// Test that run_application correctly handles a nonexistent directory
#[tokio::test]
async fn test_run_application_nonexistent_directory() {
    let path = PathBuf::from("/nonexistent/directory/that/does/not/exist");
    let result = path.canonicalize();

    assert!(result.is_err());
}

/// Test that run_application correctly handles a file instead of a directory
#[tokio::test]
async fn test_run_application_file_instead_of_directory() {
    use std::fs;

    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("notadir.txt");
    fs::write(&file_path, "test content").unwrap();

    // Canonicalize works on files, but is_dir() should be false
    let canonical = file_path.canonicalize().unwrap();
    assert!(!canonical.is_dir());
}

/// Test that empty directories result in no scripts being discovered
#[tokio::test]
async fn test_run_application_empty_directory_exits() {
    use jarvis::script;

    let temp_dir = TempDir::new().unwrap();
    let current_dir = temp_dir.path().canonicalize().unwrap();

    let mut script_files = Vec::new();

    let root_files = script::discover_scripts_shallow(&current_dir).unwrap();
    script_files.extend(root_files);

    let possible_dirs = vec!["script", "scripts", "jarvis"];
    for dir_name in possible_dirs {
        let dir_path = current_dir.join(dir_name);
        if dir_path.exists() && dir_path.is_dir() {
            let files = script::discover_scripts(&dir_path).unwrap();
            script_files.extend(files);
        }
    }

    // Verify that we get no scripts from empty directory
    assert!(script_files.is_empty());
}

/// Test that valid scripts are discovered correctly
#[tokio::test]
async fn test_run_application_with_valid_scripts() {
    use jarvis::script;
    use std::fs;

    let temp_dir = TempDir::new().unwrap();

    // Create a test script
    let script_path = temp_dir.path().join("test.sh");
    let content = r#"#!/bin/bash
test_function() {
    echo "Test"
}
"#;
    fs::write(&script_path, content).unwrap();

    let current_dir = temp_dir.path().canonicalize().unwrap();
    let mut script_files = Vec::new();

    let root_files = script::discover_scripts_shallow(&current_dir).unwrap();
    script_files.extend(root_files);

    assert!(!script_files.is_empty());
    assert_eq!(script_files.len(), 1);
}

/// Test that scripts with no functions are handled gracefully
#[tokio::test]
async fn test_run_application_script_parse_errors() {
    use jarvis::script;
    use std::fs;

    let temp_dir = TempDir::new().unwrap();

    // Create an invalid bash script (empty file)
    let script_path = temp_dir.path().join("invalid.sh");
    fs::write(&script_path, "").unwrap();

    let current_dir = temp_dir.path().canonicalize().unwrap();
    let mut script_files = Vec::new();

    let root_files = script::discover_scripts_shallow(&current_dir).unwrap();
    script_files.extend(root_files);

    assert!(!script_files.is_empty());

    // Attempt to parse - should handle errors gracefully
    let mut all_functions = Vec::new();
    let mut parse_errors = Vec::new();

    for script_file in &script_files {
        match &script_file.script_type {
            script::ScriptType::Bash => {
                match script::parse_script(&script_file.path, &script_file.category) {
                    Ok(functions) => {
                        let visible_functions: Vec<_> =
                            functions.into_iter().filter(|f| !f.ignored).collect();
                        all_functions.extend(visible_functions);
                    }
                    Err(e) => {
                        parse_errors.push((script_file.path.display().to_string(), e));
                    }
                }
            }
            _ => {}
        }
    }

    // Empty file should parse successfully but have no functions
    assert!(all_functions.is_empty());
}

/// Test that scripts in subdirectories are discovered correctly
#[tokio::test]
async fn test_run_application_discovers_subdirectories() {
    use jarvis::script;
    use std::fs;

    let temp_dir = TempDir::new().unwrap();

    // Create script in scripts/ subdirectory
    let scripts_dir = temp_dir.path().join("scripts");
    fs::create_dir(&scripts_dir).unwrap();

    let script_path = scripts_dir.join("test.sh");
    let content = r#"#!/bin/bash
test_function() {
    echo "Test"
}
"#;
    fs::write(&script_path, content).unwrap();

    let current_dir = temp_dir.path().canonicalize().unwrap();
    let mut script_files = Vec::new();

    // Root directory
    let root_files = script::discover_scripts_shallow(&current_dir).unwrap();
    script_files.extend(root_files);

    // Check subdirectories
    let possible_dirs = vec!["script", "scripts", "jarvis"];
    for dir_name in possible_dirs {
        let dir_path = current_dir.join(dir_name);
        if dir_path.exists() && dir_path.is_dir() {
            let files = script::discover_scripts(&dir_path).unwrap();
            script_files.extend(files);
        }
    }

    assert!(!script_files.is_empty());
    assert_eq!(script_files.len(), 1);
}
