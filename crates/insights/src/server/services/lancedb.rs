//! LanceDB service for vector similarity search
//!
//! This module provides integration with LanceDB for storing and searching
//! insight embeddings using vector similarity.

use anyhow::{anyhow, Result};
use lancedb::{connect, Connection, Table};
use lancedb::query::{QueryBase, ExecutableQuery};
use std::path::PathBuf;
use serde::{Deserialize, Serialize};
use chrono::Utc;
use arrow::array::{StringArray, Float32Array, Array};
use arrow::datatypes::{Schema, Field, DataType};
use arrow::record_batch::{RecordBatch, RecordBatchIterator};
use std::sync::Arc;

use crate::server::models::insight;

/// Record structure for storing in LanceDB
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InsightRecord {
    pub id: String,
    pub topic: String,  
    pub name: String,
    pub overview: String,
    pub details: String,
    pub embedding: Vec<f32>,
    pub created_at: String,
    pub updated_at: String,
}

/// LanceDB service for vector operations
pub struct LanceDbService {
    pub connection: Connection,
    pub table_name: String,
}

impl LanceDbService {
    /// Create a new LanceDB service
    pub async fn new(data_dir: PathBuf, table_name: &str) -> Result<Self> {
        // Create data directory if it doesn't exist
        if !data_dir.exists() {
            std::fs::create_dir_all(&data_dir)?;
        }

        // Connect to LanceDB (creates database if it doesn't exist)
        let connection = connect(&data_dir.to_string_lossy())
            .execute()
            .await
            .map_err(|e| anyhow!("Failed to connect to LanceDB: {}", e))?;

        let service = Self {
            connection,
            table_name: table_name.to_string(),
        };

        Ok(service)
    }

    /// Store an insight's embedding in LanceDB
    pub async fn store_embedding(&self, insight: &insight::Insight) -> Result<()> {
        if let Some(embedding) = &insight.embedding {
            let record = InsightRecord {
                id: format!("{}:{}", insight.topic, insight.name),
                topic: insight.topic.clone(),
                name: insight.name.clone(),
                overview: insight.overview.clone(),
                details: insight.details.clone(),
                embedding: embedding.clone(),
                created_at: insight.embedding_computed.map(|t| t.to_rfc3339()).unwrap_or_else(|| Utc::now().to_rfc3339()),
                updated_at: Utc::now().to_rfc3339(),
            };

            // Convert to Arrow RecordBatch and create iterator  
            let batch = records_to_arrow_batch(vec![record])?;
            let schema = batch.schema();
            let batch_iter = RecordBatchIterator::new(vec![Ok(batch)], schema);

            // Check if table exists, if not create it on first insert
            let tables = self.connection.table_names()
                .execute()
                .await
                .map_err(|e| anyhow!("Failed to list tables: {}", e))?;
                
            if !tables.contains(&self.table_name) {
                // Create table with first record
                self.connection
                    .create_table(&self.table_name, batch_iter)
                    .execute()
                    .await
                    .map_err(|e| anyhow!("Failed to create table with first record: {}", e))?;
                bentley::info!(&format!("Created table '{}' with first embedding for {}/{}", self.table_name, insight.topic, insight.name));
            } else {
                // Table exists, add record - need to recreate iterator since it was consumed
                let batch2 = records_to_arrow_batch(vec![
                    InsightRecord {
                        id: format!("{}:{}", insight.topic, insight.name),
                        topic: insight.topic.clone(),
                        name: insight.name.clone(),
                        overview: insight.overview.clone(),
                        details: insight.details.clone(),
                        embedding: embedding.clone(),
                        created_at: insight.embedding_computed.map(|t| t.to_rfc3339()).unwrap_or_else(|| Utc::now().to_rfc3339()),
                        updated_at: Utc::now().to_rfc3339(),
                    }
                ])?;
                let schema2 = batch2.schema();
                let batch_iter2 = RecordBatchIterator::new(vec![Ok(batch2)], schema2);
                
                let table = self.get_table().await?;
                table
                    .add(batch_iter2)
                    .execute()
                    .await
                    .map_err(|e| anyhow!("Failed to store embedding: {}", e))?;
                bentley::info!(&format!("Stored embedding for {}/{}", insight.topic, insight.name));
            }
        } else {
            return Err(anyhow!("Insight has no embedding to store"));
        }
        
        Ok(())
    }

