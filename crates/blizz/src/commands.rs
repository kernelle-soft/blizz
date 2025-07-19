use anyhow::{anyhow, Result};
use colored::*;
use std::fs;
use std::path::Path;
#[cfg(feature = "semantic")]
use std::collections::HashSet;

use crate::insight::*;

#[derive(Debug)]
struct SearchResult {
  topic: String,
  name: String,
  matching_lines: Vec<String>,
  score: usize, // number of matching terms
}

#[cfg(feature = "semantic")]
#[derive(Debug)]
struct SemanticSearchResult {
  topic: String,
  name: String,
  content: String,
  similarity: f32, // semantic similarity score
}

/// Creates a cross-platform symlink/junction
fn xplat_symlink(src: &Path, dst: &Path) -> Result<()> {
  #[cfg(unix)]
  {
    std::os::unix::fs::symlink(src, dst).map_err(Into::into)
  }

  #[cfg(windows)]
  {
    // On Windows, try symlink_file first, fall back to copying if it fails
    // (symlinks require admin privileges on Windows)
    match std::os::windows::fs::symlink_file(src, dst) {
      Ok(()) => Ok(()),
      Err(_) => {
        // Fall back to copying the file
        std::fs::copy(src, dst).map(|_| ()).map_err(Into::into)
      }
    }
  }
}

/// Add a new insight to the knowledge base
pub fn add_insight(topic: &str, name: &str, overview: &str, details: &str) -> Result<()> {
  let insight =
    Insight::new(topic.to_string(), name.to_string(), overview.to_string(), details.to_string());

  insight.save()?;

  println!("{} Added insight {}/{}", "✓".green(), topic.cyan(), name.yellow());
  Ok(())
}

/// Search through all insights for matching content
#[cfg(feature = "semantic")]
pub fn search_insights(
  terms: &[String],
  topic_filter: Option<&str>,
  case_sensitive: bool,
  overview_only: bool,
  semantic: bool,
) -> Result<()> {
  if semantic {
    search_insights_semantic(terms, topic_filter, overview_only)
  } else {
    search_insights_exact(terms, topic_filter, case_sensitive, overview_only)
  }
}

/// Search through all insights for matching content (non-semantic version)
#[cfg(not(feature = "semantic"))]
pub fn search_insights(
  terms: &[String],
  topic_filter: Option<&str>,
  case_sensitive: bool,
  overview_only: bool,
) -> Result<()> {
  search_insights_exact(terms, topic_filter, case_sensitive, overview_only)
}

/// Exact term matching search (the original implementation)
fn search_insights_exact(
  terms: &[String],
  topic_filter: Option<&str>,
  case_sensitive: bool,
  overview_only: bool,
) -> Result<()> {
  let insights_root = get_insights_root()?;

  if !insights_root.exists() {
    println!("No insights directory found");
    return Ok(());
  }

  let search_paths = if let Some(topic) = topic_filter {
    vec![insights_root.join(topic)]
  } else {
    get_topics()?.into_iter().map(|topic| insights_root.join(topic)).collect()
  };

  let mut results = Vec::new();

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

            // Read and search the file content
            if let Ok(content) = fs::read_to_string(&path) {
              let search_content = if overview_only {
                // Extract just the overview section
                if let Ok((overview, _)) = parse_insight_content(&content) {
                  overview
                } else {
                  continue;
                }
              } else {
                content
              };

              // Count matches for each term
              let mut score = 0;
              let mut matching_lines = Vec::new();
              
              for term in terms {
                let term_matches = if case_sensitive {
                  search_content.contains(term)
                } else {
                  search_content.to_lowercase().contains(&term.to_lowercase())
                };
                
                if term_matches {
                  score += 1;
                }
              }
              
              // If any terms matched, collect matching lines
              if score > 0 {
                for line in search_content.lines() {
                  let mut line_has_match = false;
                  for term in terms {
                    let line_matches = if case_sensitive {
                      line.contains(term)
                    } else {
                      line.to_lowercase().contains(&term.to_lowercase())
                    };
                    
                    if line_matches {
                      line_has_match = true;
                      break;
                    }
                  }
                  
                  if line_has_match {
                    matching_lines.push(line.to_string());
                  }
                }
                
                results.push(SearchResult {
                  topic: topic_name.to_string(),
                  name: insight_name.to_string(),
                  matching_lines,
                  score,
                });
              }
            }
          }
        }
      }
    }
  }

  // Sort by relevance score (descending), then by topic/name for consistency
  results.sort_by(|a, b| {
    b.score.cmp(&a.score).then_with(|| {
      a.topic.cmp(&b.topic).then_with(|| a.name.cmp(&b.name))
    })
  });

  if results.is_empty() {
    let terms_str = terms.join(" ");
    println!("No matches found for: {}", terms_str.yellow());
  } else {
    for result in results {
      println!("=== {}/{} ({} terms) ===", 
               result.topic.cyan(), 
               result.name.yellow(),
               result.score.to_string().green());
      
      for line in result.matching_lines {
        println!("{line}");
      }
      println!();
    }
  }

  Ok(())
}

