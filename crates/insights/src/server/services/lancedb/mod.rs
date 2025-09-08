//! LanceDB service for vector similarity search
//!
//! This module provides integration with LanceDB for storing and searching
//! insight embeddings using vector similarity.

pub mod connection;
pub mod models;
pub mod records;
pub mod search;
pub mod table_manager;
pub mod vector_database;

use anyhow::{anyhow, Result};
use chrono::Utc;
use std::path::PathBuf;

use crate::server::models::insight;
use connection::create_connection;
use search::search_similar_embeddings;
use table_manager::TableManager;

// Re-export commonly used types for external use
pub use models::{EmbeddingSearchResult, InsightRecord};
pub use vector_database::LanceDbVectorDatabase;

/// LanceDB service for vector operations
pub struct LanceDbService {
  table_manager: TableManager,
}

impl LanceDbService {
  /// Create a new LanceDB service
  pub async fn new(data_dir: PathBuf, table_name: &str) -> Result<Self> {
    let connection = create_connection(data_dir).await?;
    let table_manager = TableManager::new(connection, table_name.to_string());

    Ok(Self { table_manager })
  }

  /// Store an insight's embedding in LanceDB
  pub async fn store_embedding(&self, insight: &insight::Insight) -> Result<()> {
    let embedding = validate_insight_has_embedding(insight)?;
    let record = create_insight_record(insight, embedding);
    store_record_appropriately(&self.table_manager, &record).await
  }

  /// Check if any embeddings exist in the database
  pub async fn has_embeddings(&self) -> Result<bool> {
    self.table_manager.has_embeddings().await
  }

  /// Search for similar embeddings
  pub async fn search_similar(
    &self,
    query_embedding: &[f32],
    limit: usize,
    threshold: Option<f32>,
  ) -> Result<Vec<models::EmbeddingSearchResult>> {
    let table = self.table_manager.get_table().await?;
    search_similar_embeddings(&table, query_embedding, limit, threshold).await
  }

  /// Delete an insight's embedding
  pub async fn delete_embedding(&self, topic: &str, name: &str) -> Result<()> {
    self.table_manager.delete_embedding(topic, name).await
  }

  /// Update an insight's embedding
  pub async fn update_embedding(&self, insight: &insight::Insight) -> Result<()> {
    // For simplicity, delete and re-insert
    self.delete_embedding(&insight.topic, &insight.name).await?;
    self.store_embedding(insight).await?;
    Ok(())
  }

  /// Get all stored embeddings (for debugging)
  pub async fn get_all_embeddings(&self) -> Result<Vec<models::EmbeddingSearchResult>> {
    // TODO: Implement get all with current LanceDB API
    bentley::info!("Get all embeddings not implemented yet - returning empty results");
    Ok(Vec::new())
  }

  /// Clear all embeddings from the table
  pub async fn clear_all_embeddings(&self) -> Result<()> {
    execute_table_clear(&self.table_manager).await
  }

  /// Reshape the database with fresh schema (clean slate approach)
  pub async fn reshape_database(&self, embedding_dimension: usize) -> Result<()> {
    recreate_database_directory(&self.table_manager, embedding_dimension).await
  }
}

/// Validate that insight has an embedding
fn validate_insight_has_embedding(insight: &insight::Insight) -> Result<&[f32]> {
  insight.embedding.as_deref().ok_or_else(|| anyhow!("Insight has no embedding to store"))
}

/// Create an InsightRecord from an insight and embedding
fn create_insight_record(insight: &insight::Insight, embedding: &[f32]) -> models::InsightRecord {
  let created_timestamp = extract_created_timestamp(insight);
  let updated_timestamp = Utc::now().to_rfc3339();

  models::InsightRecord::new(
    insight.topic.clone(),
    insight.name.clone(),
    insight.overview.clone(),
    insight.details.clone(),
    embedding.to_vec(),
    created_timestamp,
    updated_timestamp,
  )
}

/// Store a record in the appropriate table (create new or add to existing)
async fn store_record_appropriately(
  table_manager: &TableManager,
  record: &models::InsightRecord,
) -> Result<()> {
  if table_manager.table_exists().await? {
    table_manager.add_record_to_existing_table(record).await
  } else {
    table_manager.create_table_with_first_record(record).await
  }
}

/// Execute table clear operation
async fn execute_table_clear(table_manager: &TableManager) -> Result<()> {
  if table_manager.table_exists().await? {
    let table = table_manager.get_table().await?;
    table.delete("id IS NOT NULL").await?;
    bentley::info!("Cleared all embeddings from LanceDB table");
  }
  Ok(())
}

/// Extract created timestamp from insight with fallback to current time
fn extract_created_timestamp(insight: &insight::Insight) -> String {
  insight.embedding_computed.map(|t| t.to_rfc3339()).unwrap_or_else(|| Utc::now().to_rfc3339())
}

/// Completely recreate the database directory for clean slate approach
async fn recreate_database_directory(
  table_manager: &TableManager,
  embedding_dimension: usize,
) -> Result<()> {
  // Delete the entire database directory to ensure clean schema
  let connection = &table_manager.connection;
  let db_path = get_database_path_from_connection(connection).await?;

  bentley::info!(&format!(
    "Deleting database directory for clean slate recreation: {}",
    db_path.display()
  ));
  if db_path.exists() {
    std::fs::remove_dir_all(&db_path)?;
    bentley::info!("Database directory deleted successfully");
  }

  // Update the global schema dimension for future table creation
  update_schema_dimension(embedding_dimension);

  bentley::info!(&format!(
    "Database will be recreated with {embedding_dimension} dimensions on next table creation"
  ));
  Ok(())
}

/// Extract database path from LanceDB connection
async fn get_database_path_from_connection(
  connection: &lancedb::Connection,
) -> Result<std::path::PathBuf> {
  // Use the connection's data directory path
  // LanceDB connections store the data directory internally
  let uri = connection.uri().to_string();
  Ok(std::path::PathBuf::from(uri))
}

/// Update the schema dimension for dynamic table creation
fn update_schema_dimension(dimension: usize) {
  use std::sync::atomic::{AtomicUsize, Ordering};
  static SCHEMA_DIMENSION: AtomicUsize = AtomicUsize::new(768); // Default to 768
  SCHEMA_DIMENSION.store(dimension, Ordering::Relaxed);
}

/// Get the current schema dimension for table creation
pub fn get_schema_dimension() -> usize {
  use std::sync::atomic::{AtomicUsize, Ordering};
  static SCHEMA_DIMENSION: AtomicUsize = AtomicUsize::new(768); // Default to 768
  SCHEMA_DIMENSION.load(Ordering::Relaxed)
}