    /// Search for similar embeddings
    pub async fn search_similar(
        &self,
        query_embedding: &[f32],
        limit: usize,
        threshold: Option<f32>,
    ) -> Result<Vec<EmbeddingSearchResult>> {
        let table = self.get_table().await?;
        
        let query = table
            .vector_search(query_embedding)?
            .column("embedding")
            .limit(limit);
        
        if let Some(thresh) = threshold {
            // Skip where clause for now - it's not working in the API we're using
            bentley::info!(&format!("Threshold {} specified but skipping where clause", thresh));
        }

        let mut results_stream = query
            .execute()
            .await
            .map_err(|e| anyhow!("Vector search failed: {}", e))?;

        // Convert Arrow results back to EmbeddingSearchResult
        let mut search_results = Vec::new();
        
        // Use futures stream to read from the stream
        use futures::stream::StreamExt;
        
        while let Some(batch_result) = results_stream.next().await {
            let batch = batch_result.map_err(|e| anyhow!("Error reading batch: {}", e))?;
            
            let id_array = batch.column_by_name("id").unwrap().as_any().downcast_ref::<StringArray>().unwrap();
            let topic_array = batch.column_by_name("topic").unwrap().as_any().downcast_ref::<StringArray>().unwrap();
            let name_array = batch.column_by_name("name").unwrap().as_any().downcast_ref::<StringArray>().unwrap();
            let overview_array = batch.column_by_name("overview").unwrap().as_any().downcast_ref::<StringArray>().unwrap();
            let details_array = batch.column_by_name("details").unwrap().as_any().downcast_ref::<StringArray>().unwrap();
            
            // Distance might be in _distance column
            let default_distance = 0.1;
            
            for i in 0..batch.num_rows() {
                let distance = if let Some(distance_col) = batch.column_by_name("_distance") {
                    if let Some(distance_array) = distance_col.as_any().downcast_ref::<Float32Array>() {
                        if i < distance_array.len() && !distance_array.is_null(i) {
                            distance_array.value(i)
                        } else { default_distance }
                    } else { default_distance }
                } else { default_distance };
                
                search_results.push(EmbeddingSearchResult {
                    id: id_array.value(i).to_string(),
                    topic: topic_array.value(i).to_string(),
                    name: name_array.value(i).to_string(),
                    overview: overview_array.value(i).to_string(),
                    details: details_array.value(i).to_string(),
                    similarity: 1.0 - distance, // Convert distance to similarity
                });
            }
        }

        bentley::info!(&format!("Found {} similar embeddings", search_results.len()));
        Ok(search_results)
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

    /// Update an insight's embedding
    pub async fn update_embedding(&self, insight: &insight::Insight) -> Result<()> {
        // For simplicity, delete and re-insert
        self.delete_embedding(&insight.topic, &insight.name).await?;
        self.store_embedding(insight).await?;
        Ok(())
    }

    /// Get all stored embeddings (for debugging)
    pub async fn get_all_embeddings(&self) -> Result<Vec<EmbeddingSearchResult>> {
        // TODO: Implement get all with current LanceDB API
        bentley::info!("Get all embeddings not implemented yet - returning empty results");
        Ok(Vec::new())
    }

    /// Get the table instance
    pub async fn get_table(&self) -> Result<Table> {
        self.connection
            .open_table(&self.table_name)
            .execute()
            .await
            .map_err(|e| anyhow!("Failed to open table '{}': {}", self.table_name, e))
    }
}

/// Convert InsightRecord to Arrow RecordBatch
fn records_to_arrow_batch(records: Vec<InsightRecord>) -> Result<RecordBatch> {
    if records.is_empty() {
        return Err(anyhow!("Cannot create RecordBatch from empty records"));
    }

    // Create schema
    let schema = Arc::new(Schema::new(vec![
        Field::new("id", DataType::Utf8, false),
        Field::new("topic", DataType::Utf8, false), 
        Field::new("name", DataType::Utf8, false),
        Field::new("overview", DataType::Utf8, false),
        Field::new("details", DataType::Utf8, false),
        Field::new("embedding", DataType::FixedSizeList(Arc::new(Field::new("item", DataType::Float32, true)), 384), false),
        Field::new("created_at", DataType::Utf8, false),
        Field::new("updated_at", DataType::Utf8, false),
    ]));

    // Extract data into separate vectors
    let ids: Vec<Option<&str>> = records.iter().map(|r| Some(r.id.as_str())).collect();
    let topics: Vec<Option<&str>> = records.iter().map(|r| Some(r.topic.as_str())).collect();
    let names: Vec<Option<&str>> = records.iter().map(|r| Some(r.name.as_str())).collect();
    let overviews: Vec<Option<&str>> = records.iter().map(|r| Some(r.overview.as_str())).collect();
    let details: Vec<Option<&str>> = records.iter().map(|r| Some(r.details.as_str())).collect();
    let created_ats: Vec<Option<&str>> = records.iter().map(|r| Some(r.created_at.as_str())).collect();
    let updated_ats: Vec<Option<&str>> = records.iter().map(|r| Some(r.updated_at.as_str())).collect();

    // Create string arrays
    let id_array = StringArray::from(ids);
    let topic_array = StringArray::from(topics);
    let name_array = StringArray::from(names);
    let overview_array = StringArray::from(overviews);
    let details_array = StringArray::from(details);
    let created_at_array = StringArray::from(created_ats);
    let updated_at_array = StringArray::from(updated_ats);

    // Create embedding fixed-size list array
    use arrow::array::FixedSizeListBuilder;
    let mut embedding_builder = FixedSizeListBuilder::new(Float32Array::builder(384 * records.len()), 384);
    for record in &records {
        for &value in &record.embedding {
            embedding_builder.values().append_value(value);
        }
        embedding_builder.append(true); // valid row
    }
    let embedding_array = embedding_builder.finish();

    // Create record batch
    let batch = RecordBatch::try_new(
        schema,
        vec![
            Arc::new(id_array),
            Arc::new(topic_array),
            Arc::new(name_array),
            Arc::new(overview_array),
            Arc::new(details_array),
            Arc::new(embedding_array),
            Arc::new(created_at_array),
            Arc::new(updated_at_array),
        ],
    )?;

    Ok(batch)
}

/// Result of an embedding similarity search
#[derive(Debug, Clone)]
pub struct EmbeddingSearchResult {
    pub id: String,
    pub topic: String,
    pub name: String,
    pub overview: String,
    pub details: String,
    pub similarity: f32,
}

