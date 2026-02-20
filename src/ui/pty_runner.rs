//! # PTY-based Command Runner
//!
//! This module provides PTY (pseudo-terminal) based command execution for
//! running commands inline within the TUI. Instead of suspending the TUI and
//! handing control to an external terminal, commands run inside a PTY and
//! their output is captured into a virtual terminal buffer.
//!
//! ## Architecture
//!
//! - Uses `portable-pty` to spawn commands in a pseudo-terminal
//! - Uses `vt100` to parse ANSI escape sequences and maintain terminal state
//! - Output is read from the PTY in a background thread and fed to the vt100 parser
//! - The main event loop polls for new output and renders the virtual terminal

use crate::script::{self, ScriptFile, ScriptFunction, ScriptType};
use anyhow::{Context, Result};
use portable_pty::{CommandBuilder, NativePtySystem, PtySize, PtySystem};
use std::io::{Read, Write};
use std::sync::{Arc, Mutex};
use std::time::Instant;

/// Status of a command execution
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ExecutionStatus {
    /// No command has been run for this target
    Idle,
    /// Command is currently running
    Running,
    /// Command completed with exit code 0
    Succeeded,
    /// Command completed with non-zero exit code
    Failed,
}

/// State of a single command execution, including the virtual terminal buffer
pub struct ExecutionState {
    pub status: ExecutionStatus,
    /// The vt100 parser that maintains the virtual terminal screen
    pub parser: Arc<Mutex<vt100::Parser>>,
    pub exit_code: Option<i32>,
    pub started_at: Instant,
    pub finished_at: Option<Instant>,
    pub display_name: String,
    pub category: String,
}

/// Session-scoped command history, keyed by a unique target identifier
pub struct CommandHistory {
    pub entries: std::collections::HashMap<String, ExecutionState>,
}

impl Default for CommandHistory {
    fn default() -> Self {
        Self::new()
    }
}

impl CommandHistory {
    pub fn new() -> Self {
        Self {
            entries: std::collections::HashMap::new(),
        }
    }

    /// Get a unique key for a function
    pub fn key_for(func: &ScriptFunction) -> String {
        format!("{:?}:{}", func.script_type, func.name)
    }

    /// Get the execution state for a given function, if any
    pub fn get(&self, func: &ScriptFunction) -> Option<&ExecutionState> {
        let key = Self::key_for(func);
        self.entries.get(&key)
    }

    /// Get the execution state mutably for a given function, if any
    pub fn get_mut(&mut self, func: &ScriptFunction) -> Option<&mut ExecutionState> {
        let key = Self::key_for(func);
        self.entries.get_mut(&key)
    }

    /// Insert or replace the execution state for a function
    pub fn insert(&mut self, func: &ScriptFunction, state: ExecutionState) {
        let key = Self::key_for(func);
        self.entries.insert(key, state);
    }
}

/// Escape a string for safe inclusion in a single-quoted shell argument.
///
/// This wraps the value in single quotes and escapes any embedded single
/// quotes using the `'\''` idiom (end quote, escaped literal quote, start
/// quote). Single-quoting prevents all shell expansions (`$`, `` ` ``,
/// `\`, `"`, etc.).
fn shell_escape(s: &str) -> String {
    format!("'{}'", s.replace('\'', "'\\''"))
}

