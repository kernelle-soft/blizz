//! Server management for automatic server startup and lifecycle
//!
//! This module handles automatically starting the insights server when needed
//! and managing the server lifecycle for the CLI.

use anyhow::{anyhow, Result};
use std::process::{Child, Command, Stdio};
use std::time::Duration;
use tokio::time::sleep;

use crate::cli::client::{get_client, InsightsClient};

// Server startup configuration
const SERVER_STARTUP_TIMEOUT_SECS: u64 = 30; // 30 seconds total timeout
const SERVER_CHECK_INTERVAL_MS: u64 = 500; // Check every 500ms

/// Manages the local insights server lifecycle
pub struct ServerManager {
  client: InsightsClient,
}

impl Default for ServerManager {
  fn default() -> Self {
    Self::new()
  }
}

impl ServerManager {
  /// Create a new server manager
  pub fn new() -> Self {
    Self { client: get_client() }
  }

  /// Ensure the server is running, starting it if necessary
  #[cfg(not(tarpaulin_include))] // Skip coverage - process management and filesystem operations
  pub async fn ensure_server_running(&self) -> Result<()> {
    // First check if server is already running
    if self.client.health_check().await.is_ok() {
      return Ok(());
    }

    // Server not running, let's start it
    bentley::info!("Starting local insights server...");
    self.start_server().await?;

    // Wait for server to be ready
    self.wait_for_server().await?;

    bentley::info!("Insights server is ready");
    Ok(())
  }

  /// Start the server in the background
  #[cfg(not(tarpaulin_include))] // Skip coverage - process spawning
  async fn start_server(&self) -> Result<Child> {
    // Try to find the insights_server binary
    let server_binary = self.find_server_binary()?;

    let mut cmd = Command::new(server_binary);
    cmd
      .args(["--bind", "127.0.0.1:3000"])
      .stdout(Stdio::null())
      .stderr(Stdio::null())
      .stdin(Stdio::null())
      .envs(std::env::vars()); // Pass through all environment variables (including INSIGHTS_ROOT)

    let child = cmd.spawn().map_err(|e| anyhow!("Failed to start insights server: {}", e))?;

    Ok(child)
  }

  /// Wait for the server to become ready
  #[cfg(not(tarpaulin_include))] // Skip coverage - network calls and timing
  async fn wait_for_server(&self) -> Result<()> {
    let max_attempts = (SERVER_STARTUP_TIMEOUT_SECS * 1000) / SERVER_CHECK_INTERVAL_MS;
    let mut attempts = 0;

    while attempts < max_attempts {
      if self.client.health_check().await.is_ok() {
        return Ok(());
      }

      sleep(Duration::from_millis(SERVER_CHECK_INTERVAL_MS)).await;
      attempts += 1;
    }

    let timeout_seconds = SERVER_STARTUP_TIMEOUT_SECS;
    Err(anyhow!("Server failed to start within {} seconds", timeout_seconds))
  }

  /// Find the insights_server binary
  #[cfg(not(tarpaulin_include))] // Skip coverage - filesystem operations
  fn find_server_binary(&self) -> Result<String> {
    // First check if insights_server is available in PATH
    if let Ok(output) = Command::new("which")
      .arg("insights_server")
      .output() 
    {
      if output.status.success() {
        let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !path.is_empty() {
          return Ok(path);
        }
      }
    }

    // Check local build locations as fallback
    let local_paths_to_try = [
      "target/release/insights_server", // Local release build (preferred)
      "target/debug/insights_server",   // Local debug build (fallback)
    ];

    for path in &local_paths_to_try {
      if std::fs::metadata(path).is_ok() {
        return Ok(path.to_string());
      }
    }

    Err(anyhow!("insights_server binary not found. Please ensure it's installed or build it locally."))
  }
}

/// Global function to ensure server is running
#[cfg(not(tarpaulin_include))] // Skip coverage - process management and filesystem operations
pub async fn ensure_server_running() -> Result<()> {
  let manager = ServerManager::new();
  manager.ensure_server_running().await
}
