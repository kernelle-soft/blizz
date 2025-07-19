use anyhow::{anyhow, Result};
use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::Path;
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;

pub type TasksFile = HashMap<String, String>;

#[derive(Debug, Default)]
pub struct TaskRunnerOptions {
  pub silent: bool,
  pub tasks_file_path: Option<String>,
}

#[derive(Debug)]
pub struct TaskResult {
  pub success: bool,
  pub exit_code: Option<i32>,
}

pub async fn run_task(
  alias: &str,
  args: &[String],
  options: TaskRunnerOptions,
) -> Result<TaskResult> {
  let tasks = match options.tasks_file_path {
    Some(path) => load_tasks_file(&path)?,
    None => load_merged_tasks_file()?,
  };

  let task_command = tasks.get(alias).ok_or_else(|| {
    let task_names: Vec<String> = tasks.keys().cloned().collect();
    anyhow!("Task '{}' not found. Available tasks: {}", alias, task_names.join(", "))
  })?;

  let stream_output = !options.silent;
  let preserve_colors = stream_output && !is_ci_environment();

  execute_command(task_command, args, stream_output, preserve_colors).await
}

pub async fn list_tasks(tasks_file_path: Option<String>) -> Result<Vec<String>> {
  let tasks = match tasks_file_path {
    Some(path) => load_tasks_file(&path)?,
    None => load_merged_tasks_file()?,
  };
  Ok(tasks.keys().cloned().collect())
}

pub async fn get_tasks_file(tasks_file_path: Option<String>) -> Result<TasksFile> {
  match tasks_file_path {
    Some(path) => load_tasks_file(&path),
    None => load_merged_tasks_file(),
  }
}

fn load_tasks_file(path: &str) -> Result<TasksFile> {
  if !Path::new(path).exists() {
    return Err(anyhow!("Tasks file not found: {}", path));
  }

  let content =
    fs::read_to_string(path).map_err(|e| anyhow!("Failed to read tasks file '{}': {}", path, e))?;

  json5::from_str(&content).map_err(|e| anyhow!("Failed to parse tasks file '{}': {}", path, e))
}

fn load_merged_tasks_file() -> Result<TasksFile> {
  let cursor_path = "./.cursor/kernelle.tasks";
  let root_path = "./kernelle.tasks";

  let cursor_exists = Path::new(cursor_path).exists();
  let root_exists = Path::new(root_path).exists();

  match (cursor_exists, root_exists) {
    (true, true) => {
      // Both exist - merge them with cursor taking precedence
      let mut root_tasks = load_tasks_file(root_path)?;
      let cursor_tasks = load_tasks_file(cursor_path)?;

      // Start with root tasks, then overlay cursor tasks (cursor wins conflicts)
      root_tasks.extend(cursor_tasks);
      Ok(root_tasks)
    }
    (true, false) => {
      // Only cursor exists
      load_tasks_file(cursor_path)
    }
    (false, true) => {
      // Only root exists
      load_tasks_file(root_path)
    }
    (false, false) => {
      Err(anyhow!("No tasks file found. Looked for:\n  - {}\n  - {}", cursor_path, root_path))
    }
  }
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
      println!("{line}");
    }
  });

  let stderr_handle = tokio::spawn(async move {
    let mut lines = stderr_reader.lines();
    while let Ok(Some(line)) = lines.next_line().await {
      eprintln!("{line}");
    }
  });

  let status = child.wait().await?;

  // Wait for output streaming to complete
  let _ = tokio::join!(stdout_handle, stderr_handle);

  Ok(TaskResult { success: status.success(), exit_code: status.code() })
}

async fn execute_with_capture(cmd: &mut Command) -> Result<TaskResult> {
  cmd.stdout(Stdio::piped()).stderr(Stdio::piped()).stdin(Stdio::null());

  let output = cmd.output().await?;

  Ok(TaskResult { success: output.status.success(), exit_code: output.status.code() })
}

fn is_ci_environment() -> bool {
  env::var("CI").is_ok() || env::var("NO_COLOR").is_ok()
}

#[cfg(test)]
mod tests {
  use super::*;

  #[tokio::test]
  async fn test_list_tasks_with_nonexistent_file() {
    // Test list_tasks with a file that doesn't exist
    let result = list_tasks(Some("nonexistent.tasks".to_string())).await;

    // Should return an error since the file doesn't exist
    assert!(result.is_err());
  }

  #[tokio::test]
  async fn test_get_tasks_file_with_nonexistent_file() {
    // Test get_tasks_file with a file that doesn't exist
    let result = get_tasks_file(Some("nonexistent.tasks".to_string())).await;

    // Should return an error since the file doesn't exist
    assert!(result.is_err());
  }

  #[test]
  fn test_task_runner_options_default() {
    // Test that TaskRunnerOptions has sensible defaults
    let options = TaskRunnerOptions::default();

    assert!(!options.silent);
    assert!(options.tasks_file_path.is_none());
  }

  #[test]
  fn test_task_result_fields() {
    // Test TaskResult struct fields
    let result = TaskResult { success: true, exit_code: Some(0) };

    assert!(result.success);
    assert_eq!(result.exit_code, Some(0));

    let failed_result = TaskResult { success: false, exit_code: Some(1) };

    assert!(!failed_result.success);
    assert_eq!(failed_result.exit_code, Some(1));
  }

  #[tokio::test]
  async fn test_run_task_with_nonexistent_task() {
    // Test running a task that doesn't exist
    let options =
      TaskRunnerOptions { silent: true, tasks_file_path: Some("nonexistent.tasks".to_string()) };

    let result = run_task("nonexistent_task", &[], options).await;

    // Should return an error since the file doesn't exist
    assert!(result.is_err());
  }

  #[test]
  fn test_is_ci_environment() {
    // Test the CI environment detection function
    // This will depend on the actual environment, so we just ensure it doesn't panic
    let _is_ci = is_ci_environment();
    // Function should not panic and should return a boolean
  }
}