/// Semantic similarity search using advanced text analysis
#[cfg(feature = "semantic")]
fn search_insights_semantic(
  terms: &[String],
  topic_filter: Option<&str>,
  overview_only: bool,
) -> Result<()> {
  // Combine search terms into a single query
  let query = terms.join(" ");
  
  let insights_dir = get_insights_root()?;
  if !insights_dir.exists() {
    println!("No insights found. Create some insights first!");
    return Ok(());
  }

  let mut results = Vec::new();
  let query_words = extract_words(&query.to_lowercase());
  
  // Process all insights
  for entry in fs::read_dir(&insights_dir)? {
    let entry = entry?;
    let topic_path = entry.path();
    
    if !topic_path.is_dir() {
      continue;
    }
    
    let topic_name = topic_path.file_name()
      .and_then(|name| name.to_str())
      .unwrap_or("unknown");
    
    // Apply topic filter
    if let Some(filter) = topic_filter {
      if topic_name != filter {
        continue;
      }
    }
    
    // Process insights in this topic
    for insight_entry in fs::read_dir(&topic_path)? {
      let insight_entry = insight_entry?;
      let insight_path = insight_entry.path();
      
      if insight_path.extension().and_then(|s| s.to_str()) != Some("md") {
        continue;
      }
      
      let insight_name = insight_path.file_stem()
        .and_then(|name| name.to_str())
        .unwrap_or("unknown");
      
      let content = fs::read_to_string(&insight_path)?;
      let (overview, details) = parse_insight_content(&content)?;
      
      // Determine what text to search
      let search_text = if overview_only {
        overview.clone()
      } else {
        format!("{} {}", overview, details)
      };
      
      // Calculate semantic similarity
      let similarity = calculate_semantic_similarity(&query_words, &search_text);
      
      // Only include results with meaningful similarity (> 0.1)
      if similarity > 0.1 {
        results.push(SemanticSearchResult {
          topic: topic_name.to_string(),
          name: insight_name.to_string(),
          content: if overview_only { overview } else { format!("{}\n\n{}", overview, details) },
          similarity,
        });
      }
    }
  }
  
  // Sort by similarity (descending), then alphabetically
  results.sort_by(|a, b| {
    b.similarity.partial_cmp(&a.similarity)
      .unwrap_or(std::cmp::Ordering::Equal)
      .then(a.topic.cmp(&b.topic))
      .then(a.name.cmp(&b.name))
  });
  
  // Display results in EXACT same format as regular search
  if results.is_empty() {
    println!("No matches found for: {}", terms.join(" "));
  } else {
    for result in results {
      // Format exactly like regular search
      println!("=== {}/{} ===", 
        result.topic.cyan(), 
        result.name.yellow()
      );
      
      // Show content same as regular search - split into lines and show first several
      let lines: Vec<&str> = result.content.lines().collect();
      for line in lines.iter() {
        if !line.trim().is_empty() && !line.starts_with("---") {
          println!("{}", line);
        }
      }
      println!();
    }
  }
  
  Ok(())
}

/// Extract words from text, filtering out common stop words
#[cfg(feature = "semantic")]
fn extract_words(text: &str) -> HashSet<String> {
  let stop_words: HashSet<&str> = ["the", "a", "an", "and", "or", "but", "in", "on", "at", "to", "for", "of", "with", "by", "is", "are", "was", "were", "be", "been", "have", "has", "had", "do", "does", "did", "will", "would", "could", "should"].iter().cloned().collect();
  
  text.split_whitespace()
    .map(|word| word.trim_matches(|c: char| !c.is_alphanumeric()).to_lowercase())
    .filter(|word| !word.is_empty() && !stop_words.contains(word.as_str()))
    .collect()
}

