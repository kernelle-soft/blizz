use anyhow::Result;
use clap::Args;
use colored::*;

use std::fs;
use std::path::{Path, PathBuf};

#[cfg(feature = "neural")]
use crate::embedding_client;
use crate::insight;
#[cfg(any(feature = "semantic", feature = "neural"))]
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
pub struct SearchCommandOptions {
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

pub struct SearchOptions {
  pub topic: Option<String>,
  pub case_sensitive: bool,
  pub overview_only: bool,
  #[cfg(feature = "semantic")]
  pub semantic: bool,
  pub exact: bool,
  #[cfg(feature = "neural")]
  pub embedding_client: embedding_client::EmbeddingClient,
}

impl SearchOptions {
  pub fn from(options: &SearchCommandOptions) -> Self {
    Self {
      topic: options.topic.clone(),
      case_sensitive: options.case_sensitive,
      overview_only: options.overview_only,
      #[cfg(feature = "semantic")]
      semantic: options.semantic,
      exact: options.exact,
      #[cfg(feature = "neural")]
      embedding_client: embedding_client::create(),
    }
  }
}

pub fn search(terms: &[String], options: &SearchOptions) -> Result<Vec<SearchResult>> {
  let mut results = Vec::new();

  if can_use_exact_search(options) {
    results.extend(search_topic(terms, get_exact_match, 0.0, options)?);
  }

  #[cfg(feature = "semantic")]
  if can_use_semantic_similarity_search(options) {
    results.extend(search_topic(
      terms,
      get_semantic_match,
      SEMANTIC_SIMILARITY_THRESHOLD,
      options,
    )?);
  }

  #[cfg(feature = "neural")]
  if can_use_embedding_search(options) {
    results.extend(search_topic(
      terms,
      get_embedding_match,
      EMBEDDING_SIMILARITY_THRESHOLD,
      options,
    )?);
  }

  results.sort_by(|a, b| {
    b.score
      .partial_cmp(&a.score)
      .unwrap_or(std::cmp::Ordering::Equal)
      .then_with(|| a.topic.cmp(&b.topic).then_with(|| a.name.cmp(&b.name)))
  });

  results.dedup_by(|a, b| a.topic == b.topic && a.name == b.name);

  Ok(results)
}

/// Check if exact search should be used (default behavior unless explicitly disabled)
fn can_use_exact_search(options: &SearchOptions) -> bool {
  // Don't run exact search when we want neural-only mode
  #[cfg(feature = "semantic")]
  if !options.semantic && !options.exact {
    false // Neural-only mode for testing
  } else {
    #[cfg(feature = "semantic")]
    { !options.semantic } // Run exact unless semantic-only mode
    #[cfg(not(feature = "semantic"))]
    { !options.exact }
  }
  #[cfg(not(feature = "semantic"))]
  !options.exact
}

/// Check if embedding search feature can be used
#[cfg(feature = "neural")]
fn can_use_embedding_search(options: &SearchOptions) -> bool {
  #[cfg(feature = "semantic")]
  { !options.semantic && !options.exact }
  #[cfg(not(feature = "semantic"))]
  { !options.exact }
}

/// Check if semantic search feature can be used
#[cfg(feature = "semantic")]
fn can_use_semantic_similarity_search(options: &SearchOptions) -> bool {
  !options.exact
}

/// Search a topic for matches based on a search strategy
fn search_topic(
  terms: &[String],
  search_strategy: fn(&insight::Insight, &[String], &SearchOptions) -> f32,
  threshold: f32,
  options: &SearchOptions,
) -> Result<Vec<SearchResult>> {
  let mut results = Vec::new();

  let insights_dir = insight::get_valid_insights_dir()?;
  let search_paths = get_search_paths(&insights_dir, options.topic.as_deref())?;

  for topic_path in search_paths {
    for entry in fs::read_dir(&topic_path)? {
      let entry = entry?;
      let path = entry.path();

      if insight::is_insight_file(&path) {
        let insight = insight::load_from_path(&path)?;
        if let Ok(Some(result)) =
          search_insight(&insight, search_strategy, terms, threshold, options)
        {
          results.push(result);
        }
      }
    }
  }

  Ok(results)
}

fn search_insight(
  insight: &insight::Insight,
  search_strategy: fn(&insight::Insight, &[String], &SearchOptions) -> f32,
  terms: &[String],
  threshold: f32,
  options: &SearchOptions,
) -> Result<Option<SearchResult>> {
  let score = search_strategy(insight, terms, options);
  if score > threshold {
    Ok(Some(SearchResult {
      topic: insight.topic.to_string(),
      name: insight.name.to_string(),
      overview: insight.overview.to_string(),
      details: insight.details.to_string(),
      score,
    }))
  } else {
    Ok(None)
  }
}

fn get_normalized_content(insight: &insight::Insight, options: &SearchOptions) -> String {
  if options.overview_only {
    format!("{} {} {}", insight.topic, insight.name, insight.overview)
  } else {
    format!("{} {} {} {}", insight.topic, insight.name, insight.overview, insight.details)
  }
}

fn get_normalized_terms(terms: &[String], options: &SearchOptions) -> Vec<String> {
  if options.case_sensitive {
    terms.to_vec()
  } else {
    terms.iter().map(|t| t.to_lowercase()).collect::<Vec<String>>()
  }
}

fn get_exact_match(insight: &insight::Insight, terms: &[String], options: &SearchOptions) -> f32 {
  let normalized_content = get_normalized_content(insight, options);
  let normalized_terms = get_normalized_terms(terms, options);

  normalized_terms.iter().map(|term| normalized_content.matches(term).count()).sum::<usize>() as f32
}

#[cfg(feature = "semantic")]
fn get_semantic_match(
  insight: &insight::Insight,
  terms: &[String],
  options: &SearchOptions,
) -> f32 {
  let normalized_content = get_normalized_content(insight, options);
  let normalized_terms = get_normalized_terms(terms, options);

  similarity::semantic(&normalized_terms.into_iter().collect(), &normalized_content)
}

#[cfg(feature = "neural")]
fn get_embedding_match(
  insight: &insight::Insight,
  terms: &[String],
  options: &SearchOptions,
) -> f32 {
  try_get_embedding(insight, terms, options).unwrap_or(0.0)
}

#[cfg(feature = "neural")]
fn try_get_embedding(
  insight: &insight::Insight,
  terms: &[String],
  options: &SearchOptions,
) -> Result<f32> {
  let client = &options.embedding_client;

  let normalized_terms = get_normalized_terms(terms, options);

  // Create a temporary insight for query embedding
  let mut query_insight = insight::Insight::new(
    "query".to_string(),
    "search_terms".to_string(),
    normalized_terms.join(" "),
    "".to_string(),
  );

  let query_embedding_obj = embedding_client::embed_insight(client, &mut query_insight);
  let query_embedding = query_embedding_obj.embedding;
  let content_embedding = if let Some(embedding) = insight.embedding.as_ref() {
    embedding.clone()
  } else {
    recompute_embedding(insight, options)?
  };

  Ok(similarity::cosine(&query_embedding, &content_embedding))
}

/// Recompute the embedding for an insight and save it to the file system.
#[cfg(feature = "neural")]
fn recompute_embedding(insight: &insight::Insight, options: &SearchOptions) -> Result<Vec<f32>> {
  let normalized_content = get_normalized_content(insight, options);

  // Create a temporary insight for embedding computation
  let mut temp_insight = insight::Insight::new(
    insight.topic.clone(),
    insight.name.clone(),
    insight.overview.clone(),
    normalized_content,
  );

  let embedding = embedding_client::embed_insight(&options.embedding_client, &mut temp_insight);

  // Lazily recompute and save embedding.
  let mut to_save = insight.clone();
  insight::set_embedding(&mut to_save, embedding.clone());
  insight::save_existing(&to_save)?;
  Ok(embedding.embedding)
}

/// Highlight search terms in text using colors
fn highlight_keywords(text: &str, terms: &[String]) -> String {
  let mut result = text.to_string();
  
  // Sort terms by length (longest first) to avoid partial replacements
  let mut sorted_terms = terms.to_vec();
  sorted_terms.sort_by(|a, b| b.len().cmp(&a.len()));
  
  for term in sorted_terms {
    if term.is_empty() {
      continue;
    }
    
    // Use case-insensitive matching for highlighting
    let term_lower = term.to_lowercase();
    let mut highlighted = String::new();
    let mut last_end = 0;
    
    // Find all occurrences of the term (case-insensitive)
    let result_lower = result.to_lowercase();
    let mut start = 0;
    
    while let Some(pos) = result_lower[start..].find(&term_lower) {
      let absolute_pos = start + pos;
      
      // Add the text before the match
      highlighted.push_str(&result[last_end..absolute_pos]);
      
      // Add the highlighted match (preserve original case)
      let match_text = &result[absolute_pos..absolute_pos + term.len()];
      highlighted.push_str(&match_text.yellow().bold().to_string());
      
      last_end = absolute_pos + term.len();
      start = last_end;
    }
    
    // Add any remaining text
    highlighted.push_str(&result[last_end..]);
    result = highlighted;
  }
  
  result
}

/// Build search paths based on topic filter
fn get_search_paths(insights_root: &Path, topic_filter: Option<&str>) -> Result<Vec<PathBuf>> {
  if let Some(topic) = topic_filter {
    Ok(vec![insights_root.join(topic)])
  } else {
    Ok(insight::get_topics()?.into_iter().map(|topic| insights_root.join(topic)).collect())
  }
}

/// Display the combined search results
pub fn display_results(results: &[SearchResult], terms: &[String], overview_only: bool) {
  if results.is_empty() {
    println!("No matches found for: {}", terms.join(" ").yellow());
  } else {
    for result in results {
      display_single_result(result, terms, overview_only);
    }
  }
}

/// Display a single search result with keyword highlighting
fn display_single_result(result: &SearchResult, terms: &[String], overview_only: bool) {
  let header = format!("=== {}/{} ===", result.topic.blue().bold(), result.name.yellow().bold());

  println!("{header}");

  // Wrap and display the content with proper formatting
  let wrap_with = if header.len() < 80 { 80 } else { header.len() };

  let content = if overview_only {
    result.overview.to_string()
  } else {
    format!("{}\n\n{}", result.overview, result.details)
  };

  let highlighted_content = highlight_keywords(&content, terms);
  let wrapped_lines = wrap_text(&highlighted_content, wrap_with);
  for line in wrapped_lines {
    println!("{line}");
  }
  println!();
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
