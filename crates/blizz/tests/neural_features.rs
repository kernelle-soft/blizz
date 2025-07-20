use anyhow::Result;
use blizz::insight::*;
use chrono::{DateTime, Utc};
use serial_test::serial;
use std::env;
use tempfile::TempDir;

#[cfg(test)]
mod neural_feature_tests {
  use super::*;

  fn setup_temp_insights_root(test_name: &str) -> TempDir {
    let temp_dir = TempDir::new().unwrap();
    env::set_var("BLIZZ_INSIGHTS_ROOT", temp_dir.path());
    temp_dir
  }

  #[test]
  #[serial]
  fn test_insight_new_with_embedding() {
    let insight = Insight::new_with_embedding(
      "test_topic".to_string(),
      "test_name".to_string(),
      "Test overview".to_string(),
      "Test details".to_string(),
      "v1.0".to_string(),
      vec![0.1, 0.2, 0.3],
      "embedded text".to_string(),
    );

    assert_eq!(insight.topic, "test_topic");
    assert_eq!(insight.name, "test_name");
    assert_eq!(insight.overview, "Test overview");
    assert_eq!(insight.details, "Test details");
    assert_eq!(insight.embedding_version, Some("v1.0".to_string()));
    assert_eq!(insight.embedding, Some(vec![0.1, 0.2, 0.3]));
    assert_eq!(insight.embedding_text, Some("embedded text".to_string()));
    assert!(insight.embedding_computed.is_some());
    assert!(insight.has_embedding());
  }

  #[test]
  #[serial]
  fn test_insight_set_embedding() {
    let mut insight = Insight::new(
      "test_topic".to_string(),
      "test_name".to_string(),
      "Test overview".to_string(),
      "Test details".to_string(),
    );

    assert!(!insight.has_embedding());

    insight.set_embedding(
      "v2.0".to_string(),
      vec![0.4, 0.5, 0.6],
      "new embedded text".to_string(),
    );

    assert!(insight.has_embedding());
    assert_eq!(insight.embedding_version, Some("v2.0".to_string()));
    assert_eq!(insight.embedding, Some(vec![0.4, 0.5, 0.6]));
    assert_eq!(insight.embedding_text, Some("new embedded text".to_string()));
    assert!(insight.embedding_computed.is_some());
  }

  #[test]
  #[serial]
  fn test_insight_get_embedding_text() {
    let insight = Insight::new(
      "my_topic".to_string(),
      "my_name".to_string(),
      "My overview".to_string(),
      "My details".to_string(),
    );

    let embedding_text = insight.get_embedding_text();
    assert_eq!(embedding_text, "my_topic my_name My overview My details");
  }

  #[test]
  #[serial]
  fn test_save_and_load_insight_with_embeddings() -> Result<()> {
    let _temp = setup_temp_insights_root("save_load_embeddings");

    let original = Insight::new_with_embedding(
      "test_topic".to_string(),
      "test_name".to_string(),
      "Test overview".to_string(),
      "Test details".to_string(),
      "v1.5".to_string(),
      vec![0.7, 0.8, 0.9],
      "test embedding text".to_string(),
    );

    original.save()?;

    let loaded = Insight::load("test_topic", "test_name")?;
    
    assert_eq!(loaded.topic, "test_topic");
    assert_eq!(loaded.name, "test_name");
    assert_eq!(loaded.overview, "Test overview");
    assert_eq!(loaded.details, "Test details");
    assert_eq!(loaded.embedding_version, Some("v1.5".to_string()));
    assert_eq!(loaded.embedding, Some(vec![0.7, 0.8, 0.9]));
    assert_eq!(loaded.embedding_text, Some("test embedding text".to_string()));
    assert!(loaded.embedding_computed.is_some());
    assert!(loaded.has_embedding());

    Ok(())
  }

  #[test]
  #[serial]
  fn test_save_insight_without_embeddings() -> Result<()> {
    let _temp = setup_temp_insights_root("save_no_embeddings");

    let insight = Insight::new(
      "test_topic".to_string(),
      "test_name".to_string(),
      "Test overview".to_string(),
      "Test details".to_string(),
    );

    insight.save()?;

    let loaded = Insight::load("test_topic", "test_name")?;
    
    assert_eq!(loaded.overview, "Test overview");
    assert_eq!(loaded.details, "Test details");
    assert_eq!(loaded.embedding_version, None);
    assert_eq!(loaded.embedding, None);
    assert_eq!(loaded.embedding_text, None);
    assert_eq!(loaded.embedding_computed, None);
    assert!(!loaded.has_embedding());

    Ok(())
  }

  #[test]
  #[serial]
  fn test_update_insight_clears_embeddings() -> Result<()> {
    let _temp = setup_temp_insights_root("update_clears_embeddings");

    let mut insight = Insight::new_with_embedding(
      "test_topic".to_string(),
      "test_name".to_string(),
      "Original overview".to_string(),
      "Original details".to_string(),
      "v1.0".to_string(),
      vec![0.1, 0.2, 0.3],
      "original text".to_string(),
    );

    insight.save()?;
    assert!(insight.has_embedding());

    // Update should clear embeddings
    insight.update(Some("Updated overview"), Some("Updated details"))?;
    
    assert_eq!(insight.overview, "Updated overview");
    assert_eq!(insight.details, "Updated details");
    assert!(!insight.has_embedding());
    assert_eq!(insight.embedding_version, None);
    assert_eq!(insight.embedding, None);
    assert_eq!(insight.embedding_text, None);
    assert_eq!(insight.embedding_computed, None);

    Ok(())
  }

