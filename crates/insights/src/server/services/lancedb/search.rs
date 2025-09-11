//! Vector search operations and result processing for LanceDB

use anyhow::{anyhow, Result};
use arrow::array::{Array, Float32Array, StringArray};
use arrow::record_batch::RecordBatch;
use futures::stream::StreamExt;
use lancedb::query::{ExecutableQuery, QueryBase};
use lancedb::Table;

use super::models::EmbeddingSearchResult;

/// Perform vector search and return processed results
pub async fn search_similar_embeddings(
  table: &Table,
  query_embedding: &[f32],
  limit: usize,
  threshold: Option<f32>,
) -> Result<Vec<EmbeddingSearchResult>> {
  let mut results_stream = create_search_query(table, query_embedding, limit, threshold).await?;
  let search_results = process_all_batches(&mut results_stream, threshold).await?;

  // Only log search results for debugging purposes
  if search_results.is_empty() {
    bentley::verbose!("No similar embeddings found");
  }
  Ok(search_results)
}

/// Create and execute vector search query
async fn create_search_query<'a>(
  table: &'a Table,
  query_embedding: &[f32],
  limit: usize,
  _threshold: Option<f32>,
) -> Result<impl futures::stream::Stream<Item = Result<RecordBatch, lancedb::Error>> + 'a> {
  let query = table.vector_search(query_embedding)?.column("embedding").limit(limit);

  // Skip verbose threshold logging to reduce noise

  query.execute().await.map_err(|e| anyhow!("Vector search failed: {}", e))
}

/// Process all batches from the stream
async fn process_all_batches(
  results_stream: &mut (impl futures::stream::Stream<Item = Result<RecordBatch, lancedb::Error>>
          + Unpin),
  threshold: Option<f32>,
) -> Result<Vec<EmbeddingSearchResult>> {
  let mut search_results = Vec::new();

  while let Some(batch_result) = results_stream.next().await {
    let batch = batch_result.map_err(|e| anyhow!("Error reading batch: {}", e))?;
    let batch_results = process_result_batch(&batch, threshold)?;
    search_results.extend(batch_results);
  }

  Ok(search_results)
}

/// Process a single result batch into EmbeddingSearchResult entries
fn process_result_batch(
  batch: &RecordBatch,
  threshold: Option<f32>,
) -> Result<Vec<EmbeddingSearchResult>> {
  let column_arrays = extract_column_arrays_from_batch(batch)?;
  let mut batch_results = Vec::new();

  for i in 0..batch.num_rows() {
    let distance = extract_distance_from_arrays(column_arrays.distance_array, i);
    let similarity = convert_distance_to_similarity(distance);

    if !passes_threshold_filter(similarity, threshold) {
      continue;
    }

    batch_results.push(create_search_result_from_arrays(&column_arrays, i, similarity));
  }

  Ok(batch_results)
}

/// Container for all column arrays extracted from a batch
struct BatchColumnArrays<'a> {
  id_array: &'a StringArray,
  topic_array: &'a StringArray,
  name_array: &'a StringArray,
  overview_array: &'a StringArray,
  details_array: &'a StringArray,
  distance_array: Option<&'a Float32Array>,
}

/// Extract all column arrays from the result batch
fn extract_column_arrays_from_batch(batch: &RecordBatch) -> Result<BatchColumnArrays<'_>> {
  let id_array = extract_string_column(batch, "id")?;
  let topic_array = extract_string_column(batch, "topic")?;
  let name_array = extract_string_column(batch, "name")?;
  let overview_array = extract_string_column(batch, "overview")?;
  let details_array = extract_string_column(batch, "details")?;
  let distance_array = extract_distance_column(batch);

  Ok(BatchColumnArrays {
    id_array,
    topic_array,
    name_array,
    overview_array,
    details_array,
    distance_array,
  })
}

/// Extract a string column from the batch
fn extract_string_column<'a>(batch: &'a RecordBatch, column_name: &str) -> Result<&'a StringArray> {
  batch
    .column_by_name(column_name)
    .ok_or_else(|| anyhow!("Missing '{}' column", column_name))?
    .as_any()
    .downcast_ref::<StringArray>()
    .ok_or_else(|| anyhow!("Failed to cast '{}' column to StringArray", column_name))
}

/// Extract the distance column from the batch (optional)
fn extract_distance_column(batch: &RecordBatch) -> Option<&Float32Array> {
  batch.column_by_name("_distance").and_then(|col| col.as_any().downcast_ref::<Float32Array>())
}

/// Extract distance value from distance array at specific row
fn extract_distance_from_arrays(distance_array: Option<&Float32Array>, row_index: usize) -> f32 {
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
fn convert_distance_to_similarity(distance: f32) -> f32 {
  // For normalized vectors, distance is in [0, 2] range
  // Use linear conversion: similarity = 1 - (distance / 2)
  (2.0 - distance.min(2.0)) / 2.0
}

/// Check if similarity passes the threshold filter
fn passes_threshold_filter(similarity: f32, threshold: Option<f32>) -> bool {
  if let Some(thresh) = threshold {
    similarity >= thresh
  } else {
    true
  }
}

/// Create EmbeddingSearchResult from column arrays at specific row index
fn create_search_result_from_arrays(
  column_arrays: &BatchColumnArrays<'_>,
  row_index: usize,
  similarity: f32,
) -> EmbeddingSearchResult {
  EmbeddingSearchResult {
    id: column_arrays.id_array.value(row_index).to_string(),
    topic: column_arrays.topic_array.value(row_index).to_string(),
    name: column_arrays.name_array.value(row_index).to_string(),
    overview: column_arrays.overview_array.value(row_index).to_string(),
    details: column_arrays.details_array.value(row_index).to_string(),
    similarity,
  }
}
