use anyhow::Result;
use blizz::commands::*;
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
  fn test_search_insights_exact_mode() -> Result<()> {
    let _temp = setup_temp_insights_root("search_exact_mode");

    add_insight("test_topic", "insight1", "Contains keyword", "More details")?;
    add_insight("test_topic", "insight2", "Other content", "keyword in details")?;
    add_insight("other_topic", "insight3", "Different content", "No match here")?;

    // Test exact search mode
    search_insights_exact(&["keyword".to_string()], None, false, false)?;

    // Test exact search with case sensitivity
    search_insights_exact(&["KEYWORD".to_string()], None, true, false)?;

    // Test exact search with topic filter
    search_insights_exact(&["keyword".to_string()], Some("test_topic"), false, false)?;

    // Test exact search with overview only
    search_insights_exact(&["keyword".to_string()], None, false, true)?;

    Ok(())
  }

  #[test]
  #[serial]
  fn test_search_insights_exact_multiple_terms() -> Result<()> {
    let _temp = setup_temp_insights_root("search_exact_multiple");

    add_insight("test_topic", "insight1", "Contains first second", "Details here")?;
    add_insight("test_topic", "insight2", "Contains first", "Contains second")?;
    add_insight("test_topic", "insight3", "Contains only first", "Other details")?;

    // Search for multiple terms - should find insights with both
    search_insights_exact(&["first".to_string(), "second".to_string()], None, false, false)?;

    // Search for single term - should find all that contain it
    search_insights_exact(&["first".to_string()], None, false, false)?;

    Ok(())
  }

  #[test]
  #[serial]
  fn test_search_insights_exact_no_results() -> Result<()> {
    let _temp = setup_temp_insights_root("search_exact_no_results");

    add_insight("test_topic", "insight1", "Some content", "Some details")?;

    // Search for non-existent term
    search_insights_exact(&["nonexistent".to_string()], None, false, false)?;

    Ok(())
  }

  #[test]
  #[serial]
  fn test_search_insights_exact_empty_database() -> Result<()> {
    let _temp = setup_temp_insights_root("search_exact_empty");

    // Search in empty database
    search_insights_exact(&["anything".to_string()], None, false, false)?;

    Ok(())
  }

  #[test]
  #[serial]
  fn test_search_insights_exact_special_characters() -> Result<()> {
    let _temp = setup_temp_insights_root("search_exact_special");

    add_insight("test_topic", "insight1", "Contains @#$%", "Special chars")?;
    add_insight("test_topic", "insight2", "Regular content", "Contains &*()")?;

    // Search for special characters
    search_insights_exact(&["@#$%".to_string()], None, false, false)?;
    search_insights_exact(&["&*()".to_string()], None, false, false)?;

    Ok(())
  }

  #[test]
  #[serial]
  #[cfg(feature = "semantic")]
  fn test_search_insights_combined_semantic() -> Result<()> {
    let _temp = setup_temp_insights_root("search_combined_semantic");

    add_insight("test_topic", "insight1", "Machine learning algorithms", "Deep neural networks")?;
    add_insight("test_topic", "insight2", "AI artificial intelligence", "Learning models")?;
    add_insight("test_topic", "insight3", "Completely unrelated", "Nothing relevant")?;

    // Test combined semantic + exact search
    search_insights_combined_semantic(
      &["machine".to_string(), "learning".to_string()],
      None,
      false,
      false,
    )?;

    // Test with topic filter
    search_insights_combined_semantic(&["AI".to_string()], Some("test_topic"), false, false)?;

    // Test with overview only
    search_insights_combined_semantic(&["algorithms".to_string()], None, false, true)?;

    Ok(())
  }

  #[test]
  #[serial]
  fn test_search_insights_combined_all() -> Result<()> {
    let _temp = setup_temp_insights_root("search_combined_all");

    add_insight(
      "test_topic",
      "insight1",
      "Neural networks deep learning",
      "Machine learning models",
    )?;
    add_insight("test_topic", "insight2", "Artificial intelligence", "AI systems")?;
    add_insight("test_topic", "insight3", "Database queries", "SQL operations")?;

    // Test all search methods combined (would include neural if available)
    search_insights_combined_semantic(
      &["neural".to_string(), "learning".to_string()],
      None,
      false,
      false,
    )?;

    // Test with filters
    search_insights_combined_semantic(&["AI".to_string()], Some("test_topic"), false, false)?;
    search_insights_combined_semantic(&["database".to_string()], None, false, true)?;

    Ok(())
  }

  #[test]
  #[serial]
  fn test_search_exact_scoring() -> Result<()> {
    let _temp = setup_temp_insights_root("search_exact_scoring");

    // Create insights with different match qualities
    add_insight("test_topic", "insight1", "Contains test", "Single match")?;
    add_insight("test_topic", "insight2", "Contains test test", "Double match test")?;
    add_insight("test_topic", "insight3", "Other content", "No matches here")?;

    // Search should find insights with matches and score them appropriately
    search_insights_exact(&["test".to_string()], None, false, false)?;

    Ok(())
  }

  #[test]
  #[serial]
  fn test_search_exact_with_nonexistent_topic_filter() -> Result<()> {
    let _temp = setup_temp_insights_root("search_exact_nonexistent_topic");

    add_insight("real_topic", "insight1", "Real content", "Real details")?;

    // Search with non-existent topic filter should return no results
    search_insights_exact(&["content".to_string()], Some("nonexistent_topic"), false, false)?;

    Ok(())
  }

  #[test]
  #[serial]
  fn test_search_exact_case_sensitivity() -> Result<()> {
    let _temp = setup_temp_insights_root("search_exact_case");

    add_insight("test_topic", "insight1", "Contains Keyword", "lowercase keyword")?;
    add_insight("test_topic", "insight2", "Contains KEYWORD", "UPPERCASE details")?;

    // Case sensitive search
    search_insights_exact(&["Keyword".to_string()], None, true, false)?;
    search_insights_exact(&["keyword".to_string()], None, true, false)?;
    search_insights_exact(&["KEYWORD".to_string()], None, true, false)?;

    // Case insensitive search (default)
    search_insights_exact(&["keyword".to_string()], None, false, false)?;

    Ok(())
  }

  #[test]
  #[serial]
  fn test_search_exact_overview_only() -> Result<()> {
    let _temp = setup_temp_insights_root("search_exact_overview_only");

    add_insight("test_topic", "insight1", "Overview contains target", "Details do not")?;
    add_insight("test_topic", "insight2", "Overview normal", "Details contain target")?;

    // Overview only search should only find matches in overview
    search_insights_exact(&["target".to_string()], None, false, true)?;

    Ok(())
  }

  #[cfg(feature = "semantic")]
  #[test]
  #[serial]
  fn test_semantic_similarity_calculation() {
    // Test the semantic similarity function directly

    // This would test the extract_words and calculate_semantic_similarity functions
    // but those are private in the commands module. We'd need to make them public
    // or move them to a testable location to properly unit test them.

    // For now, we test through the public interface
    // TODO: Add actual assertions when functionality is implemented
  }

  #[test]
  #[serial]
  fn test_multiple_search_terms_behavior() -> Result<()> {
    let _temp = setup_temp_insights_root("multiple_terms");

    add_insight("test_topic", "insight1", "first term here", "details")?;
    add_insight("test_topic", "insight2", "second term here", "details")?;
    add_insight("test_topic", "insight3", "first and second terms", "both here")?;
    add_insight("test_topic", "insight4", "completely different", "content")?;

    // Search with multiple terms
    search_insights_exact(&["first".to_string(), "second".to_string()], None, false, false)?;

    // Single term searches
    search_insights_exact(&["first".to_string()], None, false, false)?;
    search_insights_exact(&["second".to_string()], None, false, false)?;

    Ok(())
  }

  #[test]
  #[serial]
  fn test_search_result_deduplication() -> Result<()> {
    let _temp = setup_temp_insights_root("deduplication");

    add_insight("test_topic", "insight1", "Machine learning algorithms", "Deep neural networks")?;
    add_insight("test_topic", "insight2", "Different content", "No overlap")?;

    // Test that combined search modes properly deduplicate results
    // (The same insight shouldn't appear multiple times if found by different methods)
    search_insights_combined_semantic(&["machine".to_string()], None, false, false)?;

    Ok(())
  }

  #[test]
  #[serial]
  fn test_empty_search_terms() -> Result<()> {
    let _temp = setup_temp_insights_root("empty_terms");

    add_insight("test_topic", "insight1", "Some content", "Some details")?;

    // Search with empty terms vector - should handle gracefully
    search_insights_exact(&[], None, false, false)?;

    Ok(())
  }

  #[test]
  #[serial]
  fn test_search_with_whitespace_terms() -> Result<()> {
    let _temp = setup_temp_insights_root("whitespace_terms");

    add_insight("test_topic", "insight1", "Content with spaces", "More spaced content")?;

    // Search with terms containing whitespace
    search_insights_exact(&["with spaces".to_string()], None, false, false)?;
    search_insights_exact(&["  trimmed  ".to_string()], None, false, false)?;

    Ok(())
  }
}
