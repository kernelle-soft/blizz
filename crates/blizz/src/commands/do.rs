use anyhow::{anyhow, Result};
use serde::{Deserialize, Deserializer, Serialize};
use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::Path;
use std::process::Stdio;
use tokio::process::Command;

#[derive(Debug, Clone, Serialize)]
pub enum TaskCommand {
  String(String),
  Array(Vec<String>),
}

impl<'de> Deserialize<'de> for TaskCommand {
  fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
  where
    D: Deserializer<'de>,
  {
    use serde::de::Error;

    let value = serde_yaml::Value::deserialize(deserializer)?;

    match value {
      serde_yaml::Value::String(s) => Ok(TaskCommand::String(s)),
      serde_yaml::Value::Sequence(seq) => {
        let strings: Result<Vec<String>, D::Error> = seq
          .into_iter()
          .map(|v| match v {
            serde_yaml::Value::String(s) => Ok(s),
            serde_yaml::Value::Mapping(map) => {
              if map.len() == 1 {
                if let Some((key, value)) = map.into_iter().next() {
                  if let (
                    serde_yaml::Value::String(key_str),
                    serde_yaml::Value::String(value_str),
                  ) = (key, value)
                  {
                    if key_str == "do" {
                      return Ok(format!("blizz do {value_str}"));
                    }
                  }
                }
              }
              Err(D::Error::custom(
                "Invalid mapping in array. Only 'do: task_name' syntax is supported.",
              ))
            }
            _ => {
              Err(D::Error::custom("Array elements must be strings or 'do: task_name' mappings"))
            }
          })
          .collect();
        Ok(TaskCommand::Array(strings?))
      }
      _ => Err(D::Error::custom("Task command must be a string or array of strings")),
    }
  }
}

impl TaskCommand {
  pub fn to_command_string(&self) -> String {
    match self {
      TaskCommand::String(s) => s.clone(),
      TaskCommand::Array(arr) => arr.join(" && "),
    }
  }
}

pub type TasksFile = HashMap<String, TaskCommand>;

#[derive(Debug, Default)]
pub struct TaskRunnerOptions {
  pub silent: bool,
  pub tasks_file_path: Option<String>,
  pub force_color: bool,
  pub no_color: bool,
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

  let command_string = task_command.to_command_string();
  let stream_output = !options.silent;
  let preserve_colors = if options.no_color {
    false
  } else if options.force_color {
    stream_output
  } else {
    stream_output && !is_ci_environment()
  };

  execute_command(&command_string, args, stream_output, preserve_colors).await
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

  // Parse as generic YAML value first
  let yaml_value: serde_yaml::Value = serde_yaml::from_str(&content)
    .map_err(|e| anyhow!("Failed to parse YAML in tasks file '{}': {}", path, e))?;

  // Convert to our TasksFile format
  let mapping = yaml_value
    .as_mapping()
    .ok_or_else(|| anyhow!("Tasks file '{}' must contain a YAML mapping at the root", path))?;

  let mut tasks = HashMap::new();

  for (key, value) in mapping {
    let key_str =
      key.as_str().ok_or_else(|| anyhow!("Task names must be strings in file '{}'", path))?;

    let task_command = match value {
      serde_yaml::Value::String(s) => TaskCommand::String(s.clone()),
      serde_yaml::Value::Sequence(seq) => {
        let strings: Result<Vec<String>, _> = seq
          .iter()
          .map(|v| {
            match v {
              // Handle string elements
              serde_yaml::Value::String(s) => Ok(s.clone()),
              // Handle "do: task_name" syntax
              serde_yaml::Value::Mapping(map) => {
                if map.len() == 1 {
                  if let Some((key, value)) = map.iter().next() {
                    if let (Some(key_str), Some(value_str)) = (key.as_str(), value.as_str()) {
                      if key_str == "do" {
                        return Ok(format!("blizz do {value_str}"));
                      }
                    }
                  }
                }
                Err(anyhow!("Invalid mapping in array for task '{}' in file '{}'. Only 'do: task_name' syntax is supported.", key_str, path))
              }
              _ => Err(anyhow!("Array elements must be strings or 'do: task_name' mappings for task '{}' in file '{}'", key_str, path))
            }
          })
          .collect();
        TaskCommand::Array(strings?)
      }
      _ => {
        return Err(anyhow!(
          "Task '{}' in file '{}' must be a string or array of strings",
          key_str,
          path
        ))
      }
    };

    tasks.insert(key_str.to_string(), task_command);
  }

