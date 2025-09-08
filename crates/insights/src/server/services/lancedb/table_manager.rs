//! Table management operations for LanceDB

use anyhow::{anyhow, Result};
use arrow::record_batch::RecordBatchIterator;
use lancedb::{Connection, Table};

use super::models::InsightRecord;
use super::records::records_to_arrow_batch;

/// Table manager for LanceDB operations
pub struct TableManager {
  pub connection: Connection,
  table_name: String,
}

impl TableManager {
  pub fn new(connection: Connection, table_name: String) -> Self {
    Self { connection, table_name }
  }

  /// Check if the target table exists
  pub async fn table_exists(&self) -> Result<bool> {
    check_if_table_exists(&self.connection, &self.table_name).await
  }

  /// Get the table instance
  pub async fn get_table(&self) -> Result<Table> {
    open_table_by_name(&self.connection, &self.table_name).await
  }

  /// Create a new table with the first record
  pub async fn create_table_with_first_record(&self, record: &InsightRecord) -> Result<()> {
    let batch_iter = prepare_record_batch_iterator(record)?;

    self
      .connection
      .create_table(&self.table_name, batch_iter)
      .execute()
      .await
      .map_err(|e| anyhow!("Failed to create table with first record: {}", e))?;

    log_table_creation(&self.table_name, record);
    Ok(())
  }

  /// Add a record to an existing table
  pub async fn add_record_to_existing_table(&self, record: &InsightRecord) -> Result<()> {
    let batch_iter = prepare_record_batch_iterator(record)?;
    let table = self.get_table().await?;

    table
      .add(batch_iter)
      .execute()
      .await
      .map_err(|e| anyhow!("Failed to store embedding: {}", e))?;

    log_record_stored(record);
    Ok(())
  }

  /// Check if any embeddings exist in the database
  pub async fn has_embeddings(&self) -> Result<bool> {
    check_embeddings_exist(&self.connection, &self.table_name).await
  }

  /// Delete an insight's embedding
  pub async fn delete_embedding(&self, topic: &str, name: &str) -> Result<()> {
    let table = self.get_table().await?;
    let id = create_insight_id(topic, name);

    table
      .delete(&format!("id = '{id}'"))
      .await
      .map_err(|e| anyhow!("Failed to delete embedding: {}", e))?;

    log_embedding_deleted(topic, name);
    Ok(())
  }
}

/// Prepare RecordBatchIterator from a single InsightRecord
fn prepare_record_batch_iterator(
  record: &InsightRecord,
) -> Result<
  RecordBatchIterator<
    std::vec::IntoIter<Result<arrow::record_batch::RecordBatch, arrow::error::ArrowError>>,
  >,
> {
  let batch = records_to_arrow_batch(vec![record.clone()])?;
  let schema = batch.schema();
  Ok(RecordBatchIterator::new(vec![Ok(batch)].into_iter(), schema))
}

/// Create insight ID from topic and name
fn create_insight_id(topic: &str, name: &str) -> String {
  format!("{topic}:{name}")
}

/// Log table creation
fn log_table_creation(table_name: &str, record: &InsightRecord) {
  bentley::info!(&format!(
    "Created table '{}' with first embedding for {}/{}",
    table_name, record.topic, record.name
  ));
}

/// Log record stored
fn log_record_stored(record: &InsightRecord) {
  bentley::info!(&format!("Stored embedding for {}/{}", record.topic, record.name));
}

/// Log embedding deleted
fn log_embedding_deleted(topic: &str, name: &str) {
  bentley::info!(&format!("Deleted embedding for {topic}/{name}"));
}

/// Check if table exists in the connection
async fn check_if_table_exists(connection: &Connection, table_name: &str) -> Result<bool> {
  let tables = connection
    .table_names()
    .execute()
    .await
    .map_err(|e| anyhow!("Failed to list tables: {}", e))?;
  Ok(tables.contains(&table_name.to_string()))
}

/// Open table by name from connection
async fn open_table_by_name(connection: &Connection, table_name: &str) -> Result<Table> {
  connection
    .open_table(table_name)
    .execute()
    .await
    .map_err(|e| anyhow!("Failed to open table '{}': {}", table_name, e))
}

/// Check if any embeddings exist in the table
async fn check_embeddings_exist(connection: &Connection, table_name: &str) -> Result<bool> {
  let table = open_table_by_name(connection, table_name).await?;
  let count = table.count_rows(None).await?;
  Ok(count > 0)
}
