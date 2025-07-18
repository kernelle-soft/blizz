use anyhow::{anyhow, Result};
use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::Path;
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;

pub type TasksFile = HashMap<String, String>;

#[derive(Debug)]
pub struct TaskRunnerOptions {
  pub silent: bool,
  pub tasks_file_path: Option<String>,
}

impl Default for TaskRunnerOptions {
  fn default() -> Self {
    Self { silent: false, tasks_file_path: None }
  }
}

#[derive(Debug)]
pub struct TaskResult {
  pub stdout: String,
  pub stderr: String,
  pub success: bool,
  pub exit_code: Option<i32>,
}

pub async fn run_task(
  alias: &str,
  args: &[String],
  options: TaskRunnerOptions,
) -> Result<TaskResult> {
  let tasks_file_path = options.tasks_file_path.unwrap_or_else(|| "./.cursor/kernelle.tasks".to_string());
  let tasks = load_tasks_file(&tasks_file_path)?;

  let task_command = tasks.get(alias).ok_or_else(|| {
    let task_names: Vec<String> = tasks.keys().cloned().collect();
    anyhow!("Task '{}' not found. Available tasks: {}", alias, task_names.join(", "))
  })?;

  let stream_output = !options.silent;
  let preserve_colors = stream_output && !is_ci_environment();

  execute_command(task_command, args, stream_output, preserve_colors).await
}

pub async fn list_tasks(tasks_file_path: Option<String>) -> Result<Vec<String>> {
  let tasks_file_path = tasks_file_path.unwrap_or_else(|| "./.cursor/kernelle.tasks".to_string());
  let tasks = load_tasks_file(&tasks_file_path)?;
  Ok(tasks.keys().cloned().collect())
}

pub async fn get_tasks_file(tasks_file_path: Option<String>) -> Result<TasksFile> {
  let tasks_file_path = tasks_file_path.unwrap_or_else(|| "./.cursor/kernelle.tasks".to_string());
  load_tasks_file(&tasks_file_path)
}

fn load_tasks_file(path: &str) -> Result<TasksFile> {
  if !Path::new(path).exists() {
    return Err(anyhow!("Tasks file not found: {}", path));
  }

  let content =
    fs::read_to_string(path).map_err(|e| anyhow!("Failed to read tasks file '{}': {}", path, e))?;

  json5::from_str(&content).map_err(|e| anyhow!("Failed to parse tasks file '{}': {}", path, e))
}

async fn execute_command(
  command: &str,
  args: &[String],
  stream_output: bool,
  preserve_colors: bool,
) -> Result<TaskResult> {
  let full_command =
    if args.is_empty() { command.to_string() } else { format!("{} {}", command, args.join(" ")) };

  let mut cmd = if cfg!(target_os = "windows") {
    let mut c = Command::new("cmd");
    c.args(["/C", &full_command]);
    c
  } else {
    let mut c = Command::new("sh");
    c.args(["-c", &full_command]);
    c
  };

  // Set up environment for color support
  if preserve_colors {
    cmd.env("FORCE_COLOR", "1");
    if env::var("TERM").is_err() {
      cmd.env("TERM", "xterm-256color");
    }
  }

  if stream_output {
    execute_with_streaming(&mut cmd).await
  } else {
    execute_with_capture(&mut cmd).await
  }
}

async fn execute_with_streaming(cmd: &mut Command) -> Result<TaskResult> {
  cmd.stdout(Stdio::piped()).stderr(Stdio::piped()).stdin(Stdio::null());

  let mut child = cmd.spawn()?;

  let stdout = child.stdout.take().unwrap();
  let stderr = child.stderr.take().unwrap();

  let stdout_reader = BufReader::new(stdout);
  let stderr_reader = BufReader::new(stderr);

  let stdout_handle = tokio::spawn(async move {
    let mut lines = stdout_reader.lines();
    while let Ok(Some(line)) = lines.next_line().await {
      println!("{}", line);
    }
  });

  let stderr_handle = tokio::spawn(async move {
    let mut lines = stderr_reader.lines();
    while let Ok(Some(line)) = lines.next_line().await {
      eprintln!("{}", line);
    }
  });

  let status = child.wait().await?;

  // Wait for output streaming to complete
  let _ = tokio::join!(stdout_handle, stderr_handle);

  Ok(TaskResult {
    stdout: "[streamed to console]".to_string(),
    stderr: "[streamed to console]".to_string(),
    success: status.success(),
    exit_code: status.code(),
  })
}

async fn execute_with_capture(cmd: &mut Command) -> Result<TaskResult> {
  cmd.stdout(Stdio::piped()).stderr(Stdio::piped()).stdin(Stdio::null());

  let output = cmd.output().await?;

  Ok(TaskResult {
    stdout: String::from_utf8_lossy(&output.stdout).to_string(),
    stderr: String::from_utf8_lossy(&output.stderr).to_string(),
    success: output.status.success(),
    exit_code: output.status.code(),
  })
}

fn is_ci_environment() -> bool {
  env::var("CI").is_ok() || env::var("NO_COLOR").is_ok()
}
