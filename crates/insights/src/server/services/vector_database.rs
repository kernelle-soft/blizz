//! Vector database abstraction layer for insights storage and retrieval
//!
//! This module provides a generic interface for vector database operations,
//! allowing different implementations (LanceDB, Qdrant, etc.) to be swapped
//! without changing the higher-level application code.

use anyhow::Result;
use async_trait::async_trait;

use crate::server::models::insight;

/// Generic search result from vector similarity operations
#[derive(Debug, Clone)]
pub struct VectorSearchResult {
  /// Unique identifier for the result
  pub id: String,
  /// Topic of the insight
  pub topic: String,
  /// Name of the insight
  pub name: String,
  /// Overview content
  pub overview: String,
  /// Detail content
  pub details: String,
  /// Similarity score (0.0-1.0, higher is more similar)
  pub similarity: f32,
}

/// Vector database interface for storing and searching insight embeddings
#[async_trait]
pub trait VectorDatabase: Send + Sync {
  /// Store an insight's embedding in the database
  async fn store_embedding(&self, insight: &insight::Insight) -> Result<()>;

  /// Search for similar embeddings
  async fn search_similar(
    &self,
    query_embedding: &[f32],
    limit: usize,
    threshold: Option<f32>,
  ) -> Result<Vec<VectorSearchResult>>;

  /// Check if any embeddings exist in the database
  async fn has_embeddings(&self) -> Result<bool>;

  /// Delete an insight's embedding
  async fn delete_embedding(&self, topic: &str, name: &str) -> Result<()>;

  /// Update an insight's embedding (replace existing)
  async fn update_embedding(&self, insight: &insight::Insight) -> Result<()>;

  /// Get all stored embeddings (for debugging/admin purposes)
  async fn get_all_embeddings(&self) -> Result<Vec<VectorSearchResult>>;

  /// Clear all embeddings from the database
  async fn clear_all_embeddings(&self) -> Result<()>;

  /// Recreate the database with fresh schema (clean slate approach)
  async fn recreate_database_clean_slate(&self, embedding_dimension: usize) -> Result<()>;
}

/// Type-erased wrapper for VectorDatabase implementations
pub struct BoxedVectorDatabase(Box<dyn VectorDatabase>);

impl BoxedVectorDatabase {
  pub fn new<T: VectorDatabase + 'static>(db: T) -> Self {
    Self(Box::new(db))
  }
}

#[async_trait]
impl VectorDatabase for BoxedVectorDatabase {
  async fn store_embedding(&self, insight: &insight::Insight) -> Result<()> {
    self.0.store_embedding(insight).await
  }

  async fn search_similar(
    &self,
    query_embedding: &[f32],
    limit: usize,
    threshold: Option<f32>,
  ) -> Result<Vec<VectorSearchResult>> {
    self.0.search_similar(query_embedding, limit, threshold).await
  }

  async fn has_embeddings(&self) -> Result<bool> {
    self.0.has_embeddings().await
  }

  async fn delete_embedding(&self, topic: &str, name: &str) -> Result<()> {
    self.0.delete_embedding(topic, name).await
  }

  async fn update_embedding(&self, insight: &insight::Insight) -> Result<()> {
    self.0.update_embedding(insight).await
  }

  async fn get_all_embeddings(&self) -> Result<Vec<VectorSearchResult>> {
    self.0.get_all_embeddings().await
  }

  async fn clear_all_embeddings(&self) -> Result<()> {
    self.0.clear_all_embeddings().await
  }

  async fn recreate_database_clean_slate(&self, embedding_dimension: usize) -> Result<()> {
    self.0.recreate_database_clean_slate(embedding_dimension).await
  }
}
