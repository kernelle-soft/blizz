//! Table management operations for LanceDB

use anyhow::{anyhow, Result};
use lancedb::{Connection, Table};
use arrow::record_batch::RecordBatchIterator;

use super::models::InsightRecord;
use super::records::records_to_arrow_batch;

/// Table manager for LanceDB operations
pub struct TableManager {
    connection: Connection,
    table_name: String,
}

impl TableManager {
    pub fn new(connection: Connection, table_name: String) -> Self {
        Self { connection, table_name }
    }

    /// Check if the target table exists
    pub async fn table_exists(&self) -> Result<bool> {
        let tables = self.connection.table_names()
            .execute()
            .await
            .map_err(|e| anyhow!("Failed to list tables: {}", e))?;
        Ok(tables.contains(&self.table_name))
    }

    /// Get the table instance
    pub async fn get_table(&self) -> Result<Table> {
        self.connection
            .open_table(&self.table_name)
            .execute()
            .await
            .map_err(|e| anyhow!("Failed to open table '{}': {}", self.table_name, e))
    }

    /// Create a new table with the first record
    pub async fn create_table_with_first_record(&self, record: &InsightRecord) -> Result<()> {
        let batch = records_to_arrow_batch(vec![record.clone()])?;
        let schema = batch.schema();
        let batch_iter = RecordBatchIterator::new(vec![Ok(batch)], schema);
        
        self.connection
            .create_table(&self.table_name, batch_iter)
            .execute()
            .await
            .map_err(|e| anyhow!("Failed to create table with first record: {}", e))?;
            
        bentley::info!(&format!(
            "Created table '{}' with first embedding for {}/{}", 
            self.table_name, record.topic, record.name
        ));
        Ok(())
    }

    /// Add a record to an existing table
    pub async fn add_record_to_existing_table(&self, record: &InsightRecord) -> Result<()> {
        let batch = records_to_arrow_batch(vec![record.clone()])?;
        let schema = batch.schema();
        let batch_iter = RecordBatchIterator::new(vec![Ok(batch)], schema);
        
        let table = self.get_table().await?;
        table
            .add(batch_iter)
            .execute()
            .await
            .map_err(|e| anyhow!("Failed to store embedding: {}", e))?;
            
        bentley::info!(&format!("Stored embedding for {}/{}", record.topic, record.name));
        Ok(())
    }

    /// Check if any embeddings exist in the database
    pub async fn has_embeddings(&self) -> Result<bool> {
        let table = self.get_table().await?;
        let count = table.count_rows(None).await?;
        Ok(count > 0)
    }

    /// Delete an insight's embedding
    pub async fn delete_embedding(&self, topic: &str, name: &str) -> Result<()> {
        let table = self.get_table().await?;
        let id = format!("{}:{}", topic, name);
        
        table
            .delete(&format!("id = '{}'", id))
            .await
            .map_err(|e| anyhow!("Failed to delete embedding: {}", e))?;
            
        bentley::info!(&format!("Deleted embedding for {}/{}", topic, name));
        Ok(())
    }
}
