//! LanceDB service for vector similarity search
//!
//! This module provides integration with LanceDB for storing and searching
//! insight embeddings using vector similarity.

pub mod models;
pub mod connection;
pub mod table_manager;
pub mod search;
pub mod records;

use anyhow::{anyhow, Result};
use std::path::PathBuf;
use chrono::Utc;

use crate::server::models::insight;
use connection::create_connection;
use table_manager::TableManager;
use search::search_similar_embeddings;

// Re-export commonly used types for external use
pub use models::{InsightRecord, EmbeddingSearchResult};

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
}

/// Validate that insight has an embedding
fn validate_insight_has_embedding(insight: &insight::Insight) -> Result<&[f32]> {
    insight.embedding.as_deref()
        .ok_or_else(|| anyhow!("Insight has no embedding to store"))
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
async fn store_record_appropriately(table_manager: &TableManager, record: &models::InsightRecord) -> Result<()> {
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
    insight.embedding_computed
        .map(|t| t.to_rfc3339())
        .unwrap_or_else(|| Utc::now().to_rfc3339())
}