  #[test]
  #[serial]
  fn test_parse_yaml_frontmatter_new_format() -> Result<()> {
    let content = r#"---
overview: "Test overview content"
embedding_version: "v1.0"
embedding: [0.1, 0.2, 0.3, 0.4]
embedding_text: "test topic test name Test overview content Test details content"
embedding_computed: "2024-01-01T12:00:00Z"
---

# Details
Test details content"#;

    let (frontmatter, details) = parse_insight_with_metadata(content)?;
    
    assert_eq!(frontmatter.overview, "Test overview content");
    assert_eq!(frontmatter.embedding_version, Some("v1.0".to_string()));
    assert_eq!(frontmatter.embedding, Some(vec![0.1, 0.2, 0.3, 0.4]));
    assert_eq!(frontmatter.embedding_text, Some("test topic test name Test overview content Test details content".to_string()));
    assert!(frontmatter.embedding_computed.is_some());
    assert_eq!(details, "Test details content");

    Ok(())
  }

  #[test]
  #[serial]
  fn test_parse_yaml_frontmatter_minimal() -> Result<()> {
    let content = r#"---
overview: "Simple overview"
---

# Details
Simple details"#;

    let (frontmatter, details) = parse_insight_with_metadata(content)?;
    
    assert_eq!(frontmatter.overview, "Simple overview");
    assert_eq!(frontmatter.embedding_version, None);
    assert_eq!(frontmatter.embedding, None);
    assert_eq!(frontmatter.embedding_text, None);
    assert_eq!(frontmatter.embedding_computed, None);
    assert_eq!(details, "Simple details");

    Ok(())
  }

  #[test]
  #[serial]
  fn test_parse_legacy_format_compatibility() -> Result<()> {
    let legacy_content = r#"---
This is the legacy overview content
---

This is the legacy details content"#;

    let (frontmatter, details) = parse_insight_with_metadata(legacy_content)?;
    
    assert_eq!(frontmatter.overview, "This is the legacy overview content");
    assert_eq!(frontmatter.embedding_version, None);
    assert_eq!(frontmatter.embedding, None);
    assert_eq!(frontmatter.embedding_text, None);
    assert_eq!(frontmatter.embedding_computed, None);
    assert_eq!(details, "This is the legacy details content");

    Ok(())
  }

  #[test]
  #[serial]
  fn test_parse_insight_content_legacy_compatibility() -> Result<()> {
    let legacy_content = r#"---
Legacy overview
---

Legacy details"#;

    let (overview, details) = parse_insight_content(legacy_content)?;
    
    assert_eq!(overview, "Legacy overview");
    assert_eq!(details, "Legacy details");

    Ok(())
  }

  #[test]
  #[serial]
  fn test_parse_insight_content_new_format() -> Result<()> {
    let new_content = r#"---
overview: "New format overview"
embedding_version: "v1.0"
---

# Details
New format details"#;

    let (overview, details) = parse_insight_content(new_content)?;
    
    assert_eq!(overview, "New format overview");
    assert_eq!(details, "New format details");

    Ok(())
  }

  #[test]
  #[serial]
  fn test_invalid_frontmatter_format() {
    let invalid_content = "This content has no frontmatter";

    let result = parse_insight_with_metadata(invalid_content);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("missing frontmatter"));
  }

  #[test]
  #[serial]
  fn test_frontmatter_serialization() -> Result<()> {
    let frontmatter = FrontMatter {
      overview: "Test overview".to_string(),
      embedding_version: Some("v1.0".to_string()),
      embedding: Some(vec![0.1, 0.2, 0.3]),
      embedding_text: Some("embedded text".to_string()),
      embedding_computed: Some(DateTime::parse_from_rfc3339("2024-01-01T12:00:00Z")?.with_timezone(&Utc)),
    };

    let yaml = serde_yaml::to_string(&frontmatter)?;
    let deserialized: FrontMatter = serde_yaml::from_str(&yaml)?;

    assert_eq!(deserialized.overview, "Test overview");
    assert_eq!(deserialized.embedding_version, Some("v1.0".to_string()));
    assert_eq!(deserialized.embedding, Some(vec![0.1, 0.2, 0.3]));
    assert_eq!(deserialized.embedding_text, Some("embedded text".to_string()));
    assert!(deserialized.embedding_computed.is_some());

    Ok(())
  }

  #[test]
  #[serial]
  fn test_frontmatter_skips_none_values() -> Result<()> {
    let frontmatter = FrontMatter {
      overview: "Just overview".to_string(),
      embedding_version: None,
      embedding: None,
      embedding_text: None,
      embedding_computed: None,
    };

    let yaml = serde_yaml::to_string(&frontmatter)?;
    
    // Should only contain overview, not the None fields
    assert!(yaml.contains("overview:"));
    assert!(!yaml.contains("embedding_version:"));
    assert!(!yaml.contains("embedding:"));
    assert!(!yaml.contains("embedding_text:"));
    assert!(!yaml.contains("embedding_computed:"));

    Ok(())
  }
} 