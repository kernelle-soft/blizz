//! Simple CLI tool to inspect LanceDB database contents
//!
//! Usage: cargo run --bin inspect_lancedb -p insights

use anyhow::{anyhow, Result};
use arrow::record_batch::RecordBatch;
use futures::stream::StreamExt;
use lancedb::{
  query::{ExecutableQuery, QueryBase},
  Connection, Table,
};
use std::path::{Path, PathBuf};

#[tokio::main]
async fn main() -> Result<()> {
  print_header();

  let db_path = get_lancedb_data_path();
  println!("Database path: {}", db_path.display());

  validate_database_exists(&db_path)?;
  let connection = connect_to_database(&db_path).await?;
  let tables = list_tables(&connection).await?;

  inspect_insights_table(&connection, &tables).await
}

/// Display the application header
fn print_header() {
  println!("🔍 LanceDB Database Inspector");
  println!("============================");
}

/// Validate that the database directory exists
fn validate_database_exists(db_path: &Path) -> Result<()> {
  if !db_path.exists() {
    println!(
      "❌ Database directory does not exist yet. Run the insights server first to create it."
    );
    return Err(anyhow!("Database directory not found"));
  }
  Ok(())
}

/// Connect to the LanceDB database
async fn connect_to_database(db_path: &Path) -> Result<Connection> {
  lancedb::connect(&db_path.to_string_lossy())
    .execute()
    .await
    .map_err(|e| anyhow!("Failed to connect to LanceDB: {}", e))
}

/// List all tables in the database
async fn list_tables(connection: &Connection) -> Result<Vec<String>> {
  let tables = connection
    .table_names()
    .execute()
    .await
    .map_err(|e| anyhow!("Failed to list tables: {}", e))?;

  println!("\n📋 Tables found: {tables:?}");
  Ok(tables)
}

/// Inspect the insights_embeddings table if it exists
async fn inspect_insights_table(connection: &Connection, tables: &[String]) -> Result<()> {
  const TARGET_TABLE: &str = "insights_embeddings";

  if !tables.contains(&TARGET_TABLE.to_string()) {
    print_table_not_found_message();
    return Ok(());
  }

  println!("\n📊 Inspecting '{TARGET_TABLE}' table...");

  let table = open_table(connection, TARGET_TABLE).await?;
  let count = get_table_row_count(&table).await?;

  println!("Total embeddings: {count}");

  if count > 0 {
    display_sample_records(&table).await?;
  }

  Ok(())
}

/// Open a specific table
async fn open_table(connection: &Connection, table_name: &str) -> Result<Table> {
  connection
    .open_table(table_name)
    .execute()
    .await
    .map_err(|e| anyhow!("Failed to open table '{}': {}", table_name, e))
}

/// Get the total number of rows in a table
async fn get_table_row_count(table: &Table) -> Result<usize> {
  table.count_rows(None).await.map_err(|e| anyhow!("Failed to count rows: {}", e))
}

/// Display sample records from the table
async fn display_sample_records(table: &Table) -> Result<()> {
  println!("\n📝 Sample records:");

  let mut results = table.query().limit(5).execute().await?;
  let mut record_count = 0;

  while let Some(batch_result) = results.next().await {
    let batch = batch_result.map_err(|e| anyhow!("Failed to read batch: {}", e))?;
    record_count = process_batch_records(&batch, record_count)?;
  }

  Ok(())
}

/// Process records from a single batch
fn process_batch_records(batch: &RecordBatch, mut record_count: usize) -> Result<usize> {
  let record_data = extract_record_columns(batch)?;

  for i in 0..batch.num_rows().min(3) {
    record_count += 1;
    display_single_record(&record_data, i, record_count)?;
  }

  Ok(record_count)
}

/// Extract the required columns from a batch
fn extract_record_columns(batch: &RecordBatch) -> Result<RecordColumns<'_>> {
  use arrow::array::StringArray;

  let id_col = batch.column_by_name("id").ok_or_else(|| anyhow!("Missing 'id' column"))?;
  let topic_col = batch.column_by_name("topic").ok_or_else(|| anyhow!("Missing 'topic' column"))?;
  let name_col = batch.column_by_name("name").ok_or_else(|| anyhow!("Missing 'name' column"))?;
  let overview_col =
    batch.column_by_name("overview").ok_or_else(|| anyhow!("Missing 'overview' column"))?;

  let id_array = id_col
    .as_any()
    .downcast_ref::<StringArray>()
    .ok_or_else(|| anyhow!("Failed to cast 'id' column"))?;
  let topic_array = topic_col
    .as_any()
    .downcast_ref::<StringArray>()
    .ok_or_else(|| anyhow!("Failed to cast 'topic' column"))?;
  let name_array = name_col
    .as_any()
    .downcast_ref::<StringArray>()
    .ok_or_else(|| anyhow!("Failed to cast 'name' column"))?;
  let overview_array = overview_col
    .as_any()
    .downcast_ref::<StringArray>()
    .ok_or_else(|| anyhow!("Failed to cast 'overview' column"))?;

  Ok(RecordColumns { id_array, topic_array, name_array, overview_array })
}

/// Display a single record's information
fn display_single_record(columns: &RecordColumns<'_>, index: usize, record_count: usize) -> Result<()> {
  println!("\n--- Record {record_count} ---");
  println!("  ID: {}", columns.id_array.value(index));
  println!("  Topic: {}", columns.topic_array.value(index));
  println!("  Name: {}", columns.name_array.value(index));

  let overview_preview = columns.overview_array.value(index).chars().take(50).collect::<String>();
  println!("  Overview: {overview_preview}...");

  Ok(())
}

/// Print message when target table is not found
fn print_table_not_found_message() {
  println!("❌ 'insights_embeddings' table not found.");
  println!("💡 Run the insights server and add some insights to create the table.");
}

/// Container for extracted Arrow column arrays
struct RecordColumns<'a> {
  id_array: &'a arrow::array::StringArray,
  topic_array: &'a arrow::array::StringArray,
  name_array: &'a arrow::array::StringArray,
  overview_array: &'a arrow::array::StringArray,
}

fn get_lancedb_data_path() -> PathBuf {
  dirs::home_dir()
    .unwrap_or_else(|| std::path::Path::new("/tmp").to_path_buf())
    .join(".blizz")
    .join("persistent")
    .join("insights")
    .join("lancedb")
}
