#[cfg(all(test, feature = "neural"))]
use anyhow::Result;
#[cfg(all(test, feature = "neural"))]
use chrono::Utc;
#[cfg(all(test, feature = "neural"))]
use insights::embedding_client::Embedding;
#[cfg(all(test, feature = "neural"))]
use insights::insight::{self, Insight};
#[cfg(all(test, feature = "neural"))]
use serial_test::serial;
#[cfg(all(test, feature = "neural"))]
use std::env;
#[cfg(all(test, feature = "neural"))]
use tempfile::TempDir;

#[cfg(all(test, feature = "neural"))]
mod neural_feature_tests {
  use super::*;

  fn setup_temp_insights_root(_test_name: &str) -> TempDir {
    let temp_dir = TempDir::new().unwrap();
    env::set_var("INSIGHTS_ROOT", temp_dir.path());
    temp_dir
  }

  #[test]
  #[serial]
  fn test_insight_new_with_embedding() {
    let mut insight = Insight::new(
      "test_topic".to_string(),
      "test_name".to_string(),
      "Test overview".to_string(),
      "Test details".to_string(),
    );
    insight::set_embedding(
      &mut insight,
      Embedding {
        version: "v1.0".to_string(),
        created_at: Utc::now(),
        embedding: vec![0.1, 0.2, 0.3],
      },
    );

    assert_eq!(insight.topic, "test_topic");
    assert_eq!(insight.name, "test_name");
    assert_eq!(insight.overview, "Test overview");
    assert_eq!(insight.details, "Test details");
    assert_eq!(insight.embedding_version, Some("v1.0".to_string()));
    assert_eq!(insight.embedding, Some(vec![0.1, 0.2, 0.3]));
    assert!(insight::has_embedding(&insight));
  }

  #[test]
  #[serial]
  fn test_insight_new_without_embedding() {
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
    assert_eq!(insight.embedding_version, None);
    assert_eq!(insight.embedding, None);
    assert!(!insight::has_embedding(&insight));

    let mut insight_with_embedding = insight;
    insight::set_embedding(
      &mut insight_with_embedding,
      Embedding {
        version: "v2.0".to_string(),
        created_at: Utc::now(),
        embedding: vec![0.4, 0.5, 0.6],
      },
    );

    assert!(insight::has_embedding(&insight_with_embedding));
  }

  #[test]
  #[serial]
  fn test_get_embedding_text() -> Result<()> {
    let insight = Insight::new(
      "test_topic".to_string(),
      "test_name".to_string(),
      "Test overview".to_string(),
      "Test details".to_string(),
    );

    let embedding_text = insight::get_embedding_text(&insight);

    // Should combine topic, name, overview, and details
    assert!(embedding_text.contains("test_topic"));
    assert!(embedding_text.contains("test_name"));
    assert!(embedding_text.contains("Test overview"));
    assert!(embedding_text.contains("Test details"));

    Ok(())
  }

  #[test]
  #[serial]
  fn test_embedding_persistence() -> Result<()> {
    let _temp = setup_temp_insights_root("embedding_persistence");

    let mut original = Insight::new(
      "test_topic".to_string(),
      "test_name".to_string(),
      "Test overview".to_string(),
      "Test details".to_string(),
    );

    insight::set_embedding(
      &mut original,
      Embedding {
        version: "v1.0".to_string(),
        created_at: Utc::now(),
        embedding: vec![0.1, 0.2, 0.3],
      },
    );

    insight::save(&original)?;

    let loaded = insight::load("test_topic", "test_name")?;
    assert!(insight::has_embedding(&loaded));
    assert_eq!(loaded.embedding_version, Some("v1.0".to_string()));
    assert_eq!(loaded.embedding, Some(vec![0.1, 0.2, 0.3]));

    Ok(())
  }

  #[test]
  #[serial]
  fn test_embedding_created_timestamp() -> Result<()> {
    let _temp = setup_temp_insights_root("embedding_timestamp");

    let before = Utc::now();

    let mut insight = Insight::new(
      "test_topic".to_string(),
      "test_name".to_string(),
      "Test overview".to_string(),
      "Test details".to_string(),
    );

    let embedding_time = Utc::now();
    insight::set_embedding(
      &mut insight,
      Embedding {
        version: "v1.0".to_string(),
        created_at: embedding_time,
        embedding: vec![0.1, 0.2, 0.3],
      },
    );

    insight::save(&insight)?;
    let loaded = insight::load("test_topic", "test_name")?;

    let after = Utc::now();

    assert!(insight::has_embedding(&loaded));
    if let Some(computed_time) = loaded.embedding_computed {
      assert!(computed_time >= before);
      assert!(computed_time <= after);
    }

    Ok(())
  }

  #[test]
  #[serial]
  fn test_embedding_update_clears_previous() -> Result<()> {
    let _temp = setup_temp_insights_root("embedding_update");

    let mut insight = Insight::new(
      "test_topic".to_string(),
      "test_name".to_string(),
      "Test overview".to_string(),
      "Test details".to_string(),
    );

    // Set initial embedding
    insight::set_embedding(
      &mut insight,
      Embedding {
        version: "v1.0".to_string(),
        created_at: Utc::now(),
        embedding: vec![0.1, 0.2, 0.3],
      },
    );

    insight::save(&insight)?;
    assert!(insight::has_embedding(&insight));

    // Update the insight content (which should clear embedding in practice)
    insight::update(&mut insight, Some("Updated overview"), Some("Updated details"))?;

    // Note: In a real system, updating content would clear embeddings
    // Here we're just testing the data structure behavior
    assert!(!insight::has_embedding(&insight));

    Ok(())
  }

