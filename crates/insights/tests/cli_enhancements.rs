use anyhow::Result;
use insights::commands::*;
use insights::insight;
use serial_test::serial;
use std::env;
use tempfile::TempDir;

#[cfg(test)]
mod cli_enhancement_tests {
  use super::*;

  fn setup_temp_insights_root(_test_name: &str) -> TempDir {
    let temp_dir = TempDir::new().unwrap();
    env::set_var("INSIGHTS_ROOT", temp_dir.path());
    temp_dir
  }

  #[test]
  #[serial]
  fn test_basic_command_flow() -> Result<()> {
    let _temp = setup_temp_insights_root("basic_flow");

    // Test add -> get -> list flow
    add_insight_with_client(
      "workflow",
      "basic",
      "Basic workflow test",
      "Testing the basic command flow",
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

    // Create multiple insights across topics
    add_insight_with_client("ai", "basics", "AI Basics", "Introduction to AI")?;
    add_insight_with_client("ai", "advanced", "Advanced AI", "Deep AI concepts")?;
    add_insight_with_client("rust", "ownership", "Ownership", "Rust ownership model")?;
    add_insight_with_client("rust", "borrowing", "Borrowing", "Rust borrowing rules")?;

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

    // Create initial insight
    add_insight_with_client("updates", "test", "Original overview", "Original details")?;

    // Test various update scenarios
    update_insight_with_client("updates", "test", Some("Updated overview"), None)?;
    update_insight_with_client("updates", "test", None, Some("Updated details"))?;
    update_insight_with_client("updates", "test", Some("Final overview"), Some("Final details"))?;

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

    // Create insights to delete
    add_insight_with_client("deleteme", "first", "First insight", "First details")?;
    add_insight_with_client("deleteme", "second", "Second insight", "Second details")?;
    add_insight_with_client("keepme", "safe", "Safe insight", "Safe details")?;

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

    // Start with empty topics
    list_topics()?;

    // Add insights to create topics
    add_insight_with_client("topic1", "insight1", "Overview 1", "Details 1")?;
    add_insight_with_client("topic2", "insight2", "Overview 2", "Details 2")?;
    add_insight_with_client("topic3", "insight3", "Overview 3", "Details 3")?;

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

    // Test getting non-existent insight
    let result = get_insight("nonexistent", "insight", false);
    assert!(result.is_err());

    // Test updating non-existent insight
    let result = update_insight_with_client("nonexistent", "insight", Some("overview"), None);
    assert!(result.is_err());

    // Test deleting non-existent insight
    let result = delete_insight("nonexistent", "insight", true);
    assert!(result.is_err());

    // Test duplicate addition
    add_insight_with_client("errors", "duplicate", "Original", "Original")?;
    let result = add_insight_with_client("errors", "duplicate", "Duplicate", "Duplicate");
    assert!(result.is_err());

    Ok(())
  }

  #[test]
  #[serial]
  fn test_content_variations() -> Result<()> {
    let _temp = setup_temp_insights_root("content_variations");

    // Test with empty content
    add_insight_with_client("empty", "test1", "", "")?;
    add_insight_with_client("empty", "test2", "Overview", "")?;
    add_insight_with_client("empty", "test3", "", "Details")?;

    // Test with special characters
    add_insight_with_client(
      "special",
      "chars",
      "Overview with émojis 🚀 and symbols: @#$%",
      "Details with\nmultiple\nlines\nand unicode: ñáéíóú",
    )?;

    // Test with long content
    let long_overview = "A".repeat(1000);
    let long_details = "B".repeat(5000);
    add_insight_with_client("long", "content", &long_overview, &long_details)?;

    // Verify all can be retrieved
    list_insights(None, false)?;

    Ok(())
  }

  #[test]
  #[serial]
  fn test_cli_output_modes() -> Result<()> {
    let _temp = setup_temp_insights_root("output_modes");

    // Create test data
    add_insight_with_client("output", "test1", "Short overview", "Short details")?;
    add_insight_with_client("output", "test2", "Another overview", "More details here")?;

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
