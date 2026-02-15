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