/// Calculate semantic similarity using Jaccard + frequency analysis
#[cfg(feature = "semantic")]
fn calculate_semantic_similarity(query_words: &HashSet<String>, content: &str) -> f32 {
  let content_words = extract_words(&content.to_lowercase());
  
  if query_words.is_empty() || content_words.is_empty() {
    return 0.0;
  }
  
  // Jaccard similarity (intersection over union)
  let intersection: HashSet<_> = query_words.intersection(&content_words).collect();
  let union: HashSet<_> = query_words.union(&content_words).collect();
  let jaccard = intersection.len() as f32 / union.len() as f32;
  
  // Frequency boost for repeated terms
  let mut frequency_score = 0.0;
  let content_lower = content.to_lowercase();
  for query_word in query_words {
    let count = content_lower.matches(query_word).count();
    frequency_score += (count as f32).ln_1p(); // Natural log for diminishing returns
  }
  frequency_score /= query_words.len() as f32;
  
  // Combined score: 60% Jaccard + 40% frequency
  (jaccard * 0.6) + (frequency_score.min(1.0) * 0.4)
}

/// Get content of a specific insight
pub fn get_insight(topic: &str, name: &str, overview_only: bool) -> Result<()> {
  let insight = Insight::load(topic, name)?;

  if overview_only {
    println!("{}", insight.overview);
  } else {
    println!("---\n{}\n---\n\n{}", insight.overview, insight.details);
  }

  Ok(())
}

/// List insights in a topic or all topics
pub fn list_insights(topic_filter: Option<&str>, verbose: bool) -> Result<()> {
  let insights = get_insights(topic_filter)?;

  if insights.is_empty() {
    if let Some(topic) = topic_filter {
      println!("No insights found in topic: {}", topic.yellow());
    } else {
      println!("No insights found");
    }
    return Ok(());
  }

  for (topic, name) in insights {
    if verbose {
      if let Ok(insight) = Insight::load(&topic, &name) {
        println!(
          "{}/{}: {}",
          topic.cyan(),
          name.yellow(),
          insight.overview.trim().replace('\n', " ")
        );
      }
    } else {
      println!("{}/{}", topic.cyan(), name.yellow());
    }
  }

  Ok(())
}

/// Update an existing insight
pub fn update_insight(
  topic: &str,
  name: &str,
  new_overview: Option<&str>,
  new_details: Option<&str>,
) -> Result<()> {
  let mut insight = Insight::load(topic, name)?;
  insight.update(new_overview, new_details)?;

  println!("{} Updated insight {}/{}", "✓".green(), topic.cyan(), name.yellow());
  Ok(())
}

/// Create a link from one insight to another topic
pub fn link_insight(
  src_topic: &str,
  src_name: &str,
  target_topic: &str,
  target_name: Option<&str>,
) -> Result<()> {
  let insights_root = get_insights_root()?;
  let target_name = target_name.unwrap_or(src_name);

  let src_path = insights_root.join(src_topic).join(format!("{src_name}.insight.md"));
  let target_dir = insights_root.join(target_topic);
  let target_path = target_dir.join(format!("{target_name}.insight.md"));

  // Check if source insight exists
  if !src_path.exists() {
    return Err(anyhow!("Source insight {}/{} not found", src_topic, src_name));
  }

  // Create target directory if it doesn't exist
  fs::create_dir_all(&target_dir)?;

  // Create the symbolic link (cross-platform)
  xplat_symlink(&src_path, &target_path)?;

  println!(
    "{} Created link: {}/{} -> {}/{}",
    "✓".green(),
    target_topic.cyan(),
    target_name.yellow(),
    src_topic.cyan(),
    src_name.yellow()
  );

  Ok(())
}

/// Delete an insight
pub fn delete_insight(topic: &str, name: &str, force: bool) -> Result<()> {
  let insight = Insight::load(topic, name)?;

  if !force {
    println!("Are you sure you want to delete {}/{}? [y/N]", topic.cyan(), name.yellow());

    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;

    if !input.trim().to_lowercase().starts_with('y') {
      println!("Deletion cancelled");
      return Ok(());
    }
  }

  insight.delete()?;

  println!("{} Deleted insight {}/{}", "✓".green(), topic.cyan(), name.yellow());
  Ok(())
}

/// List all available topics
pub fn list_topics() -> Result<()> {
  let topics = get_topics()?;

  if topics.is_empty() {
    println!("No topics found");
    return Ok(());
  }

  for topic in topics {
    println!("{}", topic.cyan());
  }

  Ok(())
}
