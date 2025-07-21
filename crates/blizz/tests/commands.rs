use anyhow::Result;
use blizz::commands::*;
use blizz::embedding_client;
use blizz::insight::{self, Insight};
use serial_test::serial;
use std::env;
use tempfile::TempDir;

#[cfg(test)]
mod command_tests {
  use super::*;

  fn setup_temp_insights_root(_test_name: &str) -> TempDir {
    let temp_dir = TempDir::new().unwrap();
    env::set_var("BLIZZ_INSIGHTS_ROOT", temp_dir.path());
    temp_dir
  }

  #[test]
  #[serial]
  fn test_add_insight_success() -> Result<()> {
    let _temp = setup_temp_insights_root("add_insight_success");
    let client = embedding_client::with_mock();

    add_insight_with_client("test_topic", "test_name", "Test overview", "Test details", &client)?;

    // Verify the insight was created
    let loaded = insight::load("test_topic", "test_name")?;
    assert_eq!(loaded.topic, "test_topic");
    assert_eq!(loaded.name, "test_name");
    assert_eq!(loaded.overview, "Test overview");
    assert_eq!(loaded.details, "Test details");

    Ok(())
  }

  #[test]
  #[serial]
  fn test_add_insight_empty_fields() -> Result<()> {
    let _temp = setup_temp_insights_root("add_insight_empty");
    let client = embedding_client::with_mock();

    // Empty fields should be allowed (creating unusual but valid insights)
    add_insight_with_client("", "", "", "", &client)?;
    add_insight_with_client("topic", "name", "", "details", &client)?;
    add_insight_with_client("topic2", "name2", "overview", "", &client)?;

    Ok(())
  }

  #[test]
  #[serial]
  fn test_add_duplicate_insight_fails() -> Result<()> {
    let _temp = setup_temp_insights_root("add_duplicate");
    let client = embedding_client::with_mock();

    add_insight_with_client("test_topic", "test_name", "Overview", "Details", &client)?;

    // Adding the same insight again should fail
    let result = add_insight_with_client("test_topic", "test_name", "Different overview", "Different details", &client);
    assert!(result.is_err());

    Ok(())
  }

  #[test]
  #[serial]
  fn test_get_insight_full() -> Result<()> {
    let _temp = setup_temp_insights_root("get_insight_full");
    let client = embedding_client::with_mock();

    add_insight_with_client("test_topic", "test_name", "Test overview", "Test details", &client)?;

    // Should not panic and should run successfully
    get_insight("test_topic", "test_name", false)?;

    Ok(())
  }

  #[test]
  #[serial]
  fn test_get_insight_overview_only() -> Result<()> {
    let _temp = setup_temp_insights_root("get_insight_overview");
    let client = embedding_client::with_mock();

    add_insight_with_client("test_topic", "test_name", "Test overview", "Test details", &client)?;

    // Should not panic and should run successfully
    get_insight("test_topic", "test_name", true)?;

    Ok(())
  }

  #[test]
  #[serial]
  fn test_get_nonexistent_insight() -> Result<()> {
    let _temp = setup_temp_insights_root("get_nonexistent");

    let result = get_insight("nonexistent_topic", "nonexistent_name", false);
    assert!(result.is_err());

    Ok(())
  }

  #[test]
  #[serial]
  fn test_get_insight_with_special_characters() -> Result<()> {
    let _temp = setup_temp_insights_root("get_special_chars");
    let client = embedding_client::with_mock();

    add_insight_with_client(
      "special_topic", 
      "special_name", 
      "Overview with émojis 🚀", 
      "Details with special chars: @#$%^&*()",
      &client
    )?;

    get_insight("special_topic", "special_name", false)?;
    get_insight("special_topic", "special_name", true)?;

    Ok(())
  }

  #[test]
  #[serial]
  fn test_list_insights_empty() -> Result<()> {
    let _temp = setup_temp_insights_root("list_empty");

    list_insights(None, false)?;
    list_insights(Some("nonexistent_topic"), true)?;

    Ok(())
  }

  #[test]
  #[serial]
  fn test_list_insights_with_data() -> Result<()> {
    let _temp = setup_temp_insights_root("list_with_data");
    let client = embedding_client::with_mock();

    add_insight_with_client("topic1", "insight1", "Overview 1", "Details 1", &client)?;
    add_insight_with_client("topic1", "insight2", "Overview 2", "Details 2", &client)?;
    add_insight_with_client("topic2", "insight3", "Overview 3", "Details 3", &client)?;

    list_insights(None, false)?;
    list_insights(None, true)?;
    list_insights(Some("topic1"), false)?;
    list_insights(Some("topic1"), true)?;

    Ok(())
  }