  Ok(tasks)
}

fn load_merged_tasks_file() -> Result<TasksFile> {
  let cursor_path = "./.cursor/blizz.yaml";
  let root_path = "./blizz.yaml";

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

  // Use tokio::io::copy to preserve ANSI color codes and other control characters
  let stdout_handle = tokio::spawn(async move {
    let mut stdout_reader = stdout;
    let mut stdout_writer = tokio::io::stdout();
    let _ = tokio::io::copy(&mut stdout_reader, &mut stdout_writer).await;
  });

  let stderr_handle = tokio::spawn(async move {
    let mut stderr_reader = stderr;
    let mut stderr_writer = tokio::io::stderr();
    let _ = tokio::io::copy(&mut stderr_reader, &mut stderr_writer).await;
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
    assert!(!options.force_color);
    assert!(!options.no_color);
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
    let options = TaskRunnerOptions {
      silent: true,
      tasks_file_path: Some("nonexistent.tasks".to_string()),
      force_color: false,
      no_color: false,
    };

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

  #[test]
  fn test_task_command_string_to_command_string() {
    // Test TaskCommand::String converts correctly
    let string_task = TaskCommand::String("echo hello".to_string());
    assert_eq!(string_task.to_command_string(), "echo hello");
  }

  #[test]
  fn test_task_command_array_to_command_string() {
    // Test TaskCommand::Array converts correctly with && joining
    let array_task = TaskCommand::Array(vec![
      "echo first".to_string(),
      "echo second".to_string(),
      "echo third".to_string(),
    ]);
    assert_eq!(array_task.to_command_string(), "echo first && echo second && echo third");
  }

  #[test]
  fn test_task_command_empty_array() {
    // Test TaskCommand::Array with empty array
    let empty_array_task = TaskCommand::Array(vec![]);
    assert_eq!(empty_array_task.to_command_string(), "");
  }

  #[test]
  fn test_task_command_single_item_array() {
    // Test TaskCommand::Array with single item should not add &&
    let single_array_task = TaskCommand::Array(vec!["echo single".to_string()]);
    assert_eq!(single_array_task.to_command_string(), "echo single");
  }

  #[test]
  fn test_load_tasks_file_with_string_commands() {
    // Test loading a YAML file with string commands
    use std::fs;
    use tempfile::NamedTempFile;

    let yaml_content = r#"
build: "cargo build"
test: "cargo test"
clean: "cargo clean"
"#;

    let temp_file = NamedTempFile::new().unwrap();
    fs::write(temp_file.path(), yaml_content).unwrap();

    let result = load_tasks_file(temp_file.path().to_str().unwrap());
    assert!(result.is_ok());

    let tasks = result.unwrap();
    assert_eq!(tasks.len(), 3);

    // Check that string commands are parsed correctly
    if let Some(TaskCommand::String(cmd)) = tasks.get("build") {
      assert_eq!(cmd, "cargo build");
    } else {
      panic!("Expected TaskCommand::String for 'build' task");
    }

    if let Some(TaskCommand::String(cmd)) = tasks.get("test") {
      assert_eq!(cmd, "cargo test");
    } else {
      panic!("Expected TaskCommand::String for 'test' task");
    }
  }

  #[test]
  fn test_load_tasks_file_with_array_commands() {
    // Test loading a YAML file with array commands
    use std::fs;
    use tempfile::NamedTempFile;

    let yaml_content = r#"
tidy:
  - "cargo fmt"
  - "cargo clippy"
checks:
  - "cargo build"
  - "cargo test"
  - "cargo audit"
"#;

    let temp_file = NamedTempFile::new().unwrap();
    fs::write(temp_file.path(), yaml_content).unwrap();

    let result = load_tasks_file(temp_file.path().to_str().unwrap());
    assert!(result.is_ok());

    let tasks = result.unwrap();
    assert_eq!(tasks.len(), 2);

    // Check that array commands are parsed correctly
    if let Some(TaskCommand::Array(cmds)) = tasks.get("tidy") {
      assert_eq!(cmds.len(), 2);
      assert_eq!(cmds[0], "cargo fmt");
      assert_eq!(cmds[1], "cargo clippy");
    } else {
      panic!("Expected TaskCommand::Array for 'tidy' task");
    }

    if let Some(TaskCommand::Array(cmds)) = tasks.get("checks") {
      assert_eq!(cmds.len(), 3);
      assert_eq!(cmds[0], "cargo build");
      assert_eq!(cmds[1], "cargo test");
      assert_eq!(cmds[2], "cargo audit");
    } else {
      panic!("Expected TaskCommand::Array for 'checks' task");
    }
  }

  #[test]
  fn test_load_tasks_file_with_mixed_commands() {
    // Test loading a YAML file with both string and array commands
    use std::fs;
    use tempfile::NamedTempFile;

    let yaml_content = r#"
build: "cargo build"
tidy:
  - "cargo fmt"
  - "cargo clippy"
test: "cargo test"
full_check:
  - "cargo build"
  - "cargo test"
  - "cargo audit"
"#;

    let temp_file = NamedTempFile::new().unwrap();
    fs::write(temp_file.path(), yaml_content).unwrap();

    let result = load_tasks_file(temp_file.path().to_str().unwrap());
    assert!(result.is_ok());

    let tasks = result.unwrap();
    assert_eq!(tasks.len(), 4);

    // Check string commands
    assert!(matches!(tasks.get("build"), Some(TaskCommand::String(_))));
    assert!(matches!(tasks.get("test"), Some(TaskCommand::String(_))));

    // Check array commands
    assert!(matches!(tasks.get("tidy"), Some(TaskCommand::Array(_))));
    assert!(matches!(tasks.get("full_check"), Some(TaskCommand::Array(_))));

    // Verify command string generation works correctly for both types
    assert_eq!(tasks.get("build").unwrap().to_command_string(), "cargo build");
    assert_eq!(tasks.get("tidy").unwrap().to_command_string(), "cargo fmt && cargo clippy");
    assert_eq!(
      tasks.get("full_check").unwrap().to_command_string(),
      "cargo build && cargo test && cargo audit"
    );
  }

  #[test]
  fn test_load_tasks_file_with_invalid_yaml() {
    // Test error handling for invalid YAML
    use std::fs;
    use tempfile::NamedTempFile;

    let invalid_yaml = r#"
build: "cargo build"
invalid:
  - this
  - is
  - 123
  - valid: but this creates nested structure
"#;

    let temp_file = NamedTempFile::new().unwrap();
    fs::write(temp_file.path(), invalid_yaml).unwrap();

    let result = load_tasks_file(temp_file.path().to_str().unwrap());
    assert!(result.is_err());

    let error_message = result.unwrap_err().to_string();
    assert!(error_message.contains("Array elements must be strings"));
  }

  #[test]
  fn test_load_tasks_file_with_task_names_containing_special_chars() {
    // Test that task names with colons and other special characters work when quoted properly
    use std::fs;
    use tempfile::NamedTempFile;

    let yaml_content = r#"
"task:with:colons": "echo hello"
"task-with-dashes": "echo world"
task_with_underscores: "echo test"
array_task:
  - "echo first"
  - "echo second"
"#;

    let temp_file = NamedTempFile::new().unwrap();
    fs::write(temp_file.path(), yaml_content).unwrap();

    let result = load_tasks_file(temp_file.path().to_str().unwrap());
    assert!(result.is_ok());

    let tasks = result.unwrap();
    assert_eq!(tasks.len(), 4);

    // Check that task names with special characters are preserved
    assert!(tasks.contains_key("task:with:colons"));
    assert!(tasks.contains_key("task-with-dashes"));
    assert!(tasks.contains_key("task_with_underscores"));
    assert!(tasks.contains_key("array_task"));
  }

  #[test]
  fn test_task_command_serialization() {
    // Test that TaskCommand can be serialized and deserialized correctly
    use serde_yaml;

    // Test string variant - it serializes as a tagged enum
    let string_task = TaskCommand::String("echo hello".to_string());
    let yaml_str = serde_yaml::to_string(&string_task).unwrap();
    // The actual serialization includes the variant tag
    assert!(yaml_str.contains("echo hello"));

    // Test array variant
    let array_task = TaskCommand::Array(vec!["echo first".to_string(), "echo second".to_string()]);
    let yaml_str = serde_yaml::to_string(&array_task).unwrap();
    assert!(yaml_str.contains("echo first"));
    assert!(yaml_str.contains("echo second"));

    // Test round-trip deserialization works correctly
    let test_yaml = r#"
build: "cargo build"
tidy:
  - "cargo fmt"
  - "cargo clippy"
"#;

    let tasks: std::collections::HashMap<String, TaskCommand> =
      serde_yaml::from_str(test_yaml).unwrap();
    assert_eq!(tasks.len(), 2);

    // Verify the deserialized commands work correctly
    assert_eq!(tasks.get("build").unwrap().to_command_string(), "cargo build");
    assert_eq!(tasks.get("tidy").unwrap().to_command_string(), "cargo fmt && cargo clippy");
  }

  #[test]
  fn test_do_syntax_in_arrays() {
    // Test the new "do: task_name" syntax
    use std::fs;
    use tempfile::NamedTempFile;

    let yaml_content = r#"
basic_task: "echo hello"
chain_with_do:
  - do: basic_task
  - echo "after basic task"
mixed_commands:
  - "echo first command"
  - do: basic_task
  - "echo third command"
"#;

    let temp_file = NamedTempFile::new().unwrap();
    fs::write(temp_file.path(), yaml_content).unwrap();

    let result = load_tasks_file(temp_file.path().to_str().unwrap());
    assert!(result.is_ok());

    let tasks = result.unwrap();
    assert_eq!(tasks.len(), 3);

    // Check that "do:" syntax gets converted to "blizz do"
    if let Some(TaskCommand::Array(cmds)) = tasks.get("chain_with_do") {
      assert_eq!(cmds.len(), 2);
      assert_eq!(cmds[0], "blizz do basic_task");
      assert_eq!(cmds[1], "echo \"after basic task\"");
    } else {
      panic!("Expected TaskCommand::Array for 'chain_with_do' task");
    }

    // Check mixed commands
    if let Some(TaskCommand::Array(cmds)) = tasks.get("mixed_commands") {
      assert_eq!(cmds.len(), 3);
      assert_eq!(cmds[0], "echo first command");
      assert_eq!(cmds[1], "blizz do basic_task");
      assert_eq!(cmds[2], "echo third command");
    } else {
      panic!("Expected TaskCommand::Array for 'mixed_commands' task");
    }

    // Verify the command string generation works correctly
    assert_eq!(
      tasks.get("chain_with_do").unwrap().to_command_string(),
      "blizz do basic_task && echo \"after basic task\""
    );
    assert_eq!(
      tasks.get("mixed_commands").unwrap().to_command_string(),
      "echo first command && blizz do basic_task && echo third command"
    );
  }

  #[test]
  fn test_invalid_do_syntax() {
    // Test that invalid "do:" mappings are handled correctly
    use std::fs;
    use tempfile::NamedTempFile;

    let yaml_content = r#"
invalid_do_mapping:
  - do: valid_task
  - do:
      nested: invalid
"#;

    let temp_file = NamedTempFile::new().unwrap();
    fs::write(temp_file.path(), yaml_content).unwrap();

    let result = load_tasks_file(temp_file.path().to_str().unwrap());
    assert!(result.is_err());

    let error_message = result.unwrap_err().to_string();
    assert!(error_message.contains("Invalid mapping in array"));
  }

  #[test]
  fn test_do_syntax_with_multiple_keys() {
    // Test that mappings with multiple keys are rejected
    use std::fs;
    use tempfile::NamedTempFile;

    let yaml_content = r#"
invalid_task:
  - do: valid_task
    other_key: invalid
"#;

    let temp_file = NamedTempFile::new().unwrap();
    fs::write(temp_file.path(), yaml_content).unwrap();

    let result = load_tasks_file(temp_file.path().to_str().unwrap());
    assert!(result.is_err());

    let error_message = result.unwrap_err().to_string();
    assert!(error_message.contains("Invalid mapping in array"));
  }
}