/// Build the command to execute for a given script function and its script file.
/// Returns (program, args, `working_dir`).
fn build_command(
    func: &ScriptFunction,
    script_file: &ScriptFile,
) -> Result<(String, Vec<String>, std::path::PathBuf)> {
    let path = &script_file.path;

    match script_file.script_type {
        ScriptType::Bash => {
            let script_dir = path
                .parent()
                .context("Failed to get parent directory")?
                .to_path_buf();
            let script_name = path
                .file_name()
                .and_then(|n| n.to_str())
                .context("Invalid script filename")?;
            let bash_cmd = format!(
                "cd {} && source {} && {}",
                shell_escape(&script_dir.display().to_string()),
                shell_escape(script_name),
                func.name
            );
            Ok((
                "bash".to_string(),
                vec!["-c".to_string(), bash_cmd],
                script_dir,
            ))
        }
        ScriptType::PackageJson => {
            let dir = path
                .parent()
                .context("Failed to get parent dir")?
                .to_path_buf();
            Ok((
                "npm".to_string(),
                vec!["run".to_string(), func.name.clone()],
                dir,
            ))
        }
        ScriptType::DevboxJson => {
            let dir = path
                .parent()
                .context("Failed to get parent dir")?
                .to_path_buf();
            Ok((
                "devbox".to_string(),
                vec!["run".to_string(), func.name.clone()],
                dir,
            ))
        }
        ScriptType::Task => {
            let dir = path
                .parent()
                .context("Failed to get parent dir")?
                .to_path_buf();
            Ok((
                "task".to_string(),
                vec![
                    "--taskfile".to_string(),
                    path.display().to_string(),
                    func.name.clone(),
                ],
                dir,
            ))
        }
        ScriptType::Makefile => {
            let dir = path
                .parent()
                .context("Failed to get parent dir")?
                .to_path_buf();
            Ok((
                "make".to_string(),
                vec![
                    "--file".to_string(),
                    path.display().to_string(),
                    func.name.clone(),
                ],
                dir,
            ))
        }
        ScriptType::Just => {
            let dir = path
                .parent()
                .context("Failed to get parent dir")?
                .to_path_buf();
            Ok((
                "just".to_string(),
                vec![
                    "--justfile".to_string(),
                    path.display().to_string(),
                    func.name.clone(),
                ],
                dir,
            ))
        }
        ScriptType::CargoToml => {
            let dir = path
                .parent()
                .context("Failed to get parent dir")?
                .to_path_buf();
            let (flag, name) = if let Some(stripped) = func.name.strip_prefix("bin:") {
                ("--bin", stripped)
            } else if let Some(stripped) = func.name.strip_prefix("example:") {
                ("--example", stripped)
            } else {
                ("--bin", func.name.as_str())
            };
            Ok((
                "cargo".to_string(),
                vec![
                    "run".to_string(),
                    flag.to_string(),
                    name.to_string(),
                    "--manifest-path".to_string(),
                    path.display().to_string(),
                ],
                dir,
            ))
        }
        ScriptType::NxJson => {
            let dir = path
                .parent()
                .context("Failed to get parent dir")?
                .to_path_buf();
            let (cmd, base_args) = script::nx_parser::nx_command();
            let mut args: Vec<String> = base_args.iter().map(|s| (*s).to_string()).collect();
            args.push("run".to_string());
            args.push(func.name.clone());
            Ok((cmd.to_string(), args, dir))
        }
        ScriptType::Terraform => {
            // For Terraform, the ScriptFile path is the directory containing .tf files
            let dir = path.clone();
            let args: Vec<String> = func.name.split_whitespace().map(String::from).collect();
            let binary = script::terraform_parser::resolve_terraform_binary()
                .unwrap_or("terraform")
                .to_string();
            Ok((binary, args, dir))
        }
        ScriptType::Gradle => {
            let dir = path
                .parent()
                .context("Failed to get parent dir")?
                .to_path_buf();
            let gradle_cmd = script::gradle_parser::get_gradle_command(&dir)
                .unwrap_or_else(|| "gradle".to_string());
            Ok((gradle_cmd, vec![func.name.clone()], dir))
        }
    }
}

/// Find the matching `ScriptFile` for a function, handling the Frequently Used
/// category indirection and Nx per-project category matching.
pub fn find_script_file<'a>(
    func: &ScriptFunction,
    original_category: &str,
    script_files: &'a [ScriptFile],
) -> Option<&'a ScriptFile> {
    script_files.iter().find(|s| {
        if s.script_type != func.script_type {
            return false;
        }
        if s.script_type == ScriptType::NxJson {
            let prefix = format!("nx:{}:", s.category);
            original_category.starts_with(&prefix)
        } else {
            s.category == *original_category
        }
    })
}

