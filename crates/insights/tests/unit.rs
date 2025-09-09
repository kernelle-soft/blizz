#[cfg(test)]
mod insight_tests {
  use anyhow::Result;
  use insights::server::models::insight::{self, Insight};
  use insights::server::services::search;
  use serial_test::serial;
  use std::env;
  use tempfile::TempDir;

  fn setup_temp_insights_root(test_name: &str) -> TempDir {
    let temp_dir = TempDir::new().unwrap();
    let _unique_var = format!("INSIGHTS_ROOT_{}", test_name.to_uppercase());
    env::set_var("INSIGHTS_ROOT", temp_dir.path());
    temp_dir
  }

  #[test]
  #[serial]
  fn test_insight_creation_and_file_path() {
    let insight = Insight::new(
      "test_topic".to_string(),
      "test_name".to_string(),
      "Test overview".to_string(),
      "Test details".to_string(),
    );

    assert_eq!(insight.topic, "test_topic");
    assert_eq!(insight.name, "test_name");
    assert_eq!(insight.overview, "Test overview");
    assert_eq!(insight.details, "Test details");
  }

  #[test]
  #[serial]
  fn test_get_insights_root_with_env_var() -> Result<()> {
    let _temp = setup_temp_insights_root("root_test");
    let root = insight::get_insights_root()?;
    assert!(root.to_string_lossy().contains("tmp"));
    Ok(())
  }

  #[test]
  #[serial]
  fn test_save_and_load_insight() -> Result<()> {
    let _temp = setup_temp_insights_root("save_load");

    let insight = Insight::new(
      "save_test".to_string(),
      "test_insight".to_string(),
      "Save test overview".to_string(),
      "Save test details".to_string(),
    );

    // Save the insight
    insight::save(&insight)?;

    // Load it back
    let loaded = insight::load("save_test", "test_insight")?;
    assert_eq!(loaded.overview, "Save test overview");
    assert_eq!(loaded.details, "Save test details");

    Ok(())
  }

  #[test]
  #[serial]
  fn test_save_duplicate_insight_fails() -> Result<()> {
    let _temp = setup_temp_insights_root("dup_test");

    let insight = Insight::new(
      "dup_test".to_string(),
      "duplicate".to_string(),
      "First save".to_string(),
      "Details".to_string(),
    );

    insight::save(&insight)?;

    // Try to save again - should fail
    let result = insight::save(&insight);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("already exists"));

