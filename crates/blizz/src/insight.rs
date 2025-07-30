#[cfg(feature = "neural")]
use crate::embedding_client::Embedding;
use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use dirs::home_dir;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

// Frontmatter parsing constants
const FRONTMATTER_START: &str = "---\n";
const FRONTMATTER_END: &str = "\n---\n";
const FRONTMATTER_START_LEN: usize = 4; // Length of "---\n"
const FRONTMATTER_END_LEN: usize = 5; // Length of "\n---\n"

/// YAML frontmatter structure for insight files
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InsightMetaData {
  #[serde(default)]
  pub topic: String,
  #[serde(default)]
  pub name: String,
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
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
}

pub fn file_path(insight: &Insight) -> Result<PathBuf> {
  let insights_root = get_insights_root()?;
  // Normalize file paths for x-platform compatibility.
  // Original case is preserved in insight metadata.
  let normalized_topic = insight.topic.to_lowercase();
  let normalized_name = insight.name.to_lowercase();
  Ok(insights_root.join(&normalized_topic).join(format!("{normalized_name}.insight.md")))
}

#[cfg(feature = "neural")]
pub fn set_embedding(insight: &mut Insight, embedding: Embedding) {
  insight.embedding_version = Some(embedding.version);
  insight.embedding = Some(embedding.embedding);
  insight.embedding_computed = Some(embedding.created_at);
}

#[cfg(feature = "neural")]
pub fn has_embedding(insight: &Insight) -> bool {
  insight.embedding.is_some()
}

#[cfg(feature = "neural")]
pub fn get_embedding_text(insight: &Insight) -> String {
  format!("{} {} {} {}", insight.topic, insight.name, insight.overview, insight.details)
}

pub fn save(insight: &Insight) -> Result<()> {
  let file_path = file_path(insight)?;
  ensure_parent_dir_exists(&file_path)?;
  check_insight_is_new(&file_path, &insight.topic, &insight.name)?;
  write_to_file(insight, &file_path)
}

/// Save an insight, overwriting if it already exists (used for embedding updates)
#[cfg(feature = "neural")]
pub fn save_existing(insight: &Insight) -> Result<()> {
  let file_path = file_path(insight)?;
  write_to_file(insight, &file_path)
}

fn write_to_file(insight: &Insight, file_path: &PathBuf) -> Result<()> {
  ensure_parent_dir_exists(file_path)?;

  let frontmatter = InsightMetaData {
    topic: insight.topic.clone(),
    name: insight.name.clone(),
    overview: insight.overview.clone(),
    embedding_version: insight.embedding_version.clone(),
    embedding: insight.embedding.clone(),
    embedding_text: insight.embedding_text.clone(),
    embedding_computed: insight.embedding_computed,
  };

  let yaml_content = serde_yaml::to_string(&frontmatter)?;
  let content = format!("---\n{}---\n\n# Details\n{}", yaml_content, insight.details);
  fs::write(file_path, content)?;

  Ok(())
}

pub fn load(topic: &str, name: &str) -> Result<Insight> {
  let file_path = make_insight_path(topic, name)?;

  if !file_path.exists() {
    return Err(anyhow!("Insight {}/{} not found", topic, name));
  }

  let content = fs::read_to_string(&file_path)?;
  parse_insight_from_content(topic, name, &content)
}

pub fn load_from_path(path: &std::path::Path) -> Result<Insight> {
  let content = fs::read_to_string(path)?;
  parse_insight_from_content(
    path.parent().unwrap().file_name().unwrap().to_str().unwrap(),
    path.file_stem().unwrap().to_str().unwrap(),
    &content,
  )
}

pub fn update(
  insight: &mut Insight,
  new_overview: Option<&str>,
  new_details: Option<&str>,
) -> Result<()> {
  if let Some(overview) = new_overview {
    insight.overview = overview.to_string();
  }
  if let Some(details) = new_details {
    insight.details = details.to_string();
  }

  if new_overview.is_none() && new_details.is_none() {
    return Err(anyhow!("At least one of overview or details must be provided"));
  }

  let existing_file_path = make_insight_path(&insight.topic, &insight.name)?;
  if !existing_file_path.exists() {
    return Err(anyhow!("Insight {}/{} not found", insight.topic, insight.name));
  }

  let new_file_path = file_path(insight)?;

  // Gets recomputed lazily on next search.
  clear_embedding(insight);

  // Delete the existing file FIRST to ensure cross-platform compatibility.
  // Prevents issues on case-insensitive filesystems
  fs::remove_file(&existing_file_path)?;

  // Clean up empty directory from old location
  if let Some(parent) = existing_file_path.parent() {
    let _ = fs::remove_dir(parent);
  }

  // Now save to the normalized path
  write_to_file(insight, &new_file_path)?;

  Ok(())
}

pub fn clear_embedding(insight: &mut Insight) {
  insight.embedding_version = None;
  insight.embedding = None;
  insight.embedding_text = None;
  insight.embedding_computed = None;
}

pub fn delete(insight: &Insight) -> Result<()> {
  let file_path = file_path(insight)?;
  check_insight_exists(&file_path, &insight.topic, &insight.name)?;
  fs::remove_file(&file_path)?;
  cleanup_empty_dir(&file_path)?;
  Ok(())
}

