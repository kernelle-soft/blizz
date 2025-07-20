use anyhow::{anyhow, Result};
use dirs::home_dir;
use std::fs;
use std::path::PathBuf;
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

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
  pub embedding_text: Option<String>,  // The exact text that was embedded
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
  
  /// Create a new insight with embedding metadata
  pub fn new_with_embedding(
    topic: String, 
    name: String, 
    overview: String, 
    details: String,
    embedding_version: String,
    embedding: Vec<f32>,
    embedding_text: String,
  ) -> Self {
    Self {
      topic,
      name,
      overview,
      details,
      embedding_version: Some(embedding_version),
      embedding: Some(embedding),
      embedding_text: Some(embedding_text),
      embedding_computed: Some(Utc::now()),
    }
  }
  
  /// Update embedding metadata for this insight
  pub fn set_embedding(&mut self, version: String, embedding: Vec<f32>, text: String) {
    self.embedding_version = Some(version);
    self.embedding = Some(embedding);
    self.embedding_text = Some(text);
    self.embedding_computed = Some(Utc::now());
  }
  
  /// Check if this insight has cached embedding
  pub fn has_embedding(&self) -> bool {
    self.embedding.is_some()
  }
  
  /// Get the text that should be embedded for this insight
  pub fn get_embedding_text(&self) -> String {
    format!("{} {} {} {}", self.topic, self.name, self.overview, self.details)
  }

  /// Get the file path for this insight
  pub fn file_path(&self) -> Result<PathBuf> {
    let insights_root = get_insights_root()?;
    Ok(insights_root.join(&self.topic).join(format!("{}.insight.md", self.name)))
  }

  /// Save this insight to disk using YAML frontmatter format
  pub fn save(&self) -> Result<()> {
    let file_path = self.file_path()?;

    // Create the topic directory if it doesn't exist
    if let Some(parent) = file_path.parent() {
      fs::create_dir_all(parent)?;
    }

    // Check if insight already exists
    if file_path.exists() {
      return Err(anyhow!("Insight {}/{} already exists", self.topic, self.name));
    }

    // Create frontmatter with overview and optional embedding metadata
    let frontmatter = FrontMatter {
      overview: self.overview.clone(),
      embedding_version: self.embedding_version.clone(),
      embedding: self.embedding.clone(),
      embedding_text: self.embedding_text.clone(),
      embedding_computed: self.embedding_computed,
    };

    // Serialize frontmatter to YAML
    let yaml_content = serde_yaml::to_string(&frontmatter)?;
    
    // Write the insight file with YAML frontmatter + markdown body
    let content = format!("---\n{}---\n\n# Details\n{}", yaml_content, self.details);
    fs::write(&file_path, content)?;

    Ok(())
  }

  /// Load an insight from disk
  pub fn load(topic: &str, name: &str) -> Result<Self> {
    let insights_root = get_insights_root()?;
    let file_path = insights_root.join(topic).join(format!("{name}.insight.md"));

    if !file_path.exists() {
      return Err(anyhow!("Insight {}/{} not found", topic, name));
    }

    let content = fs::read_to_string(&file_path)?;
    let (frontmatter, details) = parse_insight_with_metadata(&content)?;

    Ok(Insight {
      topic: topic.to_string(),
      name: name.to_string(),
      overview: frontmatter.overview,
      details,
      embedding_version: frontmatter.embedding_version,
      embedding: frontmatter.embedding,
      embedding_text: frontmatter.embedding_text,
      embedding_computed: frontmatter.embedding_computed,
    })
  }

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
    // (will be recomputed in write operations)
    if new_overview.is_some() || new_details.is_some() {
      self.embedding_version = None;
      self.embedding = None;
      self.embedding_text = None;
      self.embedding_computed = None;
    }

    // Create frontmatter with updated content
    let frontmatter = FrontMatter {
      overview: self.overview.clone(),
      embedding_version: self.embedding_version.clone(),
      embedding: self.embedding.clone(),
      embedding_text: self.embedding_text.clone(),
      embedding_computed: self.embedding_computed,
    };

    // Serialize frontmatter to YAML
    let yaml_content = serde_yaml::to_string(&frontmatter)?;
    
    // Write the updated content
    let content = format!("---\n{}---\n\n# Details\n{}", yaml_content, self.details);
    fs::write(&file_path, content)?;

    Ok(())
  }

  /// Delete this insight from disk
  pub fn delete(&self) -> Result<()> {
    let file_path = self.file_path()?;

    if !file_path.exists() {
      return Err(anyhow!("Insight {}/{} not found", self.topic, self.name));
    }

    fs::remove_file(&file_path)?;

    // Clean up empty topic directory
    if let Some(topic_dir) = file_path.parent() {
      if topic_dir.read_dir()?.next().is_none() {
        fs::remove_dir(topic_dir)?;
      }
    }

    Ok(())
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

/// Parse insight content from YAML frontmatter format (returning full metadata)
pub fn parse_insight_with_metadata(content: &str) -> Result<(FrontMatter, String)> {
  // Split content into frontmatter and body
  if !content.starts_with("---\n") {
    return Err(anyhow!("Invalid insight format: missing frontmatter"));
  }

  // Find the end of frontmatter
  let content_after_first_dash = &content[4..]; // Skip initial "---\n"
  if let Some(end_pos) = content_after_first_dash.find("\n---\n") {
    let frontmatter_section = &content_after_first_dash[..end_pos];
    let body = &content_after_first_dash[end_pos + 5..]; // Skip "\n---\n"
    
    // Try to parse as YAML first (new format)
    if let Ok(frontmatter) = serde_yaml::from_str::<FrontMatter>(frontmatter_section) {
      // New YAML format
      let details = body
        .lines()
        .skip_while(|line| line.trim().is_empty() || line.starts_with('#'))
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_string();
      
      Ok((frontmatter, details))
    } else {
      // Legacy format: frontmatter_section is the overview, body is the details
      let overview = frontmatter_section.trim().to_string();
      let details = body.trim().to_string();
      
      // Create FrontMatter structure for legacy format (no embeddings)
      let frontmatter = FrontMatter {
        overview,
        embedding_version: None,
        embedding: None,
        embedding_text: None,
        embedding_computed: None,
      };
      
      Ok((frontmatter, details))
    }
  } else {
    Err(anyhow!("Invalid insight format: could not find end of frontmatter"))
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

/// List all insights, optionally filtered by topic
pub fn get_insights(topic_filter: Option<&str>) -> Result<Vec<(String, String)>> {
  let insights_root = get_insights_root()?;
  let mut insights = Vec::new();

  let search_paths = if let Some(topic) = topic_filter {
    vec![insights_root.join(topic)]
  } else {
    get_topics()?.into_iter().map(|topic| insights_root.join(topic)).collect()
  };

  for topic_path in search_paths {
    if !topic_path.exists() {
      continue;
    }

    let topic_name = topic_path.file_name().and_then(|n| n.to_str()).unwrap_or("unknown");

    for entry in fs::read_dir(&topic_path)? {
      let entry = entry?;
      let path = entry.path();

      if path.extension().and_then(|s| s.to_str()) == Some("md") {
        if let Some(file_stem) = path.file_stem().and_then(|s| s.to_str()) {
          if file_stem.ends_with(".insight") {
            let insight_name = file_stem.trim_end_matches(".insight");
            insights.push((topic_name.to_string(), insight_name.to_string()));
          }
        }
      }
    }
  }

  insights.sort();
  Ok(insights)
}