    Ok(())
  }

  #[test]
  #[serial]
  fn test_load_nonexistent_insight() {
    let _temp = setup_temp_insights_root("load_none");

    let result = insight::load("nonexistent", "insight");
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("not found"));
  }

  #[test]
  #[serial]
  fn test_update_insight() -> Result<()> {
    let _temp = setup_temp_insights_root("update_test");

    let mut insight = Insight::new(
      "update_test".to_string(),
      "updateable".to_string(),
      "Original overview".to_string(),
      "Original details".to_string(),
    );

    insight::save(&insight)?;

    // Update just overview
    insight::update(&mut insight, Some("Updated overview"), None)?;
    assert_eq!(insight.overview, "Updated overview");
    assert_eq!(insight.details, "Original details");

    // Update just details
    insight::update(&mut insight, None, Some("Updated details"))?;
    assert_eq!(insight.overview, "Updated overview");
    assert_eq!(insight.details, "Updated details");

    // Reload to verify persistence
    let reloaded = insight::load("update_test", "updateable")?;
    assert_eq!(reloaded.overview, "Updated overview");
    assert_eq!(reloaded.details, "Updated details");

    Ok(())
  }

  #[test]
  #[serial]
  fn test_update_with_no_changes_fails() -> Result<()> {
    let _temp = setup_temp_insights_root("no_update");

    let mut insight = Insight::new(
      "no_update".to_string(),
      "test".to_string(),
      "Overview".to_string(),
      "Details".to_string(),
    );

    insight::save(&insight)?;

    let result = insight::update(&mut insight, None, None);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("At least one"));

    Ok(())
  }

  #[test]
  #[serial]
  fn test_delete_insight() -> Result<()> {
    let _temp = setup_temp_insights_root("delete_test");

    let insight = Insight::new(
      "delete_test".to_string(),
      "deletable".to_string(),
      "To be deleted".to_string(),
      "Will be gone".to_string(),
    );

    insight::save(&insight)?;

    // Verify it exists
    assert!(insight::load("delete_test", "deletable").is_ok());

    // Delete it
    insight::delete(&insight)?;

    // Verify it's gone
    assert!(insight::load("delete_test", "deletable").is_err());

    Ok(())
  }

  #[test]
  #[serial]
  fn test_delete_nonexistent_insight() {
    let _temp = setup_temp_insights_root("delete_none");

    let insight = Insight::new(
      "ghost".to_string(),
      "phantom".to_string(),
      "Never existed".to_string(),
      "Not there".to_string(),
    );

    let result = insight::delete(&insight);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("not found"));
  }

  #[test]
  #[serial]
  fn test_parse_insight_content_valid() -> Result<()> {
    let content = "---\ntopic: \"TestTopic\"\nname: \"TestName\"\noverview: \"This is the overview\\nSpanning multiple lines\"\n---\n\n# Details\nThis is the details section\nWith more content";

    let (metadata, details) = insight::parse_insight_with_metadata(content)?;
    assert_eq!(metadata.topic, "TestTopic");
    assert_eq!(metadata.name, "TestName");
    assert_eq!(metadata.overview, "This is the overview\nSpanning multiple lines");
    assert_eq!(details, "This is the details section\nWith more content");

    Ok(())
  }

  #[test]
  #[serial]
  fn test_parse_insight_content_minimal() -> Result<()> {
    let content = "---\ntopic: \"MinimalTopic\"\nname: \"MinimalName\"\noverview: Simple overview\n---\n\n# Details\n";

    let (metadata, details) = insight::parse_insight_with_metadata(content)?;
    assert_eq!(metadata.topic, "MinimalTopic");
    assert_eq!(metadata.name, "MinimalName");
    assert_eq!(metadata.overview, "Simple overview");
    assert_eq!(details, "");

    Ok(())
  }

  #[test]
  #[serial]
  fn test_parse_insight_content_legacy_no_frontmatter() {
    let content = "This is not valid format";

    let result = insight::parse_insight_with_metadata(content);
    assert!(result.is_ok());
    let (metadata, details) = result.unwrap();
    assert_eq!(metadata.overview, "This is not valid format");
    assert_eq!(details, "");
  }

  #[test]
  #[serial]
  fn test_parse_insight_content_legacy_multiline_no_frontmatter() {
    let content = "Overview line\nThis is details\nMore details";

    let result = insight::parse_insight_with_metadata(content);
    assert!(result.is_ok());
    let (metadata, details) = result.unwrap();
    assert_eq!(metadata.overview, "Overview line");
    assert_eq!(details, "This is details\nMore details");
  }

  #[test]
  #[serial]
  fn test_get_topics_empty() -> Result<()> {
    let _temp = setup_temp_insights_root("topics_empty");

    let topics = insight::get_topics()?;
    assert!(topics.is_empty());

    Ok(())
  }

  #[test]
  #[serial]
  fn test_get_topics_with_data() -> Result<()> {
    let _temp = setup_temp_insights_root("topics_data");

    // Create insights in different topics
    let insight1 =
      Insight::new("alpha".to_string(), "test1".to_string(), "O1".to_string(), "D1".to_string());
    let insight2 =
      Insight::new("beta".to_string(), "test2".to_string(), "O2".to_string(), "D2".to_string());
    let insight3 =
      Insight::new("alpha".to_string(), "test3".to_string(), "O3".to_string(), "D3".to_string());

    insight::save(&insight1)?;
    insight::save(&insight2)?;
    insight::save(&insight3)?;

    let topics = insight::get_topics()?;
    assert_eq!(topics.len(), 2);
    assert!(topics.contains(&"alpha".to_string()));
    assert!(topics.contains(&"beta".to_string()));

    Ok(())
  }

  #[test]
  #[serial]
  fn test_get_insights_all() -> Result<()> {
    let _temp = setup_temp_insights_root("insights_all");

    let insight1 = Insight::new(
      "topic1".to_string(),
      "insight1".to_string(),
      "O1".to_string(),
      "D1".to_string(),
    );
    let insight2 = Insight::new(
      "topic1".to_string(),
      "insight2".to_string(),
      "O2".to_string(),
      "D2".to_string(),
    );
    let insight3 = Insight::new(
      "topic2".to_string(),
      "insight3".to_string(),
      "O3".to_string(),
      "D3".to_string(),
    );

    insight::save(&insight1)?;
    insight::save(&insight2)?;
    insight::save(&insight3)?;

    let insights = insight::get_insights(None)?;
    assert_eq!(insights.len(), 3);

    // Should be sorted by name
    assert_eq!(insights[0].topic, "topic1");
    assert_eq!(insights[0].name, "insight1");
    assert_eq!(insights[1].topic, "topic1");
    assert_eq!(insights[1].name, "insight2");
    assert_eq!(insights[2].topic, "topic2");
    assert_eq!(insights[2].name, "insight3");

    Ok(())
  }

  #[test]
  #[serial]
  fn test_get_insights_filtered() -> Result<()> {
    let _temp = setup_temp_insights_root("insights_filtered");

    let insight1 = Insight::new(
      "filter_topic".to_string(),
      "insight1".to_string(),
      "O1".to_string(),
      "D1".to_string(),
    );
    let insight2 = Insight::new(
      "other_topic".to_string(),
      "insight2".to_string(),
      "O2".to_string(),
      "D2".to_string(),
    );

    insight::save(&insight1)?;
    insight::save(&insight2)?;

    let insights = insight::get_insights(Some("filter_topic"))?;
    assert_eq!(insights.len(), 1);
    assert_eq!(insights[0].topic, "filter_topic");
    assert_eq!(insights[0].name, "insight1");

    Ok(())
  }

  #[test]
  #[serial]
  fn test_get_insights_nonexistent_topic() -> Result<()> {
    let _temp = setup_temp_insights_root("insights_none");

    let insights = insight::get_insights(Some("nonexistent"))?;
    assert!(insights.is_empty());

    Ok(())
  }

  #[test]
  #[serial]
  fn test_search_with_highlighting() -> Result<()> {
    let _temp = setup_temp_insights_root("search_highlight");

    // Create test insights
    let insight1 = Insight::new(
      "test_topic".to_string(),
      "rust_code".to_string(),
      "This is about Rust programming language".to_string(),
      "Rust is a systems programming language that runs blazingly fast".to_string(),
    );
    let insight2 = Insight::new(
      "test_topic".to_string(),
      "other_lang".to_string(),
      "This is about Python programming".to_string(),
      "Python is great for rapid development and scripting".to_string(),
    );

    insight::save(&insight1)?;
    insight::save(&insight2)?;

    // Test search functionality by creating SearchOptions directly
    let search_options = search::SearchOptions {
      topic: None,
      case_sensitive: false,
      overview_only: false,
      exact: true, // Use exact search which doesn't require neural features
      semantic: false,
    };

    let results = search::search(&["rust".to_string()], &search_options)?;

    // Should find the rust insight
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].name, "rust_code");
    assert!(results[0].score > 0.0);

    // Test that search results can be displayed (this tests our highlighting integration)
    // The highlighting happens in the display function, so we mainly test that it doesn't crash
    search::display_results(&results, &["rust".to_string()], false);

    Ok(())
  }

  #[test]
  #[serial]
  fn test_temporal_metadata_on_creation() -> Result<()> {
    let before_creation = chrono::Utc::now();

    let insight = Insight::new(
      "temporal_test".to_string(),
      "creation_test".to_string(),
      "Testing temporal metadata on creation".to_string(),
      "This tests that new insights have proper timestamps".to_string(),
    );

    let after_creation = chrono::Utc::now();

    // Check that timestamps are set
    assert!(insight.created_at >= before_creation);
    assert!(insight.created_at <= after_creation);
    assert!(insight.last_updated >= before_creation);
    assert!(insight.last_updated <= after_creation);

    // Check that created_at and last_updated are the same on creation
    assert_eq!(insight.created_at, insight.last_updated);

    // Check that update count starts at 0
    assert_eq!(insight.update_count, 0);

    Ok(())
  }

  #[test]
  #[serial]
  fn test_temporal_metadata_on_update() -> Result<()> {
    let _temp = setup_temp_insights_root("temporal_update");

    // Create and save an initial insight
    let mut insight = Insight::new(
      "temporal_test".to_string(),
      "update_test".to_string(),
      "Original overview".to_string(),
      "Original details".to_string(),
    );

    let original_created_at = insight.created_at;
    let original_last_updated = insight.last_updated;

    insight::save(&insight)?;

    // Wait a bit to ensure timestamp difference
    std::thread::sleep(std::time::Duration::from_millis(10));

    // Update the insight
    insight::update(&mut insight, Some("Updated overview"), Some("Updated details"))?;

    // Check that created_at hasn't changed
    assert_eq!(insight.created_at, original_created_at);

    // Check that last_updated has changed
    assert!(insight.last_updated > original_last_updated);

    // Check that update_count has increased
    assert_eq!(insight.update_count, 1);

    // Update again
    std::thread::sleep(std::time::Duration::from_millis(10));
    let second_last_updated = insight.last_updated;
    insight::update(&mut insight, Some("Second update"), None)?;

    // Check that update_count increased again and last_updated changed
    assert_eq!(insight.update_count, 2);
    assert!(insight.last_updated > second_last_updated);

    Ok(())
  }

  #[test]
  #[serial]
  fn test_temporal_metadata_serialization() -> Result<()> {
    let _temp = setup_temp_insights_root("temporal_serialization");

    // Create insight with known timestamps
    let mut insight = Insight::new(
      "serialization_test".to_string(),
      "temporal_metadata".to_string(),
      "Testing temporal serialization".to_string(),
      "This tests that temporal metadata is saved to files".to_string(),
    );

    // Modify timestamps to known values for testing
    insight.created_at = chrono::DateTime::parse_from_rfc3339("2024-01-15T10:30:00Z")
      .unwrap()
      .with_timezone(&chrono::Utc);
    insight.last_updated = chrono::DateTime::parse_from_rfc3339("2024-01-20T14:45:00Z")
      .unwrap()
      .with_timezone(&chrono::Utc);
    insight.update_count = 5;

    // Save the insight
    insight::save(&insight)?;

    // Read the raw file content
    let file_path = insight::file_path(&insight)?;
    let file_content = std::fs::read_to_string(&file_path)?;

    // Verify temporal metadata is in the file
    assert!(file_content.contains("created_at: 2024-01-15T10:30:00Z"));
    assert!(file_content.contains("last_updated: 2024-01-20T14:45:00Z"));
    assert!(file_content.contains("update_count: 5"));

    // Load the insight back from file
    let loaded_insight = insight::load("serialization_test", "temporal_metadata")?;

    // Verify temporal metadata was preserved
    assert_eq!(loaded_insight.created_at, insight.created_at);
    assert_eq!(loaded_insight.last_updated, insight.last_updated);
    assert_eq!(loaded_insight.update_count, insight.update_count);

    Ok(())
  }

  #[test]
  #[serial]
  fn test_backwards_compatibility_missing_temporal_fields() -> Result<()> {
    let _temp = setup_temp_insights_root("backwards_compat");

    // Create a legacy insight file without temporal metadata
    let legacy_content = r#"---
topic: legacy_test
name: old_insight
overview: This is a legacy insight without temporal metadata
---

# Details
This insight was created before temporal metadata was added.
"#;

    // Write the legacy file directly
    let insights_root = insight::get_insights_root()?;
    let topic_dir = insights_root.join("legacy_test");
    std::fs::create_dir_all(&topic_dir)?;
    let file_path = topic_dir.join("old_insight.insight.md");
    std::fs::write(&file_path, legacy_content)?;

    // Load the legacy insight
    let loaded_insight = insight::load("legacy_test", "old_insight")?;

    // Check that temporal metadata has default values
    assert_eq!(
      loaded_insight.created_at,
      chrono::DateTime::parse_from_rfc3339("2025-05-01T00:00:00Z")
        .unwrap()
        .with_timezone(&chrono::Utc)
    );
    assert!(loaded_insight.last_updated <= chrono::Utc::now()); // Should be set to current time
    assert_eq!(loaded_insight.update_count, 0);

    // Check that other fields are correct
    assert_eq!(loaded_insight.topic, "legacy_test");
    assert_eq!(loaded_insight.name, "old_insight");
    assert_eq!(loaded_insight.overview, "This is a legacy insight without temporal metadata");
    assert_eq!(
      loaded_insight.details,
      "This insight was created before temporal metadata was added."
    );

    Ok(())
  }
}
