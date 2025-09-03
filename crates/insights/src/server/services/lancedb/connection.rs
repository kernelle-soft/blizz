//! Database connection management for LanceDB

use anyhow::{anyhow, Result};
use lancedb::{connect, Connection};
use std::path::PathBuf;

/// Create a LanceDB connection, creating the data directory if needed
pub async fn create_connection(data_dir: PathBuf) -> Result<Connection> {
    ensure_data_directory_exists(&data_dir)?;
    
    connect(&data_dir.to_string_lossy())
        .execute()
        .await
        .map_err(|e| anyhow!("Failed to connect to LanceDB: {}", e))
}

/// Create data directory if it doesn't exist
fn ensure_data_directory_exists(data_dir: &PathBuf) -> Result<()> {
    if !data_dir.exists() {
        std::fs::create_dir_all(data_dir)
            .map_err(|e| anyhow!("Failed to create data directory: {}", e))?;
    }
    Ok(())
}
