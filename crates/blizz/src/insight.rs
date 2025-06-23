use anyhow::{anyhow, Result};
use std::fs;
use std::path::PathBuf;
use dirs::home_dir;

#[derive(Debug, Clone)]
pub struct Insight {
    pub topic: String,
    pub name: String,
    pub overview: String,
    pub details: String,
}

impl Insight {
    pub fn new(topic: String, name: String, overview: String, details: String) -> Self {
        Self {
            topic,
            name,
            overview,
            details,
        }
    }

    /// Get the file path for this insight
    pub fn file_path(&self) -> Result<PathBuf> {
        let insights_root = get_insights_root()?;
        Ok(insights_root.join(&self.topic).join(format!("{}.insight.md", self.name)))
    }

    /// Save this insight to disk
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

        // Write the insight file with the markdown format
        let content = format!("---\n{}\n---\n\n{}", self.overview, self.details);
        fs::write(&file_path, content)?;

        Ok(())
    }

    /// Load an insight from disk
    pub fn load(topic: &str, name: &str) -> Result<Self> {
        let insights_root = get_insights_root()?;
        let file_path = insights_root.join(topic).join(format!("{}.insight.md", name));

        if !file_path.exists() {
            return Err(anyhow!("Insight {}/{} not found", topic, name));
        }

        let content = fs::read_to_string(&file_path)?;
        let (overview, details) = parse_insight_content(&content)?;

        Ok(Insight::new(
            topic.to_string(),
            name.to_string(),
            overview,
            details,
        ))
    }

    /// Update this insight on disk
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

        // Write the updated content
        let content = format!("---\n{}\n---\n\n{}", self.overview, self.details);
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

/// Parse insight content from markdown format
pub fn parse_insight_content(content: &str) -> Result<(String, String)> {
    let lines: Vec<&str> = content.lines().collect();
    
    // Find the overview section (between first two ---)
    let mut overview_start = None;
    let mut overview_end = None;
    let mut dash_count = 0;
    
    for (i, line) in lines.iter().enumerate() {
        if line.trim() == "---" {
            dash_count += 1;
            if dash_count == 1 {
                overview_start = Some(i + 1);
            } else if dash_count == 2 {
                overview_end = Some(i);
                break;
            }
        }
    }

    let overview = if let (Some(start), Some(end)) = (overview_start, overview_end) {
        lines[start..end].join("\n").trim().to_string()
    } else {
        return Err(anyhow!("Invalid insight format: could not find overview section"));
    };

    // Everything after the second --- is details
    let details = if let Some(end) = overview_end {
        if end + 2 < lines.len() {
            lines[end + 2..].join("\n").trim().to_string()
        } else {
            String::new()
        }
    } else {
        String::new()
    };

    Ok((overview, details))
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
        get_topics()?
            .into_iter()
            .map(|topic| insights_root.join(topic))
            .collect()
    };

    for topic_path in search_paths {
        if !topic_path.exists() {
            continue;
        }

        let topic_name = topic_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown");

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