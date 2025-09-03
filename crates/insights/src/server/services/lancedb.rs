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
        // Early return validation
        let embedding = insight.embedding.as_ref()
            .ok_or_else(|| anyhow!("Insight has no embedding to store"))?;
            
        let record = self.create_insight_record(insight, embedding);
        
        if self.table_exists().await? {
            self.add_record_to_existing_table(&record).await
        } else {
            self.create_table_with_first_record(&record).await
        }
    }

    /// Create an InsightRecord from an insight and embedding
    fn create_insight_record(&self, insight: &insight::Insight, embedding: &[f32]) -> InsightRecord {
        InsightRecord {
            id: format!("{}:{}", insight.topic, insight.name),
            topic: insight.topic.clone(),
            name: insight.name.clone(),
            overview: insight.overview.clone(),
            details: insight.details.clone(),
            embedding: embedding.to_vec(),
            created_at: insight.embedding_computed
                .map(|t| t.to_rfc3339())
                .unwrap_or_else(|| Utc::now().to_rfc3339()),
            updated_at: Utc::now().to_rfc3339(),
        }
    }

    /// Check if the target table exists
    async fn table_exists(&self) -> Result<bool> {
        let tables = self.connection.table_names()
            .execute()
            .await
            .map_err(|e| anyhow!("Failed to list tables: {}", e))?;
        Ok(tables.contains(&self.table_name))
    }

    /// Create a new table with the first record
    async fn create_table_with_first_record(&self, record: &InsightRecord) -> Result<()> {
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
    async fn add_record_to_existing_table(&self, record: &InsightRecord) -> Result<()> {
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

    /// Search for similar embeddings
    pub async fn search_similar(
        &self,
        query_embedding: &[f32],
        limit: usize,
        threshold: Option<f32>,
    ) -> Result<Vec<EmbeddingSearchResult>> {
        let table = self.get_table().await?;
        let mut results_stream = self.create_search_query(&table, query_embedding, limit, threshold).await?;
        let search_results = self.process_all_batches(&mut results_stream, threshold).await?;
        
        bentley::info!(&format!("Found {} similar embeddings (threshold: {:?})", search_results.len(), threshold));
        Ok(search_results)
    }

    /// Create and execute vector search query
    async fn create_search_query(
        &self,
        table: &Table,
        query_embedding: &[f32],
        limit: usize,
        threshold: Option<f32>,
    ) -> Result<impl futures::stream::Stream<Item = Result<RecordBatch, lancedb::Error>> + '_> {
        let query = table
            .vector_search(query_embedding)?
            .column("embedding")
            .limit(limit);
        
        if let Some(thresh) = threshold {
            bentley::info!(&format!("Threshold {} specified but skipping where clause", thresh));
        }

        query
            .execute()
            .await
            .map_err(|e| anyhow!("Vector search failed: {}", e))
    }

    /// Process all batches from the stream
    async fn process_all_batches(
        &self,
        results_stream: &mut (impl futures::stream::Stream<Item = Result<RecordBatch, lancedb::Error>> + Unpin),
        threshold: Option<f32>,
    ) -> Result<Vec<EmbeddingSearchResult>> {
        use futures::stream::StreamExt;
        
        let mut search_results = Vec::new();
        
        while let Some(batch_result) = results_stream.next().await {
            let batch = batch_result.map_err(|e| anyhow!("Error reading batch: {}", e))?;
            let batch_results = self.process_result_batch(&batch, threshold)?;
            search_results.extend(batch_results);
        }
        
        Ok(search_results)
    }

    /// Process a single result batch into EmbeddingSearchResult entries
    fn process_result_batch(
        &self,
        batch: &RecordBatch,
        threshold: Option<f32>,
    ) -> Result<Vec<EmbeddingSearchResult>> {
        let mut batch_results = Vec::new();
        
        // Extract column arrays directly within scope
        let id_array = batch.column_by_name("id")
            .ok_or_else(|| anyhow!("Missing 'id' column"))?
            .as_any().downcast_ref::<StringArray>()
            .ok_or_else(|| anyhow!("Failed to cast 'id' column to StringArray"))?;
        let topic_array = batch.column_by_name("topic")
            .ok_or_else(|| anyhow!("Missing 'topic' column"))?
            .as_any().downcast_ref::<StringArray>()
            .ok_or_else(|| anyhow!("Failed to cast 'topic' column to StringArray"))?;
        let name_array = batch.column_by_name("name")
            .ok_or_else(|| anyhow!("Missing 'name' column"))?
            .as_any().downcast_ref::<StringArray>()
            .ok_or_else(|| anyhow!("Failed to cast 'name' column to StringArray"))?;
        let overview_array = batch.column_by_name("overview")
            .ok_or_else(|| anyhow!("Missing 'overview' column"))?
            .as_any().downcast_ref::<StringArray>()
            .ok_or_else(|| anyhow!("Failed to cast 'overview' column to StringArray"))?;
        let details_array = batch.column_by_name("details")
            .ok_or_else(|| anyhow!("Missing 'details' column"))?
            .as_any().downcast_ref::<StringArray>()
            .ok_or_else(|| anyhow!("Failed to cast 'details' column to StringArray"))?;
        let distance_array = batch.column_by_name("_distance")
            .and_then(|col| col.as_any().downcast_ref::<Float32Array>());
        
        for i in 0..batch.num_rows() {
            let distance = self.extract_distance_from_arrays(distance_array, i);
            let similarity = self.convert_distance_to_similarity(distance);
            
            if !self.passes_threshold_filter(similarity, threshold) {
                continue;
            }
            
            batch_results.push(EmbeddingSearchResult {
                id: id_array.value(i).to_string(),
                topic: topic_array.value(i).to_string(),
                name: name_array.value(i).to_string(),
                overview: overview_array.value(i).to_string(),
                details: details_array.value(i).to_string(),
                similarity,
            });
        }
        
        Ok(batch_results)
    }

    /// Extract distance value from distance array at specific row
    fn extract_distance_from_arrays(&self, distance_array: Option<&Float32Array>, row_index: usize) -> f32 {
        const DEFAULT_DISTANCE: f32 = 0.025;
        
        if let Some(distance_array) = distance_array {
            if row_index < distance_array.len() && !distance_array.is_null(row_index) {
                distance_array.value(row_index)
            } else {
                DEFAULT_DISTANCE
            }
        } else {
            DEFAULT_DISTANCE
        }
    }

    /// Convert Euclidean distance to similarity score for normalized vectors
    fn convert_distance_to_similarity(&self, distance: f32) -> f32 {
        // For normalized vectors, distance is in [0, 2] range
        // Use linear conversion: similarity = 1 - (distance / 2)
        (2.0 - distance.min(2.0)) / 2.0
    }

    /// Check if similarity passes the threshold filter
    fn passes_threshold_filter(&self, similarity: f32, threshold: Option<f32>) -> bool {
        if let Some(thresh) = threshold {
            if similarity < thresh {
                bentley::info!(&format!("Skipping result: similarity {:.6} < threshold {:.6}", similarity, thresh));
                false
            } else {
                true
            }
        } else {
            true
        }
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
    validate_records_not_empty(&records)?;
    
    let schema = create_insight_record_schema();
    let string_arrays = create_string_arrays_from_records(&records);
    let embedding_array = create_embedding_array_from_records(&records);
    
    assemble_record_batch(schema, string_arrays, embedding_array)
}

/// Validate that records vector is not empty
fn validate_records_not_empty(records: &[InsightRecord]) -> Result<()> {
    if records.is_empty() {
        return Err(anyhow!("Cannot create RecordBatch from empty records"));
    }
    Ok(())
}

/// Create the Arrow schema for InsightRecord
fn create_insight_record_schema() -> Arc<Schema> {
    Arc::new(Schema::new(vec![
        Field::new("id", DataType::Utf8, false),
        Field::new("topic", DataType::Utf8, false), 
        Field::new("name", DataType::Utf8, false),
        Field::new("overview", DataType::Utf8, false),
        Field::new("details", DataType::Utf8, false),
        Field::new("embedding", DataType::FixedSizeList(Arc::new(Field::new("item", DataType::Float32, true)), 1024), false),
        Field::new("created_at", DataType::Utf8, false),
        Field::new("updated_at", DataType::Utf8, false),
    ]))
}

/// Container for all string arrays from records
struct RecordStringArrays {
    id_array: StringArray,
    topic_array: StringArray,
    name_array: StringArray,
    overview_array: StringArray,
    details_array: StringArray,
    created_at_array: StringArray,
    updated_at_array: StringArray,
}

/// Create string arrays from insight records
fn create_string_arrays_from_records(records: &[InsightRecord]) -> RecordStringArrays {
    RecordStringArrays {
        id_array: extract_string_field(records, |r| &r.id),
        topic_array: extract_string_field(records, |r| &r.topic),
        name_array: extract_string_field(records, |r| &r.name),
        overview_array: extract_string_field(records, |r| &r.overview),
        details_array: extract_string_field(records, |r| &r.details),
        created_at_array: extract_string_field(records, |r| &r.created_at),
        updated_at_array: extract_string_field(records, |r| &r.updated_at),
    }
}

/// Extract a string field from all records using a field accessor function
fn extract_string_field<F>(records: &[InsightRecord], field_fn: F) -> StringArray 
where
    F: Fn(&InsightRecord) -> &str,
{
    let field_values: Vec<Option<&str>> = records.iter().map(|r| Some(field_fn(r))).collect();
    StringArray::from(field_values)
}

/// Create embedding fixed-size list array from records
fn create_embedding_array_from_records(records: &[InsightRecord]) -> arrow::array::FixedSizeListArray {
    use arrow::array::FixedSizeListBuilder;
    
    let mut embedding_builder = FixedSizeListBuilder::new(Float32Array::builder(1024 * records.len()), 1024);
    
    for record in records {
        append_embedding_to_builder(&mut embedding_builder, &record.embedding);
    }
    
    embedding_builder.finish()
}

/// Append a single embedding vector to the builder
fn append_embedding_to_builder(builder: &mut arrow::array::FixedSizeListBuilder<arrow::array::builder::Float32Builder>, embedding: &[f32]) {
    for &value in embedding {
        builder.values().append_value(value);
    }
    builder.append(true); // valid row
}

/// Assemble final RecordBatch from schema and arrays
fn assemble_record_batch(
    schema: Arc<Schema>,
    string_arrays: RecordStringArrays,
    embedding_array: arrow::array::FixedSizeListArray,
) -> Result<RecordBatch> {
    let column_arrays = prepare_column_arrays(string_arrays, embedding_array);
    
    RecordBatch::try_new(schema, column_arrays)
        .map_err(|e| anyhow!("Failed to create RecordBatch: {}", e))
}

/// Prepare all column arrays for RecordBatch creation
fn prepare_column_arrays(
    string_arrays: RecordStringArrays, 
    embedding_array: arrow::array::FixedSizeListArray
) -> Vec<Arc<dyn Array>> {
    vec![
        Arc::new(string_arrays.id_array),
        Arc::new(string_arrays.topic_array),
        Arc::new(string_arrays.name_array),
        Arc::new(string_arrays.overview_array),
        Arc::new(string_arrays.details_array),
        Arc::new(embedding_array),
        Arc::new(string_arrays.created_at_array),
        Arc::new(string_arrays.updated_at_array),
    ]
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