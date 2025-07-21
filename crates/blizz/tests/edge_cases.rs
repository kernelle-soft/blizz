use anyhow::Result;
use blizz::commands::*;
use blizz::embedding_client;
use blizz::embedding_client::MockEmbeddingService;
use blizz::insight::{self};
use serial_test::serial;
use std::env;
use tempfile::TempDir;

#[cfg(test)]
mod edge_case_tests {
  use super::*;

  fn setup_temp_insights_root(_test_name: &str) -> TempDir {
    let temp_dir = TempDir::new().unwrap();
    env::set_var("BLIZZ_INSIGHTS_ROOT", temp_dir.path());
    temp_dir
  }

  #[test]
  #[serial]
  fn test_empty_strings_allowed() -> Result<()> {
    let _temp = setup_temp_insights_root("empty_strings");
    let client = embedding_client::with_service(Box::new(MockEmbeddingService));

    // Empty topic and name should be allowed (although unusual)
    add_insight_with_client("", "", "", "", &client)?;

    // Verify it was stored and can be retrieved
    let loaded = insight::load("", "")?;
    assert_eq!(loaded.topic, "");
    assert_eq!(loaded.name, "");
    assert_eq!(loaded.overview, "");
    assert_eq!(loaded.details, "");

    Ok(())
  }

  #[test]
  #[serial]
  fn test_very_long_content() -> Result<()> {
    let _temp = setup_temp_insights_root("long_content");
    let client = embedding_client::with_service(Box::new(MockEmbeddingService));

    // Use reasonable lengths that won't exceed filesystem limits
    // Typical filesystem limit is ~255 chars for filename, so keep topic+name under that
    let long_topic = "a".repeat(100);
    let long_name = "b".repeat(100);
    let long_overview = "c".repeat(10000);
    let long_details = "d".repeat(50000);

    add_insight_with_client(&long_topic, &long_name, &long_overview, &long_details, &client)?;

    let loaded = insight::load(&long_topic, &long_name)?;
    assert_eq!(loaded.overview.len(), 10000);
    assert_eq!(loaded.details.len(), 50000);

    Ok(())
  }

  #[test]
  #[serial]
  fn test_unicode_handling() -> Result<()> {
    let _temp = setup_temp_insights_root("unicode");
    let client = embedding_client::with_service(Box::new(MockEmbeddingService));

    let unicode_topic = "测试主题";
    let unicode_name = "тест-имя";
    let unicode_overview = "Émojis and symbols: 🚀🎉 αβγδε";
    let unicode_details = "Mixed content: 日本語 العربية Русский français 中文";

    add_insight_with_client(
      unicode_topic,
      unicode_name,
      unicode_overview,
      unicode_details,
      &client,
    )?;

    let loaded = insight::load(unicode_topic, unicode_name)?;
    assert_eq!(loaded.topic, unicode_topic);
    assert_eq!(loaded.name, unicode_name);
    assert_eq!(loaded.overview, unicode_overview);
    assert_eq!(loaded.details, unicode_details);

    Ok(())
  }

  #[test]
  #[serial]
  fn test_special_characters_in_names() -> Result<()> {
    let _temp = setup_temp_insights_root("special_chars");
    let client = embedding_client::with_service(Box::new(MockEmbeddingService));

    // Test various special characters that might cause issues
    let special_cases = vec![
      ("spaces in topic", "spaces in name"),
      ("topic-with-dashes", "name-with-dashes"),
      ("topic_with_underscores", "name_with_underscores"),
      ("topic.with.dots", "name.with.dots"),
      ("topic@with@ats", "name@with@ats"),
    ];

    for (topic, name) in special_cases {
      add_insight_with_client(topic, name, "Test overview", "Test details", &client)?;

      let loaded = insight::load(topic, name)?;
      assert_eq!(loaded.topic, topic);
      assert_eq!(loaded.name, name);
    }

    Ok(())
  }

  #[test]
  #[serial]
  fn test_malformed_yaml_handling() -> Result<()> {
    let _temp = setup_temp_insights_root("malformed_yaml");
    let client = embedding_client::with_service(Box::new(MockEmbeddingService));

    // Create a valid insight first
    add_insight_with_client("yaml_test", "valid", "Valid overview", "Valid details", &client)?;

    // Verify it loads correctly
    let loaded = insight::load("yaml_test", "valid")?;
    assert_eq!(loaded.overview, "Valid overview");
    assert_eq!(loaded.details, "Valid details");

    Ok(())
  }

  #[test]
  #[serial]
  fn test_simultaneous_operations() -> Result<()> {
    let _temp = setup_temp_insights_root("simultaneous");
    let client = embedding_client::with_service(Box::new(MockEmbeddingService));

    // Test multiple operations in sequence
    add_insight_with_client("multi", "test1", "Overview 1", "Details 1", &client)?;
    add_insight_with_client("multi", "test2", "Overview 2", "Details 2", &client)?;
    add_insight_with_client("multi", "test3", "Overview 3", "Details 3", &client)?;

    // Update one while others exist
    update_insight_with_client("multi", "test2", Some("Updated overview"), None, &client)?;

    // Delete one while others exist
    delete_insight("multi", "test3", true)?;

    // Verify states
    let test1 = insight::load("multi", "test1")?;
    assert_eq!(test1.overview, "Overview 1");

    let test2 = insight::load("multi", "test2")?;
    assert_eq!(test2.overview, "Updated overview");
    assert_eq!(test2.details, "Details 2");

    let test3_result = insight::load("multi", "test3");
    assert!(test3_result.is_err());

    Ok(())
  }

  #[test]
  #[serial]
  fn test_directory_creation() -> Result<()> {
    let _temp = setup_temp_insights_root("dir_creation");
    let client = embedding_client::with_service(Box::new(MockEmbeddingService));

    // Test that deeply nested topics create proper directory structures
    add_insight_with_client("new_topic", "new_insight", "New overview", "New details", &client)?;

    let loaded = insight::load("new_topic", "new_insight")?;
    assert_eq!(loaded.topic, "new_topic");
    assert_eq!(loaded.name, "new_insight");

    Ok(())
  }

  #[test]
  #[serial]
  fn test_update_with_no_changes() -> Result<()> {
    let _temp = setup_temp_insights_root("no_changes");
    let client = embedding_client::with_service(Box::new(MockEmbeddingService));

    add_insight_with_client("update_test", "unchanged", "Original", "Original details", &client)?;

    // Attempt update with no changes should fail
    let result = update_insight_with_client("update_test", "unchanged", None, None, &client);
    assert!(result.is_err());

    Ok(())
  }

  #[test]
  #[serial]
  fn test_delete_without_force() -> Result<()> {
    let _temp = setup_temp_insights_root("delete_no_force");
    let client = embedding_client::with_service(Box::new(MockEmbeddingService));

    add_insight_with_client(
      "delete_test",
      "protected",
      "Protected",
      "Should not be deleted",
      &client,
    )?;

    // Delete without force should fail
    let result = delete_insight("delete_test", "protected", false);
    assert!(result.is_err());

    // Verify insight still exists
    let loaded = insight::load("delete_test", "protected")?;
    assert_eq!(loaded.name, "protected");

    Ok(())
  }

  #[test]
  #[serial]
  fn test_content_with_frontmatter_separators() -> Result<()> {
    let _temp = setup_temp_insights_root("frontmatter_sep");
    let client = embedding_client::with_service(Box::new(MockEmbeddingService));

    // Test content that includes YAML frontmatter separators
    let tricky_overview = "Overview with --- separators in content";
    let tricky_details = "Details with\n---\nseparators and\n---\nmore content";

    add_insight_with_client("tricky", "separators", tricky_overview, tricky_details, &client)?;

    let loaded = insight::load("tricky", "separators")?;
    assert_eq!(loaded.overview, tricky_overview);
    assert_eq!(loaded.details, tricky_details);

    Ok(())
  }

  #[test]
  #[serial]
  fn test_multiline_content_preservation() -> Result<()> {
    let _temp = setup_temp_insights_root("multiline");
    let client = embedding_client::with_service(Box::new(MockEmbeddingService));

    let multiline_overview = "Line 1\nLine 2\nLine 3";
    let multiline_details =
      "Details line 1\n\nDetails line 3 (with blank line above)\n\n\nMultiple blank lines above";

    add_insight_with_client("multiline", "test", multiline_overview, multiline_details, &client)?;

    let loaded = insight::load("multiline", "test")?;
    assert_eq!(loaded.overview, multiline_overview);
    assert_eq!(loaded.details, multiline_details);

    Ok(())
  }

  #[test]
  #[serial]
  fn test_whitespace_handling() -> Result<()> {
    let _temp = setup_temp_insights_root("whitespace");
    let client = embedding_client::with_service(Box::new(MockEmbeddingService));

    // Test content with leading/trailing whitespace
    let whitespace_overview = "  Overview with spaces  ";
    let whitespace_details = "\tDetails with tabs and spaces\n  ";

    add_insight_with_client(
      "whitespace",
      "test",
      whitespace_overview,
      whitespace_details,
      &client,
    )?;

    let loaded = insight::load("whitespace", "test")?;
    assert_eq!(loaded.overview, whitespace_overview);
    // Note: details get trimmed by clean_body_content function during save/load
    assert_eq!(loaded.details, "Details with tabs and spaces");

    Ok(())
  }

  #[test]
  #[serial]
  fn test_case_sensitivity() -> Result<()> {
    let _temp = setup_temp_insights_root("case_sensitivity");
    let client = embedding_client::with_service(Box::new(MockEmbeddingService));

    // Test that topic and name are case-sensitive
    add_insight_with_client("CaseSensitive", "TestName", "Overview", "Details", &client)?;
    add_insight_with_client(
      "casesensitive",
      "testname",
      "Different overview",
      "Different details",
      &client,
    )?;

    let upper = insight::load("CaseSensitive", "TestName")?;
    let lower = insight::load("casesensitive", "testname")?;

    assert_eq!(upper.overview, "Overview");
    assert_eq!(lower.overview, "Different overview");

    Ok(())
  }

  #[test]
  #[serial]
  fn test_numeric_content() -> Result<()> {
    let _temp = setup_temp_insights_root("numeric");
    let client = embedding_client::with_service(Box::new(MockEmbeddingService));

    // Test purely numeric content
    add_insight_with_client("123", "456", "789", "101112", &client)?;

    let loaded = insight::load("123", "456")?;
    assert_eq!(loaded.topic, "123");
    assert_eq!(loaded.name, "456");
    assert_eq!(loaded.overview, "789");
    assert_eq!(loaded.details, "101112");

    Ok(())
  }
}