pub fn get_insights_root() -> Result<PathBuf> {
  // Allow tests or callers to override the root directory via env var
  if let Ok(custom_root) = std::env::var("BLIZZ_INSIGHTS_ROOT") {
    return Ok(PathBuf::from(custom_root));
  }

  let home = home_dir().ok_or_else(|| anyhow!("Could not find home directory"))?;
  Ok(home.join(".kernelle").join("persistent").join("blizz").join("insights"))
}

pub fn get_valid_insights_dir() -> Result<std::path::PathBuf> {
  let insights_dir = get_insights_root()?;
  if !insights_dir.exists() {
    println!("No insights found. Create some insights first!");
    return Err(anyhow!("No insights directory found"));
  }
  Ok(insights_dir)
}

pub fn parse_insight_with_metadata(content: &str) -> Result<(InsightMetaData, String)> {
  let (frontmatter_section, body) = split_frontmatter_content(content)?;

  // Try YAML format first, fall back to legacy format
  if let Ok(result) = parse_yaml_format(frontmatter_section, body) {
    Ok(result)
  } else {
    Ok(parse_legacy_format(frontmatter_section, body))
  }
}

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

fn parse_yaml_format(frontmatter_section: &str, body: &str) -> Result<(InsightMetaData, String)> {
  let frontmatter = serde_yaml::from_str::<InsightMetaData>(frontmatter_section)?;
  let details = clean_body_content(body);
  Ok((frontmatter, details))
}

fn parse_legacy_format(frontmatter_section: &str, body: &str) -> (InsightMetaData, String) {
  let overview = frontmatter_section.trim().to_string();
  let details = body.trim().to_string();

  let frontmatter = InsightMetaData {
    topic: "".to_string(),
    name: "".to_string(),
    overview,
    embedding_version: None,
    embedding: None,
    embedding_text: None,
    embedding_computed: None,
  };

  (frontmatter, details)
}

fn clean_body_content(body: &str) -> String {
  body
    .lines()
    .skip_while(|line| line.trim().is_empty() || line.starts_with('#'))
    .collect::<Vec<_>>()
    .join("\n")
    .trim()
    .to_string()
}

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

pub fn get_insights(topic_filter: Option<&str>) -> Result<Vec<Insight>> {
  let search_paths = get_search_paths(topic_filter)?;
  let mut all_insights = Vec::new();

  for topic_path in search_paths {
    let mut topic_insights = collect_insights_from_topic(&topic_path)?;
    all_insights.append(&mut topic_insights);
  }

  all_insights.sort_by_key(|insight| insight.name.clone());
  Ok(all_insights)
}

fn get_search_paths(topic_filter: Option<&str>) -> Result<Vec<std::path::PathBuf>> {
  let insights_root = get_insights_root()?;

  if let Some(topic) = topic_filter {
    return Ok(vec![insights_root.join(topic)]);
  }

  let paths = get_topics()?.into_iter().map(|topic| insights_root.join(topic)).collect();
  Ok(paths)
}

fn collect_insights_from_topic(topic_path: &std::path::Path) -> Result<Vec<Insight>> {
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
      insights.push(load(topic_name, &insight_name)?);
    }
  }

  Ok(insights)
}

pub fn is_insight_file(path: &std::path::Path) -> bool {
  if path.extension().and_then(|s| s.to_str()) != Some("md") {
    return false;
  }

  if let Some(file_stem) = path.file_stem().and_then(|s| s.to_str()) {
    return file_stem.ends_with(".insight");
  }

  false
}

// Shared helper functions used by multiple public functions

fn make_insight_path(topic: &str, name: &str) -> Result<std::path::PathBuf> {
  let root = get_insights_root()?;

  // Try normalized case first.
  let normalized_topic = topic.to_lowercase();
  let normalized_name = name.to_lowercase();
  let normalized_path = root.join(&normalized_topic).join(format!("{normalized_name}.insight.md"));

  // If normalized path exists, use it
  if normalized_path.exists() {
    return Ok(normalized_path);
  }

  // Fallback to original case for backwards compatibility with legacy insights
  let legacy_path = root.join(topic).join(format!("{name}.insight.md"));
  if legacy_path.exists() {
    return Ok(legacy_path);
  }

  // If neither exists, return the normalized path (for error messages and new file creation)
  Ok(normalized_path)
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
    // Use topic and name from frontmatter to preserve original case.
    // Fall back to parameters for backward compatibility.
    topic: if !fm.topic.is_empty() { fm.topic } else { topic.to_string() },
    name: if !fm.name.is_empty() { fm.name } else { name.to_string() },
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

fn extract_topic_name(topic_path: &std::path::Path) -> &str {
  topic_path.file_name().and_then(|n| n.to_str()).unwrap_or("unknown")
}

fn extract_insight_name(path: &std::path::Path) -> Option<String> {
  let file_stem = path.file_stem()?.to_str()?;

  if !file_stem.ends_with(".insight") {
    return None;
  }

  Some(file_stem.trim_end_matches(".insight").to_string())
}
