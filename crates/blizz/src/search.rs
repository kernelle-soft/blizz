use anyhow::Result;
use colored::*;
use std::fs;
use std::path::{Path, PathBuf};
use clap::Args;

use crate::embedding_client;
use crate::insight;
use crate::semantic;
use crate::similarity;

// Semantic similarity threshold for meaningful results
#[cfg(feature = "semantic")]
const SEMANTIC_SIMILARITY_THRESHOLD: f32 = 0.2;

#[cfg(feature = "neural")]
const EMBEDDING_SIMILARITY_THRESHOLD: f32 = 0.2;

#[derive(Debug)]
pub struct SearchResult {
  pub topic: String,
  pub name: String,
  pub overview: String,
  pub details: String,
  pub score: f32, // number of matching terms
}

/// Search configuration options
#[derive(Args)]
pub struct SearchOptions {
  /// Optional topic to restrict search to
  #[arg(short, long)]
  topic: Option<String>,
  /// Case-sensitive search
  #[arg(short, long)]
  case_sensitive: bool,
  /// Search only in overview sections
  #[arg(short, long)]
  overview_only: bool,
  /// Use semantic + exact search only (drops neural for speed)
  #[cfg(feature = "semantic")]
  #[arg(short, long)]
  semantic: bool,
  /// Use exact term matching only (fastest, drops neural and semantic)
  #[arg(short, long)]
  exact: bool,
}

/// Check if embedding search feature can be used
pub fn can_use_embedding_search(options: &SearchOptions) -> bool {
  !options.semantic && !options.exact
}

/// Check if semantic search feature can be used
pub fn can_use_semantic_similarity_search(options: &SearchOptions) -> bool {
  !options.exact
}

pub fn search(terms: &[String], options: &SearchOptions) -> Result<()> {
  let mut results = Vec::new();
  results.extend(search_topic(terms, get_exact_match, 0.0, options)?);

  #[cfg(feature = "semantic")]
  if can_use_semantic_similarity_search(options) {
    results.extend(search_topic(terms, get_semantic_match, SEMANTIC_SIMILARITY_THRESHOLD, options)?);
  }

  #[cfg(feature = "neural")]
  if can_use_embedding_search(options) {
    results.extend(search_topic(terms, get_embedding_match, EMBEDDING_SIMILARITY_THRESHOLD, options)?);
  }

  // remove duplicates
  results.sort_by(
    |a, b| 
    b.score.partial_cmp(&a.score)
      .unwrap_or(std::cmp::Ordering::Equal)
      .then_with(|| a.topic.cmp(&b.topic)
      .then_with(|| a.name.cmp(&b.name)))
  );

  results.dedup_by(
    |a, b| a.topic == b.topic && a.name == b.name
  );

  display_results(&results, terms, options.overview_only);

  Ok(())
}

/// Search a topic for matches based on a search strategy
fn search_topic(terms: &[String], search_strategy: fn(&str, &[String]) -> f32, threshold: f32, options: &SearchOptions) -> Result<Vec<SearchResult>> {
  let mut results = Vec::new();

  let insights_dir = insight::get_valid_insights_dir()?;
  let search_paths = get_search_paths(&insights_dir, options.topic.as_deref())?;

  for topic_path in search_paths {
    let topic_name = topic_path.file_name().and_then(|n| n.to_str()).unwrap_or("unknown");

    for entry in fs::read_dir(&topic_path)? {
      let entry = entry?;
      let path = entry.path();

      if let Some(insight_name) = is_insight_file(&path) {
        let insight = insight::Insight::load(topic_name, insight_name)?;
        if let Ok(Some(result)) = search_insight(&insight, search_strategy, terms, threshold, options) {
          results.push(result);
        }
      }
    }
  }

  Ok(results)
}

fn search_insight(insight: &insight::Insight, search_strategy: fn(&str, &[String]) -> f32, terms: &[String], threshold: f32, options: &SearchOptions) -> Result<Option<SearchResult>> {
  let search_content = if options.overview_only {
    format!("{} {} {}", insight.topic, insight.name, insight.overview)
  } else {
    format!("{} {} {} {}", insight.topic, insight.name, insight.overview, insight.details)
  };

  let normalized_content = if options.case_sensitive {
    search_content.to_string()
  } else {
    search_content.to_lowercase()
  };

  let normalized_terms = if options.case_sensitive {
    terms.to_vec()
  } else {
    terms.iter().map(|t| t.to_lowercase()).collect::<Vec<String>>()
  };
  
  let score = search_strategy(&normalized_content, &normalized_terms);
  if score > threshold {
    Ok(Some(SearchResult {
      topic: insight.topic.to_string(),
      name: insight.name.to_string(),
      overview: insight.overview.to_string(),
      details: insight.details.to_string(),
      score: score,
    }))
  } else {
    Ok(None)
  }
}

