use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use dirs::home_dir;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

use crate::embedding_client::Embedding;

/// YAML frontmatter structure for insight files
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrontMatter {
  pub overview: String,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub embedding_version: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub embedding: Option<Vec<f32>>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub embedding_text: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub embedding_computed: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Insight {
  pub topic: String,
  pub name: String,
  pub overview: String,
  pub details: String,

  // Embedding metadata (None if not computed yet)
  pub embedding_version: Option<String>,
  pub embedding: Option<Vec<f32>>,
  pub embedding_text: Option<String>, // The exact text that was embedded
  pub embedding_computed: Option<DateTime<Utc>>,
}

impl Insight {
  pub fn new(topic: String, name: String, overview: String, details: String) -> Self {
    Self {
      topic,
      name,
      overview,
      details,
      embedding_version: None,
      embedding: None,
      embedding_text: None,
      embedding_computed: None,
    }
  }

  pub fn file_path(&self) -> Result<PathBuf> {
    let insights_root = get_insights_root()?;
    Ok(insights_root.join(&self.topic).join(format!("{}.insight.md", self.name)))
  }
}

// Embedding-related operations
impl Insight {
  /// Update embedding metadata for this insight
  pub fn set_embedding(&mut self, embedding: Embedding) {
    self.embedding_version = Some(embedding.version);
    self.embedding = Some(embedding.embedding);
    self.embedding_computed = Some(embedding.created_at);
  }

  /// Check if this insight has cached embedding
  pub fn has_embedding(&self) -> bool {
    self.embedding.is_some()
  }

  /// Get the text that should be embedded for this insight
  pub fn get_embedding_text(&self) -> String {
    format!("{} {} {} {}", self.topic, self.name, self.overview, self.details)
  }
}

// Core file operations
impl Insight {
  /// Save this insight to disk using YAML frontmatter format
  pub fn save(&self) -> Result<()> {
    let file_path = self.file_path()?;
    ensure_parent_dir_exists(&file_path)?;
    check_insight_is_new(&file_path, &self.topic, &self.name)?;
    self.write_to_file(&file_path)
  }

  /// Load an insight from disk
  pub fn load(topic: &str, name: &str) -> Result<Self> {
    let file_path = make_insight_path(topic, name)?;
    check_insight_exists(&file_path, topic, name)?;
    let content = fs::read_to_string(&file_path)?;
    parse_insight_from_content(topic, name, &content)
  }

  /// Delete this insight from disk
  pub fn delete(&self) -> Result<()> {
    let file_path = self.file_path()?;
    check_insight_exists(&file_path, &self.topic, &self.name)?;
    fs::remove_file(&file_path)?;
    cleanup_empty_dir(&file_path)?;
    Ok(())
  }
}

fn make_insight_path(topic: &str, name: &str) -> Result<std::path::PathBuf> {
  let root = get_insights_root()?;
  Ok(root.join(topic).join(format!("{name}.insight.md")))
}

fn ensure_parent_dir_exists(path: &std::path::Path) -> Result<()> {
  if let Some(parent) = path.parent() {
    fs::create_dir_all(parent)?;
  }
  Ok(())
}

fn check_insight_is_new(path: &std::path::Path, topic: &str, name: &str) -> Result<()> {
  if path.exists() {
    return Err(anyhow!("Insight {}/{} already exists", topic, name));
  }
  Ok(())
}

fn check_insight_exists(path: &std::path::Path, topic: &str, name: &str) -> Result<()> {
  if !path.exists() {
    return Err(anyhow!("Insight {}/{} not found", topic, name));
  }
  Ok(())
}

fn parse_insight_from_content(topic: &str, name: &str, content: &str) -> Result<Insight> {
  let (fm, details) = parse_insight_with_metadata(content)?;
  Ok(Insight {
    topic: topic.to_string(),
    name: name.to_string(),
    overview: fm.overview,
    details,
    embedding_version: fm.embedding_version,
    embedding: fm.embedding,
    embedding_text: fm.embedding_text,
    embedding_computed: fm.embedding_computed,
  })
}

fn cleanup_empty_dir(path: &std::path::Path) -> Result<()> {
  if let Some(dir) = path.parent() {
    if dir.read_dir()?.next().is_none() {
      fs::remove_dir(dir)?;
    }
  }
  Ok(())
}

// Update operations
impl Insight {
  /// Update this insight on disk using YAML frontmatter format
  pub fn update(&mut self, new_overview: Option<&str>, new_details: Option<&str>) -> Result<()> {
    if let Some(overview) = new_overview {
      self.overview = overview.to_string();
    }
    if let Some(details) = new_details {
      self.details = details.to_string();
    }

    // Check if at least one section is being updated
    if new_overview.is_none() && new_details.is_none() {
      return Err(anyhow!("At least one of overview or details must be provided"));
    }

    let file_path = self.file_path()?;
    if !file_path.exists() {
      return Err(anyhow!("Insight {}/{} not found", self.topic, self.name));
    }

    // Clear embedding metadata since content changed
    self.clear_embedding_cache_if_content_changed(new_overview, new_details);

    self.write_to_file(&file_path)
  }
}

// Helper functions for file operations
impl Insight {
  /// Helper method to write insight to file
  fn write_to_file(&self, file_path: &PathBuf) -> Result<()> {
    let frontmatter = FrontMatter {
      overview: self.overview.clone(),
      embedding_version: self.embedding_version.clone(),
      embedding: self.embedding.clone(),
      embedding_text: self.embedding_text.clone(),
      embedding_computed: self.embedding_computed,
    };

    let yaml_content = serde_yaml::to_string(&frontmatter)?;
    let content = format!("---\n{}---\n\n# Details\n{}", yaml_content, self.details);
    fs::write(file_path, content)?;

    Ok(())
  }

  /// Helper method to clear embedding cache when content changes
  fn clear_embedding_cache_if_content_changed(
    &mut self,
    new_overview: Option<&str>,
    new_details: Option<&str>,
  ) {
    if new_overview.is_some() || new_details.is_some() {
      self.embedding_version = None;
      self.embedding = None;
      self.embedding_text = None;
      self.embedding_computed = None;
    }
  }
}

/// Get the insights root directory (~/.kernelle/insights)
pub fn get_insights_root() -> Result<PathBuf> {
  // Allow tests or callers to override the root directory via env var
  if let Ok(custom_root) = std::env::var("BLIZZ_INSIGHTS_ROOT") {
    return Ok(PathBuf::from(custom_root));
  }

  let home = home_dir().ok_or_else(|| anyhow!("Could not find home directory"))?;
  Ok(home.join(".kernelle").join("insights"))
}

// Frontmatter parsing constants
const FRONTMATTER_START: &str = "---\n";
const FRONTMATTER_END: &str = "\n---\n";
const FRONTMATTER_START_LEN: usize = 4; // Length of "---\n"
const FRONTMATTER_END_LEN: usize = 5; // Length of "\n---\n"

/// Split content into frontmatter and body sections
fn split_frontmatter_content(content: &str) -> Result<(&str, &str)> {
  if !content.starts_with(FRONTMATTER_START) {
    return Err(anyhow!("Invalid insight format: missing frontmatter"));
  }

  let content_after_start = &content[FRONTMATTER_START_LEN..];
  if let Some(end_pos) = content_after_start.find(FRONTMATTER_END) {
    let frontmatter_section = &content_after_start[..end_pos];
    let body = &content_after_start[end_pos + FRONTMATTER_END_LEN..];
    Ok((frontmatter_section, body))
  } else {
    Err(anyhow!("Invalid insight format: could not find end of frontmatter"))
  }
}

/// Clean body content by removing empty lines and comments
fn clean_body_content(body: &str) -> String {
  body
    .lines()
    .skip_while(|line| line.trim().is_empty() || line.starts_with('#'))
    .collect::<Vec<_>>()
    .join("\n")
    .trim()
    .to_string()
}

/// Parse YAML frontmatter format
fn parse_yaml_format(frontmatter_section: &str, body: &str) -> Result<(FrontMatter, String)> {
  let frontmatter = serde_yaml::from_str::<FrontMatter>(frontmatter_section)?;
  let details = clean_body_content(body);
  Ok((frontmatter, details))
}

/// Parse legacy frontmatter format
fn parse_legacy_format(frontmatter_section: &str, body: &str) -> (FrontMatter, String) {
  let overview = frontmatter_section.trim().to_string();
  let details = body.trim().to_string();

  let frontmatter = FrontMatter {
    overview,
    embedding_version: None,
    embedding: None,
    embedding_text: None,
    embedding_computed: None,
  };

  (frontmatter, details)
}

/// Parse insight content from YAML frontmatter format (returning full metadata)
pub fn parse_insight_with_metadata(content: &str) -> Result<(FrontMatter, String)> {
  let (frontmatter_section, body) = split_frontmatter_content(content)?;

  // Try YAML format first, fall back to legacy format
  if let Ok(result) = parse_yaml_format(frontmatter_section, body) {
    Ok(result)
  } else {
    Ok(parse_legacy_format(frontmatter_section, body))
  }
}

/// Parse insight content from YAML frontmatter format (legacy compatibility)
pub fn parse_insight_content(content: &str) -> Result<(String, String)> {
  let (frontmatter, details) = parse_insight_with_metadata(content)?;
  Ok((frontmatter.overview, details))
}

/// List all available topics
pub fn get_topics() -> Result<Vec<String>> {
  let insights_root = get_insights_root()?;

  if !insights_root.exists() {
    return Ok(vec![]);
  }

  let mut topics = Vec::new();

  for entry in fs::read_dir(&insights_root)? {
    let entry = entry?;
    if entry.file_type()?.is_dir() {
      if let Some(name) = entry.file_name().to_str() {
        topics.push(name.to_string());
      }
    }
  }

  topics.sort();
  Ok(topics)
}

fn get_search_paths(topic_filter: Option<&str>) -> Result<Vec<std::path::PathBuf>> {
  let insights_root = get_insights_root()?;

  if let Some(topic) = topic_filter {
    return Ok(vec![insights_root.join(topic)]);
  }

  let paths = get_topics()?.into_iter().map(|topic| insights_root.join(topic)).collect();
  Ok(paths)
}

fn extract_topic_name(topic_path: &std::path::Path) -> &str {
  topic_path.file_name().and_then(|n| n.to_str()).unwrap_or("unknown")
}

fn is_insight_file(path: &std::path::Path) -> bool {
  if path.extension().and_then(|s| s.to_str()) != Some("md") {
    return false;
  }

  if let Some(file_stem) = path.file_stem().and_then(|s| s.to_str()) {
    return file_stem.ends_with(".insight");
  }

  false
}

fn extract_insight_name(path: &std::path::Path) -> Option<String> {
  let file_stem = path.file_stem()?.to_str()?;

  if !file_stem.ends_with(".insight") {
    return None;
  }

  Some(file_stem.trim_end_matches(".insight").to_string())
}

fn collect_insights_from_topic(topic_path: &std::path::Path) -> Result<Vec<(String, String)>> {
  if !topic_path.exists() {
    return Ok(Vec::new());
  }

  let topic_name = extract_topic_name(topic_path);
  let mut insights = Vec::new();

  for entry in fs::read_dir(topic_path)? {
    let path = entry?.path();

    if !is_insight_file(&path) {
      continue;
    }

    if let Some(insight_name) = extract_insight_name(&path) {
      insights.push((topic_name.to_string(), insight_name));
    }
  }

  Ok(insights)
}

/// List all insights, optionally filtered by topic
pub fn get_insights(topic_filter: Option<&str>) -> Result<Vec<(String, String)>> {
  let search_paths = get_search_paths(topic_filter)?;
  let mut all_insights = Vec::new();

  for topic_path in search_paths {
    let mut topic_insights = collect_insights_from_topic(&topic_path)?;
    all_insights.append(&mut topic_insights);
  }

  all_insights.sort();
  Ok(all_insights)
}
