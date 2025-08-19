use anyhow::{anyhow, Result};
use std::env;
use std::path::Path;
use tokio::net::UnixStream;
use tokio::time::{sleep, Duration};

/// Start the agent
pub async fn start(
  socket_path: &std::path::Path,
  pid_file: &std::path::Path,
  keeper_path: &std::path::Path,
) -> Result<()> {
  use std::{fs, process::Command};

  // Check if already running
  if socket_path.exists() {
    bentley::warn("agent appears to already be running");
    bentley::info("use 'secrets agent status' to check or 'secrets agent restart' to restart");
    return Ok(());
  }

  bentley::info("starting agent...");

  // Spawn keeper binary as background process
  let mut cmd = Command::new("keeper");

  // Inherit current environment and add our specific variables
  cmd.envs(env::vars());

  // Forward environment variables that the keeper needs explicitly
  if let Ok(kernelle_home) = env::var("KERNELLE_HOME") {
    cmd.env("KERNELLE_HOME", kernelle_home);
  }
  if let Ok(secrets_auth) = env::var("SECRETS_AUTH") {
    cmd.env("SECRETS_AUTH", secrets_auth);
  }

  let output = cmd.spawn();

  match output {
    Ok(mut child) => {
      fs::create_dir_all(keeper_path)?;
      fs::write(pid_file, child.id().to_string())?;

      // Wait for socket to be created (indicates successful startup)
      // We'll wait indefinitely since password entry can take time
      loop {
        // Check if process exited unexpectedly
        if let Ok(Some(status)) = child.try_wait() {
          let _ = fs::remove_file(pid_file);
          if status.success() {
            bentley::error("keeper process exited unexpectedly");
          } else {
            bentley::error("keeper process failed to start");
          }
          return Ok(());
        }

        // Check if socket exists
        if socket_path.exists() {
          bentley::success("agent started successfully");
          return Ok(());
        }

        // Short sleep to avoid busy waiting
        sleep(Duration::from_millis(100)).await;
      }
    }
    Err(e) => {
      bentley::error(&format!("failed to start agent: {e}"));
      bentley::info("make sure the 'keeper' binary is in your PATH");
    }
  }

  Ok(())
}

/// Check the status of the agent
pub async fn status(socket_path: &std::path::Path) -> Result<()> {
  if !socket_path.exists() {
    bentley::info("agent is not running");
    bentley::info("use 'secrets agent start' to start the daemon");
    return Ok(());
  }

  match UnixStream::connect(&socket_path).await {
    Ok(mut stream) => {
      use tokio::io::{AsyncReadExt, AsyncWriteExt};
      if (stream.write_all(b"GET\n").await).is_err() {
        bentley::warn("socket exists but failed to communicate");
        return Ok(());
      }

      let mut response = String::new();
      if stream.read_to_string(&mut response).await.is_ok() && !response.trim().is_empty() {
        bentley::success("keeper is running and responsive");
      } else {
        bentley::error("keeper is running but not responding correctly");
      }
    }
    Err(_) => {
      bentley::error("socket file exists but connection failed");
      bentley::error("agent may be starting up or in bad state");
    }
  }

  Ok(())
}

/// Stop the agent
pub async fn stop(socket_path: &std::path::Path, pid_file: &std::path::Path) -> Result<()> {
  use std::{fs, process::Command};

  if !socket_path.exists() {
    bentley::info("agent is not running");
    return Ok(());
  }

  bentley::info("stopping agent...");

  if !pid_file.exists() {
    bentley::warn("PID file not found, cleaning up socket");
    let _ = fs::remove_file(socket_path);
    return Ok(());
  }

  let pid_str = fs::read_to_string(pid_file).ok();

  if !pid_file.exists() || pid_str.is_none() {
    bentley::warn("PID file not found or unreadable, cleaning up socket");
    let _ = fs::remove_file(socket_path);
    return Ok(());
  }

  let pid: u32 = pid_str.unwrap().trim().parse().unwrap_or(0);
  if pid == 0 {
    bentley::warn("invalid PID, cleaning up socket");
    let _ = fs::remove_file(socket_path);
    return Ok(());
  }

  let output = Command::new("kill").arg(pid.to_string()).output();
  match output {
    Ok(result) if result.status.success() => {
      // Wait a moment for graceful shutdown
      sleep(Duration::from_millis(500)).await;

      // Clean up files
      let _ = fs::remove_file(socket_path);
      let _ = fs::remove_file(pid_file);

      bentley::success("agent stopped");
    }
    _ => {
      bentley::warn("failed to stop agent gracefully, cleaning up files");
      let _ = fs::remove_file(socket_path);
      let _ = fs::remove_file(pid_file);
    }
  }

  Ok(())
}

/// Restart the agent
pub async fn restart(
  socket_path: &std::path::Path,
  pid_file: &std::path::Path,
  keeper_path: &std::path::Path,
) -> Result<()> {
  if socket_path.exists() {
    stop(socket_path, pid_file).await?;
    sleep(Duration::from_millis(1000)).await;
  }

  start(socket_path, pid_file, keeper_path).await?;

  Ok(())
}

/// Try to get password from running daemon
pub async fn get(base_path: &Path) -> Result<String> {
  let socket_path = base_path.join("persistent").join("keeper").join("keeper.sock");

  if !socket_path.exists() {
    return Err(anyhow!("daemon socket not found"));
  }

  let mut stream = UnixStream::connect(&socket_path)
    .await
    .map_err(|e| anyhow!("failed to connect to daemon: {}", e))?;

  use tokio::io::{AsyncReadExt, AsyncWriteExt};

  // Send GET request to daemon (with newline for protocol compatibility)
  stream
    .write_all(b"GET\n")
    .await
    .map_err(|e| anyhow!("failed to send request to daemon: {}", e))?;

  // Read password response
  let mut password = String::new();
  stream
    .read_to_string(&mut password)
    .await
    .map_err(|e| anyhow!("failed to read response from daemon: {}", e))?;

  let password = password.trim();
  if password.is_empty() {
    return Err(anyhow!("daemon returned empty password"));
  }

  Ok(password.to_string())
}
