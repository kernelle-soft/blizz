// violet ignore file -- test file for complex lazy embedding recomputation interaction
use anyhow::Result;
use insights::embedding_client::{self, MockEmbeddingService};
use insights::insight::{self, Insight};
use insights::search::{self, SearchOptions};
use serial_test::serial;
use std::env;
use tempfile::TempDir;

fn setup_temp_insights_root(_test_name: &str) -> TempDir {
  let temp_dir = TempDir::new().unwrap();
  env::set_var("INSIGHTS_ROOT", temp_dir.path());
  temp_dir
}

#[test]
#[serial]
fn test_existing_embedding_not_overwritten() -> Result<()> {
  let _temp = setup_temp_insights_root("existing_embedding");

  // Create an insight and manually set an embedding
  let mut insight = Insight::new(
    "ExistingTopic".to_string(),
    "ExistingName".to_string(),
    "Existing insight overview".to_string(),
    "Existing insight details with embedding".to_string(),
  );

  let original_embedding = embedding_client::Embedding {
    version: "original-version".to_string(),
    created_at: chrono::Utc::now(),
    embedding: vec![0.9, 0.8, 0.7], // Custom embedding different from mock
  };
  insight::set_embedding(&mut insight, original_embedding);
  insight::save(&insight)?;

  let _mock_client = embedding_client::with_service(Box::new(MockEmbeddingService));
  let search_options =
    SearchOptions { topic: None, case_sensitive: false, overview_only: false, exact: false };

  let results = search::search(&["embedding".to_string()], &search_options)?;

  assert!(!results.is_empty(), "Should have found the test insight");

  // Verify the original embedding was NOT overwritten
  let loaded_after = insight::load("ExistingTopic", "ExistingName")?;

  assert!(loaded_after.embedding.is_some(), "Should still have embedding");
  assert_eq!(
    loaded_after.embedding_version.unwrap(),
    "original-version",
    "Should keep original version"
  );

  let embedding = loaded_after.embedding.unwrap();
  assert_eq!(
    embedding,
    vec![0.9, 0.8, 0.7],
    "Should keep original embedding, not overwrite with mock"
  );

  Ok(())
}
