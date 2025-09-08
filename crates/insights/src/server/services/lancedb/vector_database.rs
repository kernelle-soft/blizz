//! LanceDB implementation of the VectorDatabase trait
//!
//! This module provides an adapter that implements the generic VectorDatabase
//! interface using LanceDB as the underlying vector storage engine.

use anyhow::Result;
use async_trait::async_trait;
use std::path::PathBuf;

use crate::server::models::insight;
use crate::server::services::lancedb::LanceDbService;
use crate::server::services::vector_database::{VectorDatabase, VectorSearchResult};

/// LanceDB implementation of the VectorDatabase trait
pub struct LanceDbVectorDatabase {
  service: LanceDbService,
}

impl LanceDbVectorDatabase {
  /// Create a new LanceDB vector database instance
  pub async fn new(data_dir: PathBuf, table_name: &str) -> Result<Self> {
    let service = LanceDbService::new(data_dir, table_name).await?;
    Ok(Self { service })
  }
}

#[async_trait]
impl VectorDatabase for LanceDbVectorDatabase {
  /// Store an insight's embedding in LanceDB
  async fn store_embedding(&self, insight: &insight::Insight) -> Result<()> {
    self.service.store_embedding(insight).await
  }

  /// Search for similar embeddings using LanceDB
  async fn search_similar(
    &self,
    query_embedding: &[f32],
    limit: usize,
    threshold: Option<f32>,
  ) -> Result<Vec<VectorSearchResult>> {
    let lance_results = self.service.search_similar(query_embedding, limit, threshold).await?;

    // Convert LanceDB-specific results to generic VectorSearchResult
    let generic_results = lance_results
      .into_iter()
      .map(|result| VectorSearchResult {
        id: result.id,
        topic: result.topic,
        name: result.name,
        overview: result.overview,
        details: result.details,
        similarity: result.similarity,
      })
      .collect();

    Ok(generic_results)
  }

  /// Check if LanceDB has any embeddings
  async fn has_embeddings(&self) -> Result<bool> {
    self.service.has_embeddings().await
  }

  /// Delete an insight's embedding from LanceDB
  async fn delete_embedding(&self, topic: &str, name: &str) -> Result<()> {
    self.service.delete_embedding(topic, name).await
  }

  /// Update an insight's embedding in LanceDB
  async fn update_embedding(&self, insight: &insight::Insight) -> Result<()> {
    self.service.update_embedding(insight).await
  }

  /// Get all stored embeddings from LanceDB
  async fn get_all_embeddings(&self) -> Result<Vec<VectorSearchResult>> {
    let lance_results = self.service.get_all_embeddings().await?;

    // Convert to generic format
    let generic_results = lance_results
      .into_iter()
      .map(|result| VectorSearchResult {
        id: result.id,
        topic: result.topic,
        name: result.name,
        overview: result.overview,
        details: result.details,
        similarity: result.similarity,
      })
      .collect();

    Ok(generic_results)
  }

  /// Clear all embeddings from LanceDB
  async fn clear_all_embeddings(&self) -> Result<()> {
    self.service.clear_all_embeddings().await
  }

  /// Reshape LanceDB with fresh schema
  async fn reshape_database(&self, embedding_dimension: usize) -> Result<()> {
    self.service.reshape_database(embedding_dimension).await
  }
}
