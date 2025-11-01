use anyhow::Result;
use std::path::Path;
use std::process::{Command, Stdio};

/// Execute a bash function interactively with full terminal access
/// This allows the script to use stdin/stdout/stderr directly (for gum, etc)
pub fn execute_function_interactive(
    script_path: &Path,
    function_name: &str,
) -> Result<i32> {
    // Create a bash script that sources the file and calls the function
    let script_dir = script_path.parent().unwrap_or(Path::new("."));
    let script_name = script_path.file_name().unwrap().to_str().unwrap();
    
    let bash_command = format!(
        r#"cd "{}" && source "{}" && {}"#,
        script_dir.display(),
        script_name,
        function_name
    );
    
    // Execute with inherited stdin/stdout/stderr for full interactivity
    let status = Command::new("bash")
        .arg("-c")
        .arg(&bash_command)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()?;
    
    Ok(status.code().unwrap_or(1))
}