/// A handle to a running PTY process that can be polled for status updates
pub struct PtyHandle {
    pub parser: Arc<Mutex<vt100::Parser>>,
    pub status: Arc<Mutex<ExecutionStatus>>,
    pub exit_code: Arc<Mutex<Option<i32>>>,
    pub finished_at: Arc<Mutex<Option<Instant>>>,
    pub started_at: Instant,
    pub display_name: String,
    pub category: String,
    // Keep the master alive so the PTY doesn't close prematurely
    _master: Arc<Mutex<Option<Box<dyn portable_pty::MasterPty + Send>>>>,
    /// Writer to send input to the PTY slave (child process stdin)
    writer: Arc<Mutex<Option<Box<dyn Write + Send>>>>,
}

impl PtyHandle {
    /// Poll the current execution status
    pub fn poll_status(&self) -> ExecutionStatus {
        self.status
            .lock()
            .map(|s| *s)
            .unwrap_or(ExecutionStatus::Failed)
    }

    /// Get the exit code if the process has finished
    pub fn poll_exit_code(&self) -> Option<i32> {
        self.exit_code.lock().ok().and_then(|ec| *ec)
    }

    /// Get the finished timestamp if the process has finished
    pub fn poll_finished_at(&self) -> Option<Instant> {
        self.finished_at.lock().ok().and_then(|f| *f)
    }

    /// Write input bytes to the PTY (sends to child process stdin)
    pub fn write_input(&self, data: &[u8]) -> Result<()> {
        if let Ok(mut writer_guard) = self.writer.lock() {
            if let Some(ref mut writer) = *writer_guard {
                writer.write_all(data).context("Failed to write to PTY")?;
                writer.flush().context("Failed to flush PTY writer")?;
            }
        }
        Ok(())
    }

    /// Convert into an `ExecutionState` for storage in history
    pub fn into_execution_state(self) -> ExecutionState {
        let status = self.poll_status();
        let exit_code = self.poll_exit_code();
        let finished_at = self.poll_finished_at();
        ExecutionState {
            status,
            parser: self.parser,
            exit_code,
            started_at: self.started_at,
            finished_at,
            display_name: self.display_name,
            category: self.category,
        }
    }
}