  #[test]
  #[serial]
  fn test_list_insights_nonexistent_topic() -> Result<()> {
    let _temp = setup_temp_insights_root("list_nonexistent_topic");
    let client = embedding_client::with_mock();

    add_insight_with_client("real_topic", "insight1", "Overview", "Details", &client)?;

    list_insights(Some("nonexistent_topic"), false)?;

    Ok(())
  }

  #[test]
  #[serial]
  fn test_list_topics_empty() -> Result<()> {
    let _temp = setup_temp_insights_root("list_topics_empty");

    list_topics()?;

    Ok(())
  }

  #[test]
  #[serial]
  fn test_list_topics_with_data() -> Result<()> {
    let _temp = setup_temp_insights_root("list_topics_with_data");
    let client = embedding_client::with_mock();

    add_insight_with_client("topic1", "insight1", "Overview 1", "Details 1", &client)?;
    add_insight_with_client("topic2", "insight2", "Overview 2", "Details 2", &client)?;
    add_insight_with_client("topic3", "insight3", "Overview 3", "Details 3", &client)?;

    list_topics()?;

    Ok(())
  }

  #[test]
  #[serial]
  fn test_update_insight_overview() -> Result<()> {
    let _temp = setup_temp_insights_root("update_overview");
    let client = embedding_client::with_mock();

    add_insight_with_client("test_topic", "test_name", "Original overview", "Original details", &client)?;

    update_insight_with_client("test_topic", "test_name", Some("Updated overview"), None, &client)?;

    let loaded = insight::load("test_topic", "test_name")?;
    assert_eq!(loaded.overview, "Updated overview");
    assert_eq!(loaded.details, "Original details");

    Ok(())
  }

  #[test]
  #[serial]
  fn test_update_insight_details() -> Result<()> {
    let _temp = setup_temp_insights_root("update_details");
    let client = embedding_client::with_mock();

    add_insight_with_client("test_topic", "test_name", "Original overview", "Original details", &client)?;

    update_insight_with_client("test_topic", "test_name", None, Some("Updated details"), &client)?;

    let loaded = insight::load("test_topic", "test_name")?;
    assert_eq!(loaded.overview, "Original overview");
    assert_eq!(loaded.details, "Updated details");

    Ok(())
  }

  #[test]
  #[serial]
  fn test_update_insight_both() -> Result<()> {
    let _temp = setup_temp_insights_root("update_both");
    let client = embedding_client::with_mock();

    add_insight_with_client("test_topic", "test_name", "Original overview", "Original details", &client)?;

    update_insight_with_client(
      "test_topic", 
      "test_name", 
      Some("Updated overview"), 
      Some("Updated details"),
      &client
    )?;

    let loaded = insight::load("test_topic", "test_name")?;
    assert_eq!(loaded.overview, "Updated overview");
    assert_eq!(loaded.details, "Updated details");

    Ok(())
  }

  #[test]
  #[serial]
  fn test_update_insight_no_changes() -> Result<()> {
    let _temp = setup_temp_insights_root("update_no_changes");
    let client = embedding_client::with_mock();

    add_insight_with_client("test_topic", "test_name", "Original overview", "Original details", &client)?;

    let result = update_insight_with_client("test_topic", "test_name", None, None, &client);
    assert!(result.is_err());

    Ok(())
  }

  #[test]
  #[serial]
  fn test_update_nonexistent_insight() -> Result<()> {
    let _temp = setup_temp_insights_root("update_nonexistent");
    let client = embedding_client::with_mock();

    let result = update_insight_with_client(
      "nonexistent_topic", 
      "nonexistent_name", 
      Some("New overview"), 
      None,
      &client
    );
    assert!(result.is_err());

    Ok(())
  }

  #[test]
  #[serial]
  fn test_delete_insight_force() -> Result<()> {
    let _temp = setup_temp_insights_root("delete_force");
    let client = embedding_client::with_mock();

    add_insight_with_client("test_topic", "test_name", "Overview", "Details", &client)?;

    delete_insight("test_topic", "test_name", true)?;

    // Verify the insight is gone
    let result = insight::load("test_topic", "test_name");
    assert!(result.is_err());

    Ok(())
  }

  #[test]
  #[serial]
  fn test_delete_nonexistent_insight() -> Result<()> {
    let _temp = setup_temp_insights_root("delete_nonexistent");

    let result = delete_insight("nonexistent_topic", "nonexistent_name", true);
    assert!(result.is_err());

    Ok(())
  }
}
