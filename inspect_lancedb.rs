//! Simple CLI tool to inspect LanceDB database contents
//!
//! Usage: cargo run --bin inspect_lancedb -p insights

use anyhow::{anyhow, Result};
use lancedb::query::{QueryBase, ExecutableQuery};
use std::path::PathBuf;

#[tokio::main]
async fn main() -> Result<()> {
    println!("🔍 LanceDB Database Inspector");
    println!("============================");

    // Get database path
    let db_path = get_lancedb_data_path();
    println!("Database path: {}", db_path.display());

    if !db_path.exists() {
        println!("❌ Database directory does not exist yet. Run the insights server first to create it.");
        return Ok(());
    }

    // Connect to LanceDB
    let connection = lancedb::connect(&db_path.to_string_lossy())
        .execute()
        .await
        .map_err(|e| anyhow!("Failed to connect to LanceDB: {}", e))?;

    // List tables
    let tables = connection.table_names()
        .execute()
        .await
        .map_err(|e| anyhow!("Failed to list tables: {}", e))?;

    println!("\n📋 Tables found: {:?}", tables);

    if tables.contains(&"insights_embeddings".to_string()) {
        println!("\n📊 Inspecting 'insights_embeddings' table...");

        let table = connection.open_table("insights_embeddings")
            .execute()
            .await
            .map_err(|e| anyhow!("Failed to open table: {}", e))?;

        // Get row count
        let count = table.count_rows(None)
            .await
            .map_err(|e| anyhow!("Failed to count rows: {}", e))?;

        println!("Total embeddings: {}", count);

        // Get sample records
        if count > 0 {
            println!("\n📝 Sample records:");

            // Use query to get first 5 records
            let mut results = table.query().limit(5).execute().await?;
            use futures::stream::StreamExt;

            let mut record_count = 0;
            while let Some(batch_result) = results.next().await {
                let batch = batch_result.map_err(|e| anyhow!("Failed to read batch: {}", e))?;

                // Extract data from Arrow columns
                if let (Some(id_col), Some(topic_col), Some(name_col), Some(overview_col)) = (
                    batch.column_by_name("id"),
                    batch.column_by_name("topic"),
                    batch.column_by_name("name"),
                    batch.column_by_name("overview"),
                ) {
                    if let (Some(id_array), Some(topic_array), Some(name_array), Some(overview_array)) = (
                        id_col.as_any().downcast_ref::<arrow::array::StringArray>(),
                        topic_col.as_any().downcast_ref::<arrow::array::StringArray>(),
                        name_col.as_any().downcast_ref::<arrow::array::StringArray>(),
                        overview_col.as_any().downcast_ref::<arrow::array::StringArray>(),
                    ) {
                        for i in 0..batch.num_rows().min(3) {
                            record_count += 1;
                            println!("\n--- Record {} ---", record_count);
                            println!("  ID: {}", id_array.value(i));
                            println!("  Topic: {}", topic_array.value(i));
                            println!("  Name: {}", name_array.value(i));
                            println!("  Overview: {:.50}...",
                                overview_array.value(i).chars().take(50).collect::<String>());
                        }
                    }
                }
            }
        }

    } else {
        println!("❌ 'insights_embeddings' table not found.");
        println!("💡 Run the insights server and add some insights to create the table.");
    }

    Ok(())
}

fn get_lancedb_data_path() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| std::path::Path::new("/tmp").to_path_buf())
        .join(".blizz")
        .join("persistent")
        .join("insights")
        .join("lancedb")
}
