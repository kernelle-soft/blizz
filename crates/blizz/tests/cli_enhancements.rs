use anyhow::Result;
use blizz::commands::*;
use blizz::embedding_client::MockEmbeddingService;
use blizz::insight::{self, Insight};
use serial_test::serial;
use std::env;
use tempfile::TempDir;

#[cfg(test)]
mod cli_enhancement_tests {
  use super::*;

  fn setup_temp_insights_root(_test_name: &str) -> TempDir {
    let temp_dir = TempDir::new().unwrap();
    env::set_var("BLIZZ_INSIGHTS_ROOT", temp_dir.path());
    temp_dir
  }

  #[test]
  #[serial]
  fn test_basic_command_flow() -> Result<()> {
    let _temp = setup_temp_insights_root("basic_flow");
    let embedding_service = MockEmbeddingService;

    // Test the complete command flow
    add_insight_with_service("cli_topic", "cli_insight", "CLI Overview", "CLI Details", &embedding_service)?;

    get_insight("cli_topic", "cli_insight", false)?;
    get_insight("cli_topic", "cli_insight", true)?;

    list_insights(None, false)?;
    list_insights(Some("cli_topic"), true)?;

    update_insight_with_service("cli_topic", "cli_insight", Some("Updated Overview"), None, &embedding_service)?;

    let loaded = insight::load("cli_topic", "cli_insight")?;
    assert_eq!(loaded.overview, "Updated Overview");

    Ok(())
  }

  #[test]
  #[serial]
  fn test_multiple_insights_workflow() -> Result<()> {
    let _temp = setup_temp_insights_root("multi_workflow");
    let embedding_service = MockEmbeddingService;

    // Create multiple insights
    add_insight_with_service("topic1", "insight1", "First insight", "Details 1", &embedding_service)?;
    add_insight_with_service("topic1", "insight2", "Second insight", "Details 2", &embedding_service)?;
    add_insight_with_service("topic2", "insight3", "Third insight", "Details 3", &embedding_service)?;

    // Test listing functionality
    list_insights(None, false)?;
    list_insights(Some("topic1"), false)?;
    list_insights(Some("topic2"), true)?;

    // Test getting individual insights
    get_insight("topic1", "insight1", false)?;
    get_insight("topic2", "insight3", true)?;

    Ok(())
  }

  #[test]
  #[serial]
  fn test_update_workflow() -> Result<()> {
    let _temp = setup_temp_insights_root("update_workflow");
    let embedding_service = MockEmbeddingService;

    add_insight_with_service("update_topic", "test_insight", "Original overview", "Original details", &embedding_service)?;

    // Test overview-only update
    update_insight_with_service("update_topic", "test_insight", Some("New overview"), None, &embedding_service)?;
    let loaded = insight::load("update_topic", "test_insight")?;
    assert_eq!(loaded.overview, "New overview");
    assert_eq!(loaded.details, "Original details");

    // Test details-only update
    update_insight_with_service("update_topic", "test_insight", None, Some("New details"), &embedding_service)?;
    let loaded = insight::load("update_topic", "test_insight")?;
    assert_eq!(loaded.overview, "New overview");
    assert_eq!(loaded.details, "New details");

    // Test both overview and details update
    update_insight_with_service("update_topic", "test_insight", Some("Final overview"), Some("Final details"), &embedding_service)?;
    let loaded = insight::load("update_topic", "test_insight")?;
    assert_eq!(loaded.overview, "Final overview");
    assert_eq!(loaded.details, "Final details");

    Ok(())
  }

  #[test]
  #[serial]
  fn test_delete_workflow() -> Result<()> {
    let _temp = setup_temp_insights_root("delete_workflow");
    let embedding_service = MockEmbeddingService;

    add_insight_with_service("delete_topic", "temp_insight", "Temporary", "To be deleted", &embedding_service)?;

    // Verify it exists
    let _loaded = insight::load("delete_topic", "temp_insight")?;

    // Delete it
    delete_insight("delete_topic", "temp_insight", true)?;

    // Verify it's gone
    let result = insight::load("delete_topic", "temp_insight");
    assert!(result.is_err());

    Ok(())
  }

  #[test]
  #[serial]
  fn test_topics_management() -> Result<()> {
    let _temp = setup_temp_insights_root("topics_mgmt");
    let embedding_service = MockEmbeddingService;

    // Start with empty topics
    list_topics()?;

    // Add insights in different topics
    add_insight_with_service("topic_alpha", "insight1", "Alpha content", "Details", &embedding_service)?;
    add_insight_with_service("topic_beta", "insight2", "Beta content", "Details", &embedding_service)?;
    add_insight_with_service("topic_gamma", "insight3", "Gamma content", "Details", &embedding_service)?;

    // List topics
    list_topics()?;

    // List insights by topic
    list_insights(Some("topic_alpha"), false)?;
    list_insights(Some("topic_beta"), true)?;

    Ok(())
  }

  #[test]
  #[serial]
  fn test_error_handling() -> Result<()> {
    let _temp = setup_temp_insights_root("error_handling");
    let embedding_service = MockEmbeddingService;

    // Test getting non-existent insight
    let result = get_insight("nonexistent", "insight", false);
    assert!(result.is_err());

    // Test updating non-existent insight
    let result = update_insight_with_service("nonexistent", "insight", Some("overview"), None, &embedding_service);
    assert!(result.is_err());

    // Test deleting non-existent insight
    let result = delete_insight("nonexistent", "insight", true);
    assert!(result.is_err());

    // Test duplicate creation
    add_insight_with_service("dup_topic", "dup_insight", "Original", "Details", &embedding_service)?;
    let result = add_insight_with_service("dup_topic", "dup_insight", "Duplicate", "Details", &embedding_service);
    assert!(result.is_err());

    Ok(())
  }

  #[test]
  #[serial]
  fn test_content_variations() -> Result<()> {
    let _temp = setup_temp_insights_root("content_variations");
    let embedding_service = MockEmbeddingService;

    // Test various content types
    add_insight_with_service("content", "multiline", "Line 1\nLine 2\nLine 3", "Multi\nLine\nDetails", &embedding_service)?;
    add_insight_with_service("content", "unicode", "Unicode: émojis 🚀", "中文 text", &embedding_service)?;
    add_insight_with_service("content", "special_chars", "Special: @#$%", "More: &*()", &embedding_service)?;

    // Verify they can be retrieved
    get_insight("content", "multiline", false)?;
    get_insight("content", "unicode", true)?;
    get_insight("content", "special_chars", false)?;

    Ok(())
  }

  #[test]
  #[serial]
  fn test_cli_output_modes() -> Result<()> {
    let _temp = setup_temp_insights_root("output_modes");
    let embedding_service = MockEmbeddingService;

    add_insight_with_service("output", "test1", "Short overview", "Longer details section with more content", &embedding_service)?;
    add_insight_with_service("output", "test2", "Another overview", "More details here", &embedding_service)?;

    // Test different output modes
    get_insight("output", "test1", false)?; // Full content
    get_insight("output", "test1", true)?;  // Overview only

    list_insights(Some("output"), false)?; // Brief listing
    list_insights(Some("output"), true)?;  // Verbose listing

    Ok(())
  }

  // Note: Search functionality tests have been removed as they require complex CLI argument parsing
  // and are better suited for higher-level integration tests at the CLI binary level
}
