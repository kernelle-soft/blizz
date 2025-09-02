#[cfg(test)]
mod index_command_tests {

  use anyhow::Result;
  use insights::cli::commands::*;
  use insights::insight::{self};
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
  fn test_index_insights_empty_database() -> Result<()> {
    let _temp = setup_temp_insights_root("index_empty");

    // Should handle empty database gracefully
    index_insights(false)?;

    Ok(())
  }

  #[test]
  #[serial]
  fn test_index_insights_multiple_topics() -> Result<()> {
    let _temp = setup_temp_insights_root("index_multiple_topics");

    // Create insights across multiple topics
    add_insight_with_client(
      "ai",
      "neural_networks",
      "About neural networks",
      "Deep learning details",
    )?;
    add_insight_with_client("ai", "machine_learning", "About ML", "ML algorithms")?;
    add_insight_with_client("databases", "postgresql", "About PostgreSQL", "Database management")?;
    add_insight_with_client("databases", "redis", "About Redis", "In-memory store")?;
    add_insight_with_client("rust", "ownership", "About ownership", "Memory management")?;

    // Index all insights
    index_insights(false)?;

    Ok(())
  }

  #[test]
  #[serial]
  fn test_index_insights_preserves_existing_insights() -> Result<()> {
    let _temp = setup_temp_insights_root("index_preserves");

    // Create an insight
    add_insight_with_client("preserve", "test", "Original overview", "Original details")?;

    // Verify content before indexing
    let before = insight::load("preserve", "test")?;
    assert_eq!(before.overview, "Original overview");
    assert_eq!(before.details, "Original details");

    // Run indexing
    index_insights(true)?;

    // Verify content is preserved after indexing
    let after = insight::load("preserve", "test")?;
    assert_eq!(after.overview, "Original overview");
    assert_eq!(after.details, "Original details");

    Ok(())
  }

  #[test]
  #[serial]
  fn test_index_insights_handles_unicode_content() -> Result<()> {
    let _temp = setup_temp_insights_root("index_unicode");

    // Create insights with unicode content
    add_insight_with_client(
      "unicode",
      "test",
      "Overview with émojis 🚀 and unicode: ñáéíóú",
      "Details with Chinese: 你好世界, Arabic: مرحبا, Russian: Привет",
    )?;

    // Index should handle unicode content without issues
    index_insights(false)?;

    // Verify the insight still exists and has correct content
    let insight = insight::load("unicode", "test")?;
    assert!(insight.overview.contains("émojis 🚀"));
    assert!(insight.details.contains("你好世界"));

    Ok(())
  }

  #[test]
  #[serial]
  fn test_index_insights_verify_content_preserved() -> Result<()> {
    let _temp = setup_temp_insights_root("index_content_preserved");

    let original_overview = "This is a test overview with specific content";
    let original_details =
      "These are test details with\nmultiple lines\nand special characters: @#$%^&*()";

    // Create insight with specific content
    add_insight_with_client("content", "preservation", original_overview, original_details)?;

    // Index the insights
    index_insights(true)?;

    // Verify exact content preservation
    let preserved = insight::load("content", "preservation")?;
    assert_eq!(preserved.overview, original_overview);
    assert_eq!(preserved.details, original_details);

    Ok(())
  }
}