  #[test]
  #[serial]
  fn test_embedding_version_tracking() -> Result<()> {
    let _temp = setup_temp_insights_root("embedding_version");

    let mut insight = Insight::new(
      "test_topic".to_string(),
      "test_name".to_string(),
      "Test overview".to_string(),
      "Test details".to_string(),
    );

    // Test version v1.0
    insight::set_embedding(
      &mut insight,
      Embedding {
        version: "v1.0".to_string(),
        created_at: Utc::now(),
        embedding: vec![0.1, 0.2, 0.3],
      },
    );

    assert_eq!(insight.embedding_version, Some("v1.0".to_string()));

    // Test version v2.0
    insight::set_embedding(
      &mut insight,
      Embedding {
        version: "v2.0".to_string(),
        created_at: Utc::now(),
        embedding: vec![0.4, 0.5, 0.6],
      },
    );

    assert_eq!(insight.embedding_version, Some("v2.0".to_string()));

    Ok(())
  }

  #[test]
  #[serial]
  fn test_embedding_dimension_consistency() -> Result<()> {
    let _temp = setup_temp_insights_root("embedding_dimension");

    let mut insight = Insight::new(
      "test_topic".to_string(),
      "test_name".to_string(),
      "Test overview".to_string(),
      "Test details".to_string(),
    );

    // Test different embedding dimensions
    let small_embedding = vec![0.1, 0.2, 0.3];
    let large_embedding = vec![0.1; 384]; // Common dimension size

    insight::set_embedding(
      &mut insight,
      Embedding {
        version: "small".to_string(),
        created_at: Utc::now(),
        embedding: small_embedding.clone(),
      },
    );

    assert_eq!(insight.embedding, Some(small_embedding));

    insight::set_embedding(
      &mut insight,
      Embedding {
        version: "large".to_string(),
        created_at: Utc::now(),
        embedding: large_embedding.clone(),
      },
    );

    assert_eq!(insight.embedding, Some(large_embedding));

    Ok(())
  }

  #[test]
  #[serial]
  fn test_embedding_text_content_changes() -> Result<()> {
    let insight1 = Insight::new(
      "topic1".to_string(),
      "name1".to_string(),
      "Overview 1".to_string(),
      "Details 1".to_string(),
    );

    let insight2 = Insight::new(
      "topic2".to_string(),
      "name2".to_string(),
      "Overview 2".to_string(),
      "Details 2".to_string(),
    );

    let text1 = insight::get_embedding_text(&insight1);
    let text2 = insight::get_embedding_text(&insight2);

    // Different insights should produce different embedding text
    assert_ne!(text1, text2);

    // Same insight should produce same text
    let text1_again = insight::get_embedding_text(&insight1);
    assert_eq!(text1, text1_again);

    Ok(())
  }

  // Legacy format parsing tests
  #[test]
  #[serial]
  fn test_parse_legacy_insight_format() -> Result<()> {
    let legacy_content = "---\nThis is the overview content.\n---\n\nThis is the details section.\nWith multiple lines.";

    let (metadata, details) = insight::parse_insight_with_metadata(legacy_content)?;
    assert_eq!(metadata.overview, "This is the overview content.");
    assert_eq!(details, "This is the details section.\nWith multiple lines.");

    Ok(())
  }

  #[test]
  #[serial]
  fn test_parse_new_yaml_format() -> Result<()> {
    let new_content =
      "---\ntopic: \"TestTopic\"\nname: \"TestName\"\noverview: \"This is the overview\"\n---\n\n# Details\nThis is the details section.";

    let (metadata, details) = insight::parse_insight_with_metadata(new_content)?;
    assert_eq!(metadata.topic, "TestTopic");
    assert_eq!(metadata.name, "TestName");
    assert_eq!(metadata.overview, "This is the overview");
    assert_eq!(details, "This is the details section.");

    Ok(())
  }

  #[test]
  #[serial]
  fn test_embedding_edge_cases() -> Result<()> {
    let _temp = setup_temp_insights_root("embedding_edge_cases");

    // Test empty embedding vector
    let mut insight = Insight::new(
      "test_topic".to_string(),
      "test_name".to_string(),
      "Test overview".to_string(),
      "Test details".to_string(),
    );

    insight::set_embedding(
      &mut insight,
      Embedding { version: "empty".to_string(), created_at: Utc::now(), embedding: vec![] },
    );

    assert!(insight::has_embedding(&insight));
    assert_eq!(insight.embedding, Some(vec![]));

    // Test very large embedding
    let large_embedding = vec![0.5; 1536]; // GPT-3 embedding size
    insight::set_embedding(
      &mut insight,
      Embedding {
        version: "large".to_string(),
        created_at: Utc::now(),
        embedding: large_embedding.clone(),
      },
    );

    assert_eq!(insight.embedding, Some(large_embedding));

    Ok(())
  }

  #[test]
  #[serial]
  fn test_embedding_special_characters() -> Result<()> {
    let insight = Insight::new(
      "特殊文字".to_string(),
      "émojis🚀".to_string(),
      "Overview with special chars: @#$%^&*()".to_string(),
      "Details with unicode: ñáéíóú and symbols: ∑∆∞".to_string(),
    );

    let embedding_text = insight::get_embedding_text(&insight);

    // Should contain all special characters
    assert!(embedding_text.contains("特殊文字"));
    assert!(embedding_text.contains("émojis🚀"));
    assert!(embedding_text.contains("@#$%^&*()"));
    assert!(embedding_text.contains("ñáéíóú"));
    assert!(embedding_text.contains("∑∆∞"));

    Ok(())
  }
}
