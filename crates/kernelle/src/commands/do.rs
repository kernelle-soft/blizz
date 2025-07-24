use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::env;
use std::fs;
use std::path::Path;
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum TaskDefinition {
  Simple(String),
  List(Vec<TaskCommand>),
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum TaskCommand {
  Simple(String),
  WithEnv {
    #[serde(flatten)]
    command: TaskCommandType,
    env: HashMap<String, String>,
  },
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum TaskCommandType {
  Shell(String),
  Do { r#do: String },
}

pub type TasksFile = HashMap<String, TaskDefinition>;

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

#[derive(Debug, Clone)]
struct QueuedCommand {
  command: TaskCommand,
  args: Vec<String>,
}

pub async fn run_task(
  alias: &str,
  args: &[String],
  options: TaskRunnerOptions,
) -> Result<TaskResult> {
  let tasks = match &options.tasks_file_path {
    Some(path) => load_tasks_file(path)?,
    None => load_merged_tasks_file()?,
  };

  let task_definition = tasks.get(alias).ok_or_else(|| {
    let task_names: Vec<String> = tasks.keys().cloned().collect();
    anyhow!("Task '{}' not found. Available tasks: {}", alias, task_names.join(", "))
  })?;

  execute_task_with_queue(task_definition, args, &tasks, &options).await
}

async fn execute_task_with_queue(
  definition: &TaskDefinition,
  args: &[String],
  tasks: &TasksFile,
  options: &TaskRunnerOptions,
) -> Result<TaskResult> {
  let mut command_queue: VecDeque<QueuedCommand> = VecDeque::new();
  
  // Clone options to avoid borrow issues
  let silent = options.silent;
  
  // Populate initial queue based on task definition
  match definition {
    TaskDefinition::Simple(command) => {
      command_queue.push_back(QueuedCommand {
        command: TaskCommand::Simple(command.clone()),
        args: args.to_vec(),
      });
    }
    TaskDefinition::List(commands) => {
      for command in commands {
        command_queue.push_back(QueuedCommand {
          command: command.clone(),
          args: args.to_vec(),
        });
      }
    }
  }

  // Process queue iteratively
  while let Some(queued_cmd) = command_queue.pop_front() {
    let result = execute_single_command(&queued_cmd, tasks, silent, &mut command_queue).await?;
    if !result.success {
      return Ok(result);
    }
  }

  Ok(TaskResult { success: true, exit_code: Some(0) })
}

async fn execute_single_command(
  queued_cmd: &QueuedCommand,
  tasks: &TasksFile,
  silent: bool,
  command_queue: &mut VecDeque<QueuedCommand>,
) -> Result<TaskResult> {
  let (command_type, env_vars) = match &queued_cmd.command {
    TaskCommand::Simple(cmd) => (TaskCommandType::Shell(cmd.clone()), HashMap::new()),
    TaskCommand::WithEnv { command, env } => (command.clone(), env.clone()),
  };

  match command_type {
    TaskCommandType::Shell(shell_command) => {
      let stream_output = !silent;
      let preserve_colors = stream_output && !is_ci_environment();
      execute_shell_command(&shell_command, &queued_cmd.args, env_vars, stream_output, preserve_colors).await
    }
    TaskCommandType::Do { r#do } => {
      let nested_task = tasks.get(&r#do).ok_or_else(|| {
        anyhow!("Task '{}' referenced by 'do:' not found", r#do)
      })?;
      
      // Instead of recursing, add the nested task's commands to the front of the queue
      let mut new_commands = VecDeque::new();
      match nested_task {
        TaskDefinition::Simple(command) => {
          new_commands.push_back(QueuedCommand {
            command: TaskCommand::Simple(command.clone()),
            args: vec![], // do: commands don't inherit args
          });
        }
        TaskDefinition::List(commands) => {
          for command in commands {
            new_commands.push_back(QueuedCommand {
              command: command.clone(),
              args: vec![], // do: commands don't inherit args
            });
          }
        }
      }
      
      // Prepend new commands to the queue
      while let Some(cmd) = new_commands.pop_back() {
        command_queue.push_front(cmd);
      }
      
      // Return success for the do: command itself
      Ok(TaskResult { success: true, exit_code: Some(0) })
    }
  }
}

pub async fn list_tasks(tasks_file_path: Option<String>) -> Result<Vec<String>> {
  let tasks = match tasks_file_path {
    Some(path) => load_tasks_file(&path)?,
    None => load_merged_tasks_file()?,
  };
  Ok(tasks.keys().cloned().collect())
}

pub async fn get_tasks_file(tasks_file_path: Option<String>) -> Result<HashMap<String, String>> {
  let tasks = match tasks_file_path {
    Some(path) => load_tasks_file(&path)?,
    None => load_merged_tasks_file()?,
  };
  
  // Convert TasksFile to the legacy format for compatibility with verbose listing
  let mut legacy_format = HashMap::new();
  for (name, definition) in tasks {
    let description = match definition {
      TaskDefinition::Simple(cmd) => cmd,
      TaskDefinition::List(commands) => {
        // Show a summary for list tasks
        let count = commands.len();
        format!("[{} commands]", count)
      }
    };
    legacy_format.insert(name, description);
  }
  
  Ok(legacy_format)
}

fn load_tasks_file(path: &str) -> Result<TasksFile> {
  if !Path::new(path).exists() {
    return Err(anyhow!("Tasks file not found: {}", path));
  }

  let content =
    fs::read_to_string(path).map_err(|e| anyhow!("Failed to read tasks file '{}': {}", path, e))?;

  serde_yaml::from_str(&content)
    .map_err(|e| anyhow!("Failed to parse tasks file '{}': {}", path, e))
}

fn load_merged_tasks_file() -> Result<TasksFile> {
  let cursor_path = ".cursor/kernelle.yaml";
  let root_path = "./kernelle.yaml";

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

async fn execute_shell_command(
  command: &str,
  args: &[String],
  env_vars: HashMap<String, String>,
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
    let mut c = Command::new("bash");
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

  // Apply custom environment variables
  for (key, value) in env_vars {
    cmd.env(key, value);
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
    let result = list_tasks(Some("nonexistent.yaml".to_string())).await;
    assert!(result.is_err());
  }

  #[tokio::test]
  async fn test_get_tasks_file_with_nonexistent_file() {
    let result = get_tasks_file(Some("nonexistent.yaml".to_string())).await;
    assert!(result.is_err());
  }

  #[test]
  fn test_task_runner_options_default() {
    let options = TaskRunnerOptions::default();
    assert!(!options.silent);
    assert!(options.tasks_file_path.is_none());
  }

  #[test]
  fn test_task_result_fields() {
    let result = TaskResult { success: true, exit_code: Some(0) };
    assert!(result.success);
    assert_eq!(result.exit_code, Some(0));

    let failed_result = TaskResult { success: false, exit_code: Some(1) };
    assert!(!failed_result.success);
    assert_eq!(failed_result.exit_code, Some(1));
  }

  #[tokio::test]
  async fn test_run_task_with_nonexistent_task() {
    let options =
      TaskRunnerOptions { silent: true, tasks_file_path: Some("nonexistent.yaml".to_string()) };

    let result = run_task("nonexistent_task", &[], options).await;
    assert!(result.is_err());
  }

  #[test]
  fn test_is_ci_environment() {
    let _is_ci = is_ci_environment();
  }

  #[test]
  fn test_task_definition_deserialization() {
    // Test simple string task
    let yaml = r#""cargo build""#;
    let task: TaskDefinition = serde_yaml::from_str(yaml).unwrap();
    match task {
      TaskDefinition::Simple(cmd) => assert_eq!(cmd, "cargo build"),
      _ => panic!("Expected simple task"),
    }

    // Test list task
    let yaml = r#"
- "cargo clean"
- "cargo build"
"#;
    let task: TaskDefinition = serde_yaml::from_str(yaml).unwrap();
    match task {
      TaskDefinition::List(commands) => assert_eq!(commands.len(), 2),
      _ => panic!("Expected list task"),
    }
  }

  #[test]
  fn test_task_command_with_env() {
    let yaml = r#"
command: "cargo test"
env:
  NO_COLOR: "1"
  RUST_LOG: "debug"
"#;
    
    // Note: This would be part of a larger YAML structure in practice
    // Just testing the serde structure works
    let cmd: TaskCommand = serde_yaml::from_str(yaml).unwrap();
    match cmd {
      TaskCommand::WithEnv { env, .. } => {
        assert_eq!(env.get("NO_COLOR"), Some(&"1".to_string()));
        assert_eq!(env.get("RUST_LOG"), Some(&"debug".to_string()));
      }
      _ => panic!("Expected command with env"),
    }
  }

  #[tokio::test]
  async fn test_queue_based_execution() {
    // Test that simple commands work
    let mut queue = VecDeque::new();
    queue.push_back(QueuedCommand {
      command: TaskCommand::Simple("echo test".to_string()),
      args: vec![],
    });
    
    assert_eq!(queue.len(), 1);
  }
}
