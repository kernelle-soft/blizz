use anyhow::Result;
use blizz::insight::*;
use serial_test::serial;
use std::env;
use tempfile::TempDir;

#[cfg(test)]
mod insight_tests {
  use super::*;

  fn setup_temp_insights_root(test_name: &str) -> TempDir {
    let temp_dir = TempDir::new().unwrap();
    let _unique_var = format!("BLIZZ_INSIGHTS_ROOT_{}", test_name.to_uppercase());
    env::set_var("BLIZZ_INSIGHTS_ROOT", temp_dir.path());
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
    let root = get_insights_root()?;
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
    insight.save()?;

    // Load it back
    let loaded = Insight::load("save_test", "test_insight")?;
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

    insight.save()?;

    // Try to save again - should fail
    let result = insight.save();
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("already exists"));

    Ok(())
  }

  #[test]
  #[serial]
  fn test_load_nonexistent_insight() {
    let _temp = setup_temp_insights_root("load_none");

    let result = Insight::load("nonexistent", "insight");
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

    insight.save()?;

    // Update just overview
    insight.update(Some("Updated overview"), None)?;
    assert_eq!(insight.overview, "Updated overview");
    assert_eq!(insight.details, "Original details");

    // Update just details
    insight.update(None, Some("Updated details"))?;
    assert_eq!(insight.overview, "Updated overview");
    assert_eq!(insight.details, "Updated details");

    // Reload to verify persistence
    let reloaded = Insight::load("update_test", "updateable")?;
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

    insight.save()?;

    let result = insight.update(None, None);
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

    insight.save()?;

    // Verify it exists
    assert!(Insight::load("delete_test", "deletable").is_ok());

    // Delete it
    insight.delete()?;

    // Verify it's gone
    assert!(Insight::load("delete_test", "deletable").is_err());

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

    let result = insight.delete();
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("not found"));
  }

  #[test]
  #[serial]
  fn test_parse_insight_content_valid() -> Result<()> {
    let content = "---\nThis is the overview\nSpanning multiple lines\n---\n\nThis is the details section\nWith more content";

    let (overview, details) = parse_insight_content(content)?;
    assert_eq!(overview, "This is the overview\nSpanning multiple lines");
    assert_eq!(details, "This is the details section\nWith more content");

    Ok(())
  }

  #[test]
  #[serial]
  fn test_parse_insight_content_minimal() -> Result<()> {
    let content = "---\nSimple overview\n---\n\n";

    let (overview, details) = parse_insight_content(content)?;
    assert_eq!(overview, "Simple overview");
    assert_eq!(details, "");

    Ok(())
  }

  #[test]
  #[serial]
  fn test_parse_insight_content_invalid() {
    let content = "This is not valid format";

    let result = parse_insight_content(content);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Invalid insight format"));
  }

  #[test]
  #[serial]
  fn test_get_topics_empty() -> Result<()> {
    let _temp = setup_temp_insights_root("topics_empty");

    let topics = get_topics()?;
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

    insight1.save()?;
    insight2.save()?;
    insight3.save()?;

    let topics = get_topics()?;
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

    insight1.save()?;
    insight2.save()?;
    insight3.save()?;

    let insights = get_insights(None)?;
    assert_eq!(insights.len(), 3);

    // Should be sorted
    assert_eq!(insights[0], ("topic1".to_string(), "insight1".to_string()));
    assert_eq!(insights[1], ("topic1".to_string(), "insight2".to_string()));
    assert_eq!(insights[2], ("topic2".to_string(), "insight3".to_string()));

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

    insight1.save()?;
    insight2.save()?;

    let insights = get_insights(Some("filter_topic"))?;
    assert_eq!(insights.len(), 1);
    assert_eq!(insights[0], ("filter_topic".to_string(), "insight1".to_string()));

    Ok(())
  }

  #[test]
  #[serial]
  fn test_get_insights_nonexistent_topic() -> Result<()> {
    let _temp = setup_temp_insights_root("insights_none");

    let insights = get_insights(Some("nonexistent"))?;
    assert!(insights.is_empty());

    Ok(())
  }

  #[test]
  #[serial]
  fn test_empty_directory_cleanup_on_delete() -> Result<()> {
    let _temp = setup_temp_insights_root("cleanup");

    let insight = Insight::new(
      "cleanup_topic".to_string(),
      "only_insight".to_string(),
      "Lonely insight".to_string(),
      "Will clean up directory".to_string(),
    );

    insight.save()?;

    // Verify topic directory exists
    let topics = get_topics()?;
    assert!(topics.contains(&"cleanup_topic".to_string()));

    // Delete the insight
    insight.delete()?;

    // Verify topic directory is removed
    let topics_after = get_topics()?;
    assert!(!topics_after.contains(&"cleanup_topic".to_string()));

    Ok(())
  }
}
