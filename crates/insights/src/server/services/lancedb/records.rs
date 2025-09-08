//! Arrow RecordBatch conversion utilities for LanceDB

use anyhow::{anyhow, Result};
use arrow::array::{Array, Float32Array, StringArray};
use arrow::datatypes::{DataType, Field, Schema};
use arrow::record_batch::RecordBatch;
use std::sync::Arc;

use super::get_schema_dimension;
use super::models::InsightRecord;
use super::get_schema_dimension;

/// Convert InsightRecord to Arrow RecordBatch
pub fn records_to_arrow_batch(records: Vec<InsightRecord>) -> Result<RecordBatch> {
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
  let embedding_dimension = get_schema_dimension();
  Arc::new(Schema::new(vec![
    Field::new("id", DataType::Utf8, false),
    Field::new("topic", DataType::Utf8, false),
    Field::new("name", DataType::Utf8, false),
    Field::new("overview", DataType::Utf8, false),
    Field::new("details", DataType::Utf8, false),
    Field::new(
      "embedding",
      DataType::FixedSizeList(
        Arc::new(Field::new("item", DataType::Float32, true)),
        embedding_dimension as i32,
      ),
      false,
    ),
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
fn create_embedding_array_from_records(
  records: &[InsightRecord],
) -> arrow::array::FixedSizeListArray {
  use arrow::array::FixedSizeListBuilder;

  let embedding_dimension = get_schema_dimension();
  let mut embedding_builder = FixedSizeListBuilder::new(
    Float32Array::builder(embedding_dimension * records.len()),
    embedding_dimension as i32,
  );

  for record in records {
    append_embedding_to_builder(&mut embedding_builder, &record.embedding);
  }

  embedding_builder.finish()
}

/// Append a single embedding vector to the builder
fn append_embedding_to_builder(
  builder: &mut arrow::array::FixedSizeListBuilder<arrow::array::builder::Float32Builder>,
  embedding: &[f32],
) {
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
  embedding_array: arrow::array::FixedSizeListArray,
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
