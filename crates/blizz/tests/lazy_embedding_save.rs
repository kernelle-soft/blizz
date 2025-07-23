use anyhow::Result;
use blizz::embedding_client::{self, MockEmbeddingService};
use blizz::insight::{self, Insight};
use blizz::search::{self, SearchOptions};
use serial_test::serial;
use std::env;
use tempfile::TempDir;

fn setup_temp_insights_root(_test_name: &str) -> TempDir {
  let temp_dir = TempDir::new().unwrap();
  env::set_var("BLIZZ_INSIGHTS_ROOT", temp_dir.path());
  temp_dir
}

#[test]
#[serial]
fn test_lazy_embedding_save_on_search() -> Result<()> {
  let _temp = setup_temp_insights_root("lazy_embedding_save");

  // Create an insight without embedding data
  let insight = Insight::new(
    "TestTopic".to_string(),
    "TestName".to_string(),
    "Test overview for lazy embedding".to_string(),
    "Test details for lazy embedding functionality. This content should trigger neural search."
      .to_string(),
  );

  // Save it without embeddings
  insight::save(&insight)?;

  // Verify it has no embedding initially
  let loaded_before = insight::load("TestTopic", "TestName")?;
  assert!(loaded_before.embedding.is_none(), "Should have no embedding initially");
  assert!(loaded_before.embedding_version.is_none(), "Should have no embedding version initially");

  // Create search options with mock embedding client
  let mock_client = embedding_client::with_service(Box::new(MockEmbeddingService));
  let search_options = SearchOptions {
    topic: None,
    case_sensitive: false,
    overview_only: false,
    #[cfg(feature = "semantic")]
    semantic: false, // Disable semantic to force neural search
    exact: false, // Disable exact to force neural search
    embedding_client: mock_client,
  };

  // Perform a search which should trigger lazy embedding computation and save
  let results = search::search(&["embedding".to_string()], &search_options)?;

  // Verify we got search results
  assert!(!results.is_empty(), "Should have found the test insight");
  assert_eq!(results[0].topic, "TestTopic");
  assert_eq!(results[0].name, "TestName");

  // Most importantly: verify embedding was computed and saved to file
  let loaded_after = insight::load("TestTopic", "TestName")?;

  assert!(loaded_after.embedding.is_some(), "Should have embedding after search");
  assert!(loaded_after.embedding_version.is_some(), "Should have embedding version");
  assert!(loaded_after.embedding_computed.is_some(), "Should have embedding timestamp");
  assert_eq!(loaded_after.embedding_version.unwrap(), "test-mock", "Should have mock version");

  // Verify the embedding vector is the mock embedding
  let embedding = loaded_after.embedding.unwrap();
  assert_eq!(embedding.len(), 384, "Mock embedding should have 384 dimensions");
  assert_eq!(embedding[0], 0.1, "First element should match mock embedding");

  println!("✅ Lazy embedding save functionality working correctly!");
  Ok(())
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

  // Set a custom embedding manually
  let original_embedding = embedding_client::Embedding {
    version: "original-version".to_string(),
    created_at: chrono::Utc::now(),
    embedding: vec![0.9, 0.8, 0.7], // Custom embedding different from mock
  };
  insight::set_embedding(&mut insight, original_embedding);
  insight::save(&insight)?;

  // Create search options with mock embedding client
  let mock_client = embedding_client::with_service(Box::new(MockEmbeddingService));
  let search_options = SearchOptions {
    topic: None,
    case_sensitive: false,
    overview_only: false,
    #[cfg(feature = "semantic")]
    semantic: false,
    exact: false,
    embedding_client: mock_client,
  };

  // Perform a search
  let results = search::search(&["embedding".to_string()], &search_options)?;

  // Verify we got search results
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

  println!("✅ Existing embeddings are preserved during search!");
  Ok(())
}