/// Spawn a command in a PTY and return a handle for polling.
/// This is the primary API — it returns a `PtyHandle` that can be polled
/// for status, exit code, and terminal output.
pub fn spawn_pty_command(
    func: &ScriptFunction,
    script_file: &ScriptFile,
    original_category: &str,
    cols: u16,
    rows: u16,
) -> Result<PtyHandle> {
    let (program, args, working_dir) = build_command(func, script_file)?;

    let pty_system = NativePtySystem::default();

    let pty_pair = pty_system
        .openpty(PtySize {
            rows,
            cols,
            pixel_width: 0,
            pixel_height: 0,
        })
        .context("Failed to open PTY")?;

    let mut cmd = CommandBuilder::new(&program);
    for arg in &args {
        cmd.arg(arg);
    }
    cmd.cwd(&working_dir);

    let child = pty_pair
        .slave
        .spawn_command(cmd)
        .context("Failed to spawn command in PTY")?;

    // Drop the slave side — we only need the master for I/O
    drop(pty_pair.slave);

    let parser = Arc::new(Mutex::new(vt100::Parser::new(rows, cols, 10000)));
    let status = Arc::new(Mutex::new(ExecutionStatus::Running));
    let exit_code: Arc<Mutex<Option<i32>>> = Arc::new(Mutex::new(None));
    let finished_at: Arc<Mutex<Option<Instant>>> = Arc::new(Mutex::new(None));

    let mut reader = pty_pair
        .master
        .try_clone_reader()
        .context("Failed to clone PTY reader")?;

    let master: Arc<Mutex<Option<Box<dyn portable_pty::MasterPty + Send>>>> =
        Arc::new(Mutex::new(Some(pty_pair.master)));

    // Extract the writer for sending input to the child process
    let writer: Arc<Mutex<Option<Box<dyn Write + Send>>>> = {
        let master_guard = master
            .lock()
            .map_err(|e| anyhow::anyhow!("Failed to lock PTY master for writer: {}", e))?;
        let w = master_guard.as_ref().and_then(|m| m.take_writer().ok());
        Arc::new(Mutex::new(w))
    };

    // Reader thread
    let parser_clone = Arc::clone(&parser);
    let master_reader_clone = Arc::clone(&master);
    std::thread::spawn(move || {
        let mut buf = [0u8; 4096];
        loop {
            match reader.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => {
                    if let Ok(mut p) = parser_clone.lock() {
                        p.process(&buf[..n]);
                    }
                }
                Err(_) => break,
            }
        }
        // Drop master to close PTY when reading is done
        if let Ok(mut m) = master_reader_clone.lock() {
            m.take();
        }
    });

    // Child waiter thread
    let status_clone = Arc::clone(&status);
    let exit_clone = Arc::clone(&exit_code);
    let finished_clone = Arc::clone(&finished_at);
    let child = Arc::new(Mutex::new(child));
    let child_clone = Arc::clone(&child);
    std::thread::spawn(move || {
        if let Ok(mut c) = child_clone.lock() {
            match c.wait() {
                Ok(exit_status) => {
                    let code: i32 = exit_status.exit_code().try_into().unwrap_or(1);
                    if let Ok(mut ec) = exit_clone.lock() {
                        *ec = Some(code);
                    }
                    if let Ok(mut s) = status_clone.lock() {
                        *s = if code == 0 {
                            ExecutionStatus::Succeeded
                        } else {
                            ExecutionStatus::Failed
                        };
                    }
                    if let Ok(mut f) = finished_clone.lock() {
                        *f = Some(Instant::now());
                    }
                }
                Err(_) => {
                    if let Ok(mut ec) = exit_clone.lock() {
                        *ec = Some(1);
                    }
                    if let Ok(mut s) = status_clone.lock() {
                        *s = ExecutionStatus::Failed;
                    }
                    if let Ok(mut f) = finished_clone.lock() {
                        *f = Some(Instant::now());
                    }
                }
            }
        }
    });

    Ok(PtyHandle {
        parser,
        status,
        exit_code,
        finished_at,
        started_at: Instant::now(),
        display_name: func.display_name.clone(),
        category: original_category.to_string(),
        _master: master,
        writer,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::script::{ScriptFile, ScriptFunction, ScriptType};
    use std::path::PathBuf;

    // --- shell_escape tests ---

    #[test]
    fn test_shell_escape_simple() {
        assert_eq!(shell_escape("hello"), "'hello'");
    }

    #[test]
    fn test_shell_escape_with_spaces() {
        assert_eq!(shell_escape("hello world"), "'hello world'");
    }

    #[test]
    fn test_shell_escape_with_double_quotes() {
        assert_eq!(shell_escape(r#"say "hi""#), r#"'say "hi"'"#);
    }

    #[test]
    fn test_shell_escape_with_single_quotes() {
        assert_eq!(shell_escape("it's"), "'it'\\''s'");
    }

    #[test]
    fn test_shell_escape_with_dollar_and_backtick() {
        assert_eq!(shell_escape("$HOME `whoami`"), "'$HOME `whoami`'");
    }

    #[test]
    fn test_shell_escape_with_backslash() {
        assert_eq!(shell_escape(r"path\to\file"), r"'path\to\file'");
    }

    #[test]
    fn test_shell_escape_empty() {
        assert_eq!(shell_escape(""), "''");
    }

    // --- build_command tests ---

    fn make_func(name: &str, script_type: ScriptType) -> ScriptFunction {
        ScriptFunction {
            name: name.to_string(),
            display_name: name.to_string(),
            category: "Test".to_string(),
            description: String::new(),
            emoji: None,
            ignored: false,
            script_type,
        }
    }

    fn make_script_file(path: &str, script_type: ScriptType) -> ScriptFile {
        ScriptFile {
            path: PathBuf::from(path),
            name: PathBuf::from(path)
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default(),
            category: "Test".to_string(),
            display_name: "Test".to_string(),
            script_type,
        }
    }

    #[test]
    fn test_build_command_bash() {
        let func = make_func("deploy", ScriptType::Bash);
        let sf = make_script_file("/home/user/scripts/deploy.sh", ScriptType::Bash);

        let (program, args, cwd) = build_command(&func, &sf).unwrap();

        assert_eq!(program, "bash");
        assert_eq!(args.len(), 2);
        assert_eq!(args[0], "-c");
        assert!(args[1].contains("deploy"));
        assert!(args[1].contains("source"));
        assert_eq!(cwd, PathBuf::from("/home/user/scripts"));
    }

    #[test]
    fn test_build_command_bash_shell_escapes_paths() {
        let func = make_func("run", ScriptType::Bash);
        let sf = make_script_file("/home/user/my scripts/test's.sh", ScriptType::Bash);

        let (_, args, _) = build_command(&func, &sf).unwrap();
        let bash_cmd = &args[1];

        // The command should use single-quote escaping, not double quotes
        assert!(
            bash_cmd.contains("'"),
            "Expected single-quote escaping in: {}",
            bash_cmd
        );
        // Verify the single quote in the filename is properly escaped
        assert!(
            bash_cmd.contains("'\\''"),
            "Expected escaped single quote in: {}",
            bash_cmd
        );
    }

    #[test]
    fn test_build_command_npm() {
        let func = make_func("build", ScriptType::PackageJson);
        let sf = make_script_file("/app/package.json", ScriptType::PackageJson);

        let (program, args, cwd) = build_command(&func, &sf).unwrap();

        assert_eq!(program, "npm");
        assert_eq!(args, vec!["run", "build"]);
        assert_eq!(cwd, PathBuf::from("/app"));
    }

    #[test]
    fn test_build_command_devbox() {
        let func = make_func("start", ScriptType::DevboxJson);
        let sf = make_script_file("/app/devbox.json", ScriptType::DevboxJson);

        let (program, args, cwd) = build_command(&func, &sf).unwrap();

        assert_eq!(program, "devbox");
        assert_eq!(args, vec!["run", "start"]);
        assert_eq!(cwd, PathBuf::from("/app"));
    }

    #[test]
    fn test_build_command_task() {
        let func = make_func("lint", ScriptType::Task);
        let sf = make_script_file("/app/Taskfile.yml", ScriptType::Task);

        let (program, args, cwd) = build_command(&func, &sf).unwrap();

        assert_eq!(program, "task");
        assert_eq!(args, vec!["--taskfile", "/app/Taskfile.yml", "lint"]);
        assert_eq!(cwd, PathBuf::from("/app"));
    }

    #[test]
    fn test_build_command_makefile() {
        let func = make_func("clean", ScriptType::Makefile);
        let sf = make_script_file("/app/Makefile", ScriptType::Makefile);

        let (program, args, cwd) = build_command(&func, &sf).unwrap();

        assert_eq!(program, "make");
        assert_eq!(args, vec!["--file", "/app/Makefile", "clean"]);
        assert_eq!(cwd, PathBuf::from("/app"));
    }

    #[test]
    fn test_build_command_just() {
        let func = make_func("test", ScriptType::Just);
        let sf = make_script_file("/app/justfile", ScriptType::Just);

        let (program, args, cwd) = build_command(&func, &sf).unwrap();

        assert_eq!(program, "just");
        assert_eq!(args, vec!["--justfile", "/app/justfile", "test"]);
        assert_eq!(cwd, PathBuf::from("/app"));
    }

    #[test]
    fn test_build_command_cargo_bin() {
        let func = make_func("bin:myapp", ScriptType::CargoToml);
        let sf = make_script_file("/app/Cargo.toml", ScriptType::CargoToml);

        let (program, args, cwd) = build_command(&func, &sf).unwrap();

        assert_eq!(program, "cargo");
        assert_eq!(
            args,
            vec![
                "run",
                "--bin",
                "myapp",
                "--manifest-path",
                "/app/Cargo.toml"
            ]
        );
        assert_eq!(cwd, PathBuf::from("/app"));
    }

    #[test]
    fn test_build_command_cargo_example() {
        let func = make_func("example:demo", ScriptType::CargoToml);
        let sf = make_script_file("/app/Cargo.toml", ScriptType::CargoToml);

        let (program, args, cwd) = build_command(&func, &sf).unwrap();

        assert_eq!(program, "cargo");
        assert_eq!(
            args,
            vec![
                "run",
                "--example",
                "demo",
                "--manifest-path",
                "/app/Cargo.toml"
            ]
        );
        assert_eq!(cwd, PathBuf::from("/app"));
    }

    // --- CommandHistory tests ---

    #[test]
    fn test_command_history_new() {
        let history = CommandHistory::new();
        assert!(history.entries.is_empty());
    }

    #[test]
    fn test_command_history_key_for() {
        let func = make_func("deploy", ScriptType::Bash);
        let key = CommandHistory::key_for(&func);
        assert_eq!(key, "Bash:deploy");
    }

    #[test]
    fn test_command_history_key_for_different_types() {
        let bash_func = make_func("build", ScriptType::Bash);
        let npm_func = make_func("build", ScriptType::PackageJson);

        let bash_key = CommandHistory::key_for(&bash_func);
        let npm_key = CommandHistory::key_for(&npm_func);

        // Same name, different type => different key
        assert_ne!(bash_key, npm_key);
    }

    #[test]
    fn test_command_history_insert_and_get() {
        let mut history = CommandHistory::new();
        let func = make_func("deploy", ScriptType::Bash);

        assert!(history.get(&func).is_none());

        let state = ExecutionState {
            status: ExecutionStatus::Succeeded,
            parser: Arc::new(Mutex::new(vt100::Parser::new(24, 80, 100))),
            exit_code: Some(0),
            started_at: Instant::now(),
            finished_at: Some(Instant::now()),
            display_name: "Deploy".to_string(),
            category: "Test".to_string(),
        };

        history.insert(&func, state);
        let retrieved = history.get(&func).unwrap();
        assert_eq!(retrieved.status, ExecutionStatus::Succeeded);
        assert_eq!(retrieved.exit_code, Some(0));
    }

    #[test]
    fn test_command_history_get_mut() {
        let mut history = CommandHistory::new();
        let func = make_func("test", ScriptType::Bash);

        let state = ExecutionState {
            status: ExecutionStatus::Running,
            parser: Arc::new(Mutex::new(vt100::Parser::new(24, 80, 100))),
            exit_code: None,
            started_at: Instant::now(),
            finished_at: None,
            display_name: "Test".to_string(),
            category: "Test".to_string(),
        };

        history.insert(&func, state);

        let entry = history.get_mut(&func).unwrap();
        entry.status = ExecutionStatus::Failed;
        entry.exit_code = Some(1);

        assert_eq!(history.get(&func).unwrap().status, ExecutionStatus::Failed);
    }

    #[test]
    fn test_command_history_replace() {
        let mut history = CommandHistory::new();
        let func = make_func("build", ScriptType::Bash);

        let state1 = ExecutionState {
            status: ExecutionStatus::Failed,
            parser: Arc::new(Mutex::new(vt100::Parser::new(24, 80, 100))),
            exit_code: Some(1),
            started_at: Instant::now(),
            finished_at: Some(Instant::now()),
            display_name: "Build".to_string(),
            category: "Test".to_string(),
        };
        history.insert(&func, state1);

        let state2 = ExecutionState {
            status: ExecutionStatus::Succeeded,
            parser: Arc::new(Mutex::new(vt100::Parser::new(24, 80, 100))),
            exit_code: Some(0),
            started_at: Instant::now(),
            finished_at: Some(Instant::now()),
            display_name: "Build".to_string(),
            category: "Test".to_string(),
        };
        history.insert(&func, state2);

        // Should have the latest state
        assert_eq!(
            history.get(&func).unwrap().status,
            ExecutionStatus::Succeeded
        );
        assert_eq!(history.entries.len(), 1);
    }

    // --- find_script_file tests ---

    #[test]
    fn test_find_script_file_bash() {
        let func = make_func("deploy", ScriptType::Bash);
        let files = vec![
            make_script_file("/app/Makefile", ScriptType::Makefile),
            make_script_file("/app/scripts/deploy.sh", ScriptType::Bash),
        ];

        let result = find_script_file(&func, "Test", &files);
        assert!(result.is_some());
        assert_eq!(result.unwrap().script_type, ScriptType::Bash);
    }

    #[test]
    fn test_find_script_file_no_match_wrong_type() {
        let func = make_func("deploy", ScriptType::Bash);
        let files = vec![make_script_file(
            "/app/package.json",
            ScriptType::PackageJson,
        )];

        let result = find_script_file(&func, "Test", &files);
        assert!(result.is_none());
    }

    #[test]
    fn test_find_script_file_no_match_wrong_category() {
        let func = make_func("deploy", ScriptType::Bash);
        let mut sf = make_script_file("/app/deploy.sh", ScriptType::Bash);
        sf.category = "Other".to_string();
        let files = vec![sf];

        let result = find_script_file(&func, "Test", &files);
        assert!(result.is_none());
    }

    #[test]
    fn test_find_script_file_nx_prefix_match() {
        let func = make_func("my-app:build", ScriptType::NxJson);
        let mut sf = make_script_file("/app/nx.json", ScriptType::NxJson);
        sf.category = "my-app".to_string();
        let files = vec![sf];

        // Nx categories use the "nx:<project>:" prefix format
        let result = find_script_file(&func, "nx:my-app:targets", &files);
        assert!(result.is_some());
    }

    #[test]
    fn test_find_script_file_empty_list() {
        let func = make_func("deploy", ScriptType::Bash);
        let result = find_script_file(&func, "Test", &[]);
        assert!(result.is_none());
    }

    // --- Terraform build_command tests ---

    #[test]
    fn test_build_command_terraform_common() {
        let func = make_func("plan", ScriptType::Terraform);
        let sf = make_script_file("/infra/terraform", ScriptType::Terraform);

        let (program, args, cwd) = build_command(&func, &sf).unwrap();

        // Binary may be "terraform" or "tofu" depending on what's installed
        assert!(
            program == "terraform" || program == "tofu",
            "Expected 'terraform' or 'tofu', got '{}'",
            program
        );
        assert_eq!(args, vec!["plan"]);
        assert_eq!(cwd, PathBuf::from("/infra/terraform"));
    }

    #[test]
    fn test_build_command_terraform_workspace_select() {
        let func = make_func("workspace select staging", ScriptType::Terraform);
        let sf = make_script_file("/infra/terraform", ScriptType::Terraform);

        let (program, args, cwd) = build_command(&func, &sf).unwrap();

        assert!(
            program == "terraform" || program == "tofu",
            "Expected 'terraform' or 'tofu', got '{}'",
            program
        );
        assert_eq!(args, vec!["workspace", "select", "staging"]);
        assert_eq!(cwd, PathBuf::from("/infra/terraform"));
    }

    #[test]
    fn test_build_command_terraform_init() {
        let func = make_func("init", ScriptType::Terraform);
        let sf = make_script_file("/infra", ScriptType::Terraform);

        let (program, args, cwd) = build_command(&func, &sf).unwrap();

        assert!(
            program == "terraform" || program == "tofu",
            "Expected 'terraform' or 'tofu', got '{}'",
            program
        );
        assert_eq!(args, vec!["init"]);
        assert_eq!(cwd, PathBuf::from("/infra"));
    }

    // --- ExecutionStatus tests ---

    #[test]
    fn test_execution_status_equality() {
        assert_eq!(ExecutionStatus::Idle, ExecutionStatus::Idle);
        assert_eq!(ExecutionStatus::Running, ExecutionStatus::Running);
        assert_eq!(ExecutionStatus::Succeeded, ExecutionStatus::Succeeded);
        assert_eq!(ExecutionStatus::Failed, ExecutionStatus::Failed);
        assert_ne!(ExecutionStatus::Idle, ExecutionStatus::Running);
    }
}
