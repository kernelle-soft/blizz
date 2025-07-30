#[cfg(feature = "neural")]
use anyhow::Result;
#[cfg(feature = "neural")]
use blizz::commands::*;
#[cfg(feature = "neural")]
use blizz::embedding_client;
#[cfg(feature = "neural")]
use blizz::embedding_client::MockEmbeddingService;
#[cfg(feature = "neural")]
use blizz::insight;
#[cfg(feature = "neural")]
use serial_test::serial;
#[cfg(feature = "neural")]
use std::env;
#[cfg(feature = "neural")]
use tempfile::TempDir;

#[cfg(test)]
#[cfg(feature = "neural")]
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
    let client = embedding_client::with_service(Box::new(MockEmbeddingService));

    // Test add -> get -> list flow
    add_insight_with_client(
      "workflow",
      "basic",
      "Basic workflow test",
      "Testing the basic command flow",
      &client,
    )?;

    get_insight("workflow", "basic", false)?;
    get_insight("workflow", "basic", true)?;

    list_insights(Some("workflow"), false)?;
    list_insights(None, false)?;
    list_topics()?;

    Ok(())
  }

  #[test]
  #[serial]
  fn test_multiple_insights_workflow() -> Result<()> {
    let _temp = setup_temp_insights_root("multiple_insights");
    let client = embedding_client::with_service(Box::new(MockEmbeddingService));

    // Create multiple insights across topics
    add_insight_with_client("ai", "basics", "AI Basics", "Introduction to AI", &client)?;
    add_insight_with_client("ai", "advanced", "Advanced AI", "Deep AI concepts", &client)?;
    add_insight_with_client("rust", "ownership", "Ownership", "Rust ownership model", &client)?;
    add_insight_with_client("rust", "borrowing", "Borrowing", "Rust borrowing rules", &client)?;

    // Test listing and filtering
    list_insights(None, false)?;
    list_insights(Some("ai"), false)?;
    list_insights(Some("rust"), false)?;
    list_topics()?;

    Ok(())
  }

  #[test]
  #[serial]
  fn test_update_workflow() -> Result<()> {
    let _temp = setup_temp_insights_root("update_workflow");
    let client = embedding_client::with_service(Box::new(MockEmbeddingService));

    // Create initial insight
    add_insight_with_client("updates", "test", "Original overview", "Original details", &client)?;

    // Test various update scenarios
    update_insight_with_client("updates", "test", Some("Updated overview"), None, &client)?;
    update_insight_with_client("updates", "test", None, Some("Updated details"), &client)?;
    update_insight_with_client(
      "updates",
      "test",
      Some("Final overview"),
      Some("Final details"),
      &client,
    )?;

    // Verify final state
    let final_insight = insight::load("updates", "test")?;
    assert_eq!(final_insight.overview, "Final overview");
    assert_eq!(final_insight.details, "Final details");

    Ok(())
  }

  #[test]
  #[serial]
  fn test_delete_workflow() -> Result<()> {
    let _temp = setup_temp_insights_root("delete_workflow");
    let client = embedding_client::with_service(Box::new(MockEmbeddingService));

    // Create insights to delete
    add_insight_with_client("deleteme", "first", "First insight", "First details", &client)?;
    add_insight_with_client("deleteme", "second", "Second insight", "Second details", &client)?;
    add_insight_with_client("keepme", "safe", "Safe insight", "Safe details", &client)?;

    // Delete one insight
    delete_insight("deleteme", "first", true)?;

    // Verify deletion
    let result = insight::load("deleteme", "first");
    assert!(result.is_err());

    // Verify others still exist
    let second = insight::load("deleteme", "second")?;
    assert_eq!(second.name, "second");

    let safe = insight::load("keepme", "safe")?;
    assert_eq!(safe.name, "safe");

    Ok(())
  }

  #[test]
  #[serial]
  fn test_topics_management() -> Result<()> {
    let _temp = setup_temp_insights_root("topics_management");
    let client = embedding_client::with_service(Box::new(MockEmbeddingService));

    // Start with empty topics
    list_topics()?;

    // Add insights to create topics
    add_insight_with_client("topic1", "insight1", "Overview 1", "Details 1", &client)?;
    add_insight_with_client("topic2", "insight2", "Overview 2", "Details 2", &client)?;
    add_insight_with_client("topic3", "insight3", "Overview 3", "Details 3", &client)?;

    // Test topic listing
    list_topics()?;

    // Delete all insights from a topic
    delete_insight("topic2", "insight2", true)?;

    // Topic should still appear in directory structure
    list_topics()?;

    Ok(())
  }

  #[test]
  #[serial]
  fn test_error_handling() -> Result<()> {
    let _temp = setup_temp_insights_root("error_handling");
    let client = embedding_client::with_service(Box::new(MockEmbeddingService));

    // Test getting non-existent insight
    let result = get_insight("nonexistent", "insight", false);
    assert!(result.is_err());

    // Test updating non-existent insight
    let result =
      update_insight_with_client("nonexistent", "insight", Some("overview"), None, &client);
    assert!(result.is_err());

    // Test deleting non-existent insight
    let result = delete_insight("nonexistent", "insight", true);
    assert!(result.is_err());

    // Test duplicate addition
    add_insight_with_client("errors", "duplicate", "Original", "Original", &client)?;
    let result = add_insight_with_client("errors", "duplicate", "Duplicate", "Duplicate", &client);
    assert!(result.is_err());

    Ok(())
  }

  #[test]
  #[serial]
  fn test_content_variations() -> Result<()> {
    let _temp = setup_temp_insights_root("content_variations");
    let client = embedding_client::with_service(Box::new(MockEmbeddingService));

    // Test with empty content
    add_insight_with_client("empty", "test1", "", "", &client)?;
    add_insight_with_client("empty", "test2", "Overview", "", &client)?;
    add_insight_with_client("empty", "test3", "", "Details", &client)?;

    // Test with special characters
    add_insight_with_client(
      "special",
      "chars",
      "Overview with émojis 🚀 and symbols: @#$%",
      "Details with\nmultiple\nlines\nand unicode: ñáéíóú",
      &client,
    )?;

    // Test with long content
    let long_overview = "A".repeat(1000);
    let long_details = "B".repeat(5000);
    add_insight_with_client("long", "content", &long_overview, &long_details, &client)?;

    // Verify all can be retrieved
    list_insights(None, false)?;

    Ok(())
  }

  #[test]
  #[serial]
  fn test_cli_output_modes() -> Result<()> {
    let _temp = setup_temp_insights_root("output_modes");
    let client = embedding_client::with_service(Box::new(MockEmbeddingService));

    // Create test data
    add_insight_with_client("output", "test1", "Short overview", "Short details", &client)?;
    add_insight_with_client("output", "test2", "Another overview", "More details here", &client)?;

    // Test different output modes
    get_insight("output", "test1", false)?; // Full content
    get_insight("output", "test1", true)?; // Overview only

    list_insights(Some("output"), false)?; // Normal list
    list_insights(Some("output"), true)?; // Verbose list

    list_insights(None, false)?; // All insights normal
    list_insights(None, true)?; // All insights verbose

    Ok(())
  }
}
