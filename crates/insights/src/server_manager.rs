//! Server management for automatic server startup and lifecycle
//!
//! This module handles automatically starting the insights server when needed
//! and managing the server lifecycle for the CLI.

use anyhow::{anyhow, Result};
use std::process::{Child, Command, Stdio};
use std::time::Duration;
use tokio::time::sleep;

use crate::client::{get_client, InsightsClient};

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
        Self {
            client: get_client(),
        }
    }
    
    /// Ensure the server is running, starting it if necessary
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
    async fn start_server(&self) -> Result<Child> {
        // Try to find the insights_server binary
        let server_binary = self.find_server_binary()?;
        
        let mut cmd = Command::new(server_binary);
        cmd.args(["--bind", "127.0.0.1:3000"])
           .stdout(Stdio::null())
           .stderr(Stdio::null())
           .stdin(Stdio::null());
        
        let child = cmd.spawn()
            .map_err(|e| anyhow!("Failed to start insights server: {}", e))?;
            
        Ok(child)
    }
    
    /// Wait for the server to become ready
    async fn wait_for_server(&self) -> Result<()> {
        let max_attempts = 30; // 15 seconds total
        let mut attempts = 0;
        
        while attempts < max_attempts {
            if self.client.health_check().await.is_ok() {
                return Ok(());
            }
            
            sleep(Duration::from_millis(500)).await;
            attempts += 1;
        }
        
        Err(anyhow!("Server failed to start within 15 seconds"))
    }
    
    /// Find the insights_server binary
    fn find_server_binary(&self) -> Result<String> {
        // Try different possible locations for the binary
        let possible_paths = [
            "insights_server",                                    // In PATH
            "./target/debug/insights_server",                    // Local debug build
            "./target/release/insights_server",                  // Local release build
            "../target/debug/insights_server",                   // From CLI working dir
            "../target/release/insights_server",                 // From CLI working dir  
            "target/debug/insights_server",                      // Relative
            "target/release/insights_server",                    // Relative
        ];
        
        for path in &possible_paths {
            if std::fs::metadata(path).is_ok() {
                return Ok(path.to_string());
            }
        }
        
        // If nothing found, try to compile it
        bentley::info!("insights_server binary not found, attempting to build...");
        self.build_server()?;
        
        // Try again after build
        if std::fs::metadata("target/debug/insights_server").is_ok() {
            return Ok("target/debug/insights_server".to_string());
        }
        
        Err(anyhow!(
            "Could not find or build insights_server binary. Please run 'cargo build --bin insights_server'"
        ))
    }
    
    /// Build the server binary
    fn build_server(&self) -> Result<()> {
        let output = Command::new("cargo")
            .args(["build", "--bin", "insights_server"])
            .output()
            .map_err(|e| anyhow!("Failed to run cargo build: {}", e))?;
        
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!("Failed to build insights_server: {}", stderr));
        }
        
        Ok(())
    }
}

/// Global function to ensure server is running
pub async fn ensure_server_running() -> Result<()> {
    let manager = ServerManager::new();
    manager.ensure_server_running().await
}
