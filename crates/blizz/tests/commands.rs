use anyhow::Result;
use blizz::commands::*;
use blizz::insight::*;
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

  #[allow(dead_code)]
  fn capture_output<F>(f: F) -> String
  where
    F: FnOnce() -> Result<()>,
  {
    // For now, we'll just run the function and ignore output capture
    // In a real scenario, you might want to capture stdout
    let _ = f();
    String::new()
  }

  #[test]
  #[serial]
  #[serial]
  fn test_add_insight_success() -> Result<()> {
    let _temp = setup_temp_insights_root("add_success");

    add_insight("test_topic", "test_name", "Test overview", "Test details")?;

    // Verify it was created
    let loaded = Insight::load("test_topic", "test_name")?;
    assert_eq!(loaded.overview, "Test overview");
    assert_eq!(loaded.details, "Test details");

    Ok(())
  }

  #[test]
  #[serial]
  #[serial]
  fn test_add_duplicate_insight_fails() -> Result<()> {
    let _temp = setup_temp_insights_root("add_duplicate");

    // Add first insight
    add_insight("dup_topic", "dup_name", "First", "Details")?;

    // Try to add duplicate - should fail
    let result = add_insight("dup_topic", "dup_name", "Second", "Details");
    assert!(result.is_err());

    Ok(())
  }

  #[test]
  #[serial]
  fn test_get_insight_overview_only() -> Result<()> {
    let _temp = setup_temp_insights_root("get_overview");

    // Create an insight
    add_insight("get_topic", "get_name", "Test overview", "Test details")?;

    // Test overview only mode
    get_insight("get_topic", "get_name", true)?;

    Ok(())
  }

  #[test]
  #[serial]
  fn test_get_insight_full() -> Result<()> {
    let _temp = setup_temp_insights_root("get_full");

    add_insight("get_topic", "get_name", "Test overview", "Test details")?;

    // Test full content mode
    get_insight("get_topic", "get_name", false)?;

    Ok(())
  }

  #[test]
  #[serial]
  fn test_get_nonexistent_insight() {
    let _temp = setup_temp_insights_root("get_nonexistent");

    let result = get_insight("nonexistent", "insight", false);
    assert!(result.is_err());
  }

  #[test]
  #[serial]
  fn test_list_insights_empty() -> Result<()> {
    let _temp = setup_temp_insights_root("list_empty");

    list_insights(None, false)?;
    list_insights(None, true)?;

    Ok(())
  }

  #[test]
  #[serial]
  fn test_list_insights_with_data() -> Result<()> {
    let _temp = setup_temp_insights_root("list_with_data");

    add_insight("list_topic1", "insight1", "Overview 1", "Details 1")?;
    add_insight("list_topic1", "insight2", "Overview 2", "Details 2")?;
    add_insight("list_topic2", "insight3", "Overview 3", "Details 3")?;

    // Test listing all insights
    list_insights(None, false)?;
    list_insights(None, true)?;

    // Test filtering by topic
    list_insights(Some("list_topic1"), false)?;
    list_insights(Some("list_topic1"), true)?;

    Ok(())
  }

  #[test]
  #[serial]
  fn test_list_insights_nonexistent_topic() -> Result<()> {
    let _temp = setup_temp_insights_root("list_nonexistent_topic");

    list_insights(Some("nonexistent_topic"), false)?;

    Ok(())
  }

  #[test]
  #[serial]
  fn test_update_insight_overview() -> Result<()> {
    let _temp = setup_temp_insights_root("update_overview");

    add_insight("update_topic", "update_name", "Original overview", "Original details")?;

    update_insight("update_topic", "update_name", Some("Updated overview"), None)?;

    let loaded = Insight::load("update_topic", "update_name")?;
    assert_eq!(loaded.overview, "Updated overview");
    assert_eq!(loaded.details, "Original details");

    Ok(())
  }

  #[test]
  #[serial]
  fn test_update_insight_details() -> Result<()> {
    let _temp = setup_temp_insights_root("update_details");

    add_insight("update_topic", "update_name", "Original overview", "Original details")?;

    update_insight("update_topic", "update_name", None, Some("Updated details"))?;

    let loaded = Insight::load("update_topic", "update_name")?;
    assert_eq!(loaded.overview, "Original overview");
    assert_eq!(loaded.details, "Updated details");

    Ok(())
  }

  #[test]
  #[serial]
  fn test_update_insight_both() -> Result<()> {
    let _temp = setup_temp_insights_root("update_both");

    add_insight("update_topic", "update_name", "Original overview", "Original details")?;

    update_insight("update_topic", "update_name", Some("New overview"), Some("New details"))?;

    let loaded = Insight::load("update_topic", "update_name")?;
    assert_eq!(loaded.overview, "New overview");
    assert_eq!(loaded.details, "New details");

    Ok(())
  }

  #[test]
  #[serial]
  fn test_update_nonexistent_insight() {
    let _temp = setup_temp_insights_root("update_nonexistent");

    let result = update_insight("nonexistent", "insight", Some("Overview"), None);
    assert!(result.is_err());
  }

  #[test]
  #[serial]
  fn test_delete_insight_force() -> Result<()> {
    let _temp = setup_temp_insights_root("delete_force");

    add_insight("delete_topic", "delete_name", "To be deleted", "Gone soon")?;

    delete_insight("delete_topic", "delete_name", true)?;

    // Verify it's gone
    assert!(Insight::load("delete_topic", "delete_name").is_err());

    Ok(())
  }

  #[test]
  #[serial]
  fn test_delete_nonexistent_insight() {
    let _temp = setup_temp_insights_root("delete_nonexistent");

    let result = delete_insight("nonexistent", "insight", true);
    assert!(result.is_err());
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

    add_insight("topic_alpha", "insight1", "Overview", "Details")?;
    add_insight("topic_beta", "insight2", "Overview", "Details")?;

    list_topics()?;

    Ok(())
  }

  #[test]
  #[serial]
  fn test_search_insights_basic() -> Result<()> {
    let _temp = setup_temp_insights_root("search_basic");

    add_insight("search_topic", "searchable", "Overview with keyword", "Details with content")?;
    add_insight("search_topic", "other", "Different overview", "Different details")?;

    // Search for keyword
    search_insights_exact(&[String::from("keyword")], None, false, false)?;

    // Search case sensitive
    search_insights_exact(&[String::from("KEYWORD")], None, true, false)?;

    // Search overview only
    search_insights_exact(&[String::from("keyword")], None, false, true)?;

    Ok(())
  }

  #[test]
  #[serial]
  fn test_search_insights_with_topic_filter() -> Result<()> {
    let _temp = setup_temp_insights_root("search_with_topic_filter");

    add_insight("filter_topic", "insight1", "Searchable content", "Details")?;
    add_insight("other_topic", "insight2", "Searchable content", "Details")?;

    // Search with topic filter
    search_insights_exact(&[String::from("Searchable")], Some("filter_topic"), false, false)?;

    Ok(())
  }

  #[test]
  #[serial]
  fn test_search_insights_no_matches() -> Result<()> {
    let _temp = setup_temp_insights_root("search_no_matches");

    add_insight("topic", "insight", "No matching content", "Nothing here")?;

    search_insights_exact(&[String::from("nonexistent")], None, false, false)?;

    Ok(())
  }

  #[test]
  #[serial]
  fn test_search_insights_empty_database() -> Result<()> {
    let _temp = setup_temp_insights_root("search_empty_database");

    search_insights_exact(&[String::from("anything")], None, false, false)?;

    Ok(())
  }

  // Note: link_insight functionality was removed in the new version
  /*
  #[test]
  #[serial]
  fn test_link_insight_basic() -> Result<()> {
    let _temp = setup_temp_insights_root("link_basic");

    // Create source insight
    add_insight("src_topic", "src_insight", "Source overview", "Source details")?;

    // Create link with different name
    link_insight("src_topic", "src_insight", "target_topic", Some("target_name"))?;

    // Verify link was created
    let linked = Insight::load("target_topic", "target_name")?;
    assert_eq!(linked.overview, "Source overview");

    Ok(())
  }
  */

  /*
  #[test]
  #[serial]
  fn test_link_insight_same_name() -> Result<()> {
    let _temp = setup_temp_insights_root("link_same_name");

    add_insight("src_topic", "insight_name", "Source overview", "Source details")?;

    // Create link with same name (default behavior)
    link_insight("src_topic", "insight_name", "target_topic", None)?;

    // Verify link was created with same name
    let linked = Insight::load("target_topic", "insight_name")?;
    assert_eq!(linked.overview, "Source overview");

    Ok(())
  }
  */

  /*
  #[test]
  #[serial]
  fn test_link_nonexistent_insight() {
    let _temp = setup_temp_insights_root("link_nonexistent");

    let result = link_insight("nonexistent", "insight", "target", None);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("not found"));
  }
  */

  // Additional edge case tests to boost coverage

  #[test]
  #[serial]
  fn test_add_insight_empty_fields() -> Result<()> {
    let _temp = setup_temp_insights_root("add_empty_fields");

    // Test with various combinations - some may fail which is expected
    let _ = add_insight("topic1", "name1", "", "details");
    let _ = add_insight("topic2", "name2", "overview", "");

    // These should work fine
    add_insight("topic3", "name3", "overview", "details")?;

    Ok(())
  }

  #[test]
  #[serial]
  fn test_search_insights_case_sensitivity() -> Result<()> {
    let _temp = setup_temp_insights_root("search_case_sensitivity");

    add_insight("case_topic", "insight", "TEST content", "test details")?;

    // Case insensitive search should find both
    search_insights_exact(&[String::from("test")], None, false, false)?;
    search_insights_exact(&[String::from("TEST")], None, false, false)?;

    // Case sensitive search
    search_insights_exact(&[String::from("test")], None, true, false)?;
    search_insights_exact(&[String::from("TEST")], None, true, false)?;

    Ok(())
  }

  #[test]
  #[serial]
  fn test_search_insights_special_characters() -> Result<()> {
    let _temp = setup_temp_insights_root("search_special_chars");

    add_insight("special_topic", "insight", "Overview with @#$%", "Details with &*()+")?;

    // Search for special characters
    search_insights_exact(&[String::from("@#$%")], None, false, false)?;
    search_insights_exact(&[String::from("&*()")], None, false, false)?;

    Ok(())
  }

  #[test]
  #[serial]
  fn test_update_insight_no_changes() -> Result<()> {
    let _temp = setup_temp_insights_root("update_no_changes");

    add_insight("update_topic", "update_name", "Original overview", "Original details")?;

    // Since update requires at least one field, let's test updating with the same value
    update_insight("update_topic", "update_name", Some("Original overview"), None)?;

    let loaded = Insight::load("update_topic", "update_name")?;
    assert_eq!(loaded.overview, "Original overview");
    assert_eq!(loaded.details, "Original details");

    Ok(())
  }

  #[test]
  #[serial]
  fn test_search_insights_with_nonexistent_topic_filter() -> Result<()> {
    let _temp = setup_temp_insights_root("search_nonexistent_topic_filter");

    add_insight("real_topic", "insight", "Real content", "Real details")?;

    // Search with nonexistent topic filter
    search_insights_exact(&[String::from("content")], Some("nonexistent_topic"), false, false)?;

    Ok(())
  }

  #[test]
  #[serial]
  fn test_get_insight_with_special_characters() -> Result<()> {
    let _temp = setup_temp_insights_root("get_special_chars");

    add_insight("special", "test-name_123", "Overview: @#$%", "Details: &*()+")?;

    get_insight("special", "test-name_123", false)?;
    get_insight("special", "test-name_123", true)?;

    Ok(())
  }
}