fn get_exact_match(content: &str, terms: &[String]) -> f32 {
  terms
    .iter()
    .map(|term| content.matches(term).count())
    .sum::<usize>() as f32
}

#[cfg(feature = "semantic")]
fn get_semantic_match(content: &str, terms: &[String]) -> f32 {
  let extracted_terms = semantic::extract_words(&terms.join(" "));
  semantic::similarity(&extracted_terms, content)
}

#[cfg(feature = "neural")]
fn get_embedding_match(content: &str, terms: &[String]) -> f32 {
  let mut session = init_embedding_session().unwrap();
  let query_embedding = embedding_client::create_embedding(&mut session, &terms.join(" ")).unwrap();
  let content_embedding = embedding_client::create_embedding(&mut session, content).unwrap();
  similarity::cosine_similarity(&query_embedding, &content_embedding)
}

/// Check if a file path represents a valid insight file
fn is_insight_file(path: &Path) -> Option<&str> {
  if path.extension().and_then(|s| s.to_str()) == Some("md") {
    if let Some(file_stem) = path.file_stem().and_then(|s| s.to_str()) {
      if file_stem.ends_with(".insight") {
        return Some(file_stem.trim_end_matches(".insight"));
      }
    }
  }
  None
}

#[cfg(feature = "neural")]
fn init_embedding_session() -> Result<ort::session::Session> {
  use ort::session::{builder::GraphOptimizationLevel, Session};

  ort::init().with_name("blizz").commit()?;

  Session::builder()?
    .with_optimization_level(GraphOptimizationLevel::Level1)?
    .with_intra_threads(1)?
    .commit_from_url("https://cdn.pyke.io/0/pyke:ort-rs/example-models@0.0.0/all-MiniLM-L6-v2.onnx")
    .map_err(Into::into)
}

/// Build search paths based on topic filter
fn get_search_paths(insights_root: &Path, topic_filter: Option<&str>) -> Result<Vec<PathBuf>> {
  if let Some(topic) = topic_filter {
    Ok(vec![insights_root.join(topic)])
  } else {
    Ok(insight::get_topics()?.into_iter().map(|topic| insights_root.join(topic)).collect())
  }
}

/// Wrap text to fit within a specified width
fn wrap_text(text: &str, width: usize) -> Vec<String> {
  let mut lines = Vec::new();
  
  for paragraph in text.split('\n') {
    if paragraph.trim().is_empty() {
      lines.push(String::new());
      continue;
    }
    
    let words: Vec<&str> = paragraph.split_whitespace().collect();
    let mut current_line = String::new();
    
    for word in words {
      if current_line.is_empty() {
        current_line = word.to_string();
      } else if current_line.len() + 1 + word.len() <= width {
        current_line.push(' ');
        current_line.push_str(word);
      } else {
        lines.push(current_line);
        current_line = word.to_string();
      }
    }
    
    if !current_line.is_empty() {
      lines.push(current_line);
    }
  }
  
  lines
}

/// Display the combined search results
fn display_results(results: &[SearchResult], terms: &[String], overview_only: bool) {
  if results.is_empty() {
    println!("No matches found for: {}", terms.join(" ").yellow());
  } else {
    for result in results {
      display_single_result(&result, overview_only);
    }
  }
}

/// Display a single search result
fn display_single_result(result: &SearchResult, overview_only: bool) {
  let header = format!(
    "=== {}/{} ===",
    result.topic.blue().bold(),
    result.name.yellow().bold()
  );

  println!("{}", header);
        
  // Wrap and display the content with proper formatting
  let wrap_with = if header.len() < 80 { 80 } else { header.len() };

  let content = if overview_only {
    result.overview.to_string()
  } else {
    format!("{}\n\n{}", result.overview, result.details)
  };

  let wrapped_lines = wrap_text(&content, wrap_with);
  for line in wrapped_lines {
    println!("{}", line);
  }
  println!();
}
