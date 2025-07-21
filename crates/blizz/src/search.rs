use anyhow::{anyhow, Result};
use colored::*;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use clap::Args;

use crate::embedding_client;
use crate::insight::*;
use crate::semantic;
use crate::similarity;

#[derive(Debug)]
pub struct SearchResult {
  pub topic: String,
  pub name: String,
  pub content: String,
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

pub fn can_use_neural_search(options: &SearchOptions) -> bool {
  cfg!(feature = "neural") && !options.semantic && !options.exact
}

pub fn can_use_semantic_search(options: &SearchOptions) -> bool {
  cfg!(feature = "semantic") && !options.exact
}

pub fn search(terms: &[String], options: &SearchOptions) -> Result<()> {
  let mut results = Vec::new();
  results.extend(find_exact_matches(terms, options.topic.as_deref(), options.case_sensitive, options.overview_only)?);

  if can_use_semantic_search(options) {
    results.extend(find_semantic_matches(terms, options)?);
  }

  if can_use_neural_search(options) {
    results.extend(find_embedding_matches(terms, options.topic.as_deref(), options.case_sensitive, options.overview_only)?);
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
    |a, b| 
    a.topic == b.topic && a.name == b.name
  );

  display_results(&results, terms);

  Ok(())
}

fn find_semantic_matches(terms: &[String], options: &SearchOptions) -> Result<Vec<SearchResult>> {
  if !can_use_semantic_search(options) {
    return Ok(Vec::new());
  } 

  let insights_dir = get_insights_dir()?;
  let query = terms.join(" ");
  let query_words = semantic::extract_words(&query);

  let results = process_all_topics_semantic(&insights_dir, &query_words, options.topic.as_deref(), options.overview_only)?;
  Ok(results)
}

/// Extract searchable content from insight file
fn extract_searchable_content(content: &str, overview_only: bool) -> Result<String> {
  if overview_only {
    let (overview, _) = parse_insight_content(content)?;
    Ok(overview)
  } else {
    Ok(content.to_string())
  }
}

fn calc_term_matches(
  content: &str,
  terms: &[String],
  case_sensitive: bool,
) -> f32 {
  let mut score = 0;

  let content = if case_sensitive {
    content.to_string()
  } else {
    content.to_lowercase()
  };

  for term in terms {
    let term = if case_sensitive {
      term.to_string()
    } else {
      term.to_lowercase()
    };
    
    score += content.matches(&term).count();
  }

  score as f32
}

// Semantic similarity threshold for meaningful results
#[cfg(feature = "semantic")]
const SEMANTIC_SIMILARITY_THRESHOLD: f32 = 0.2;

/// Extract insight name from file path safely
#[cfg(feature = "semantic")]
fn extract_insight_name_from_path(insight_path: &Path) -> &str {
  insight_path.file_stem().and_then(|name| name.to_str()).unwrap_or("unknown")
}

/// Build search text including topic and insight names for better matching
#[cfg(feature = "semantic")]
fn build_search_text(
  topic_name: &str,
  insight_name: &str,
  overview: &str,
  details: &str,
  overview_only: bool,
) -> String {
  if overview_only {
    format!("{topic_name} {insight_name} {overview}")
  } else {
    format!("{topic_name} {insight_name} {overview} {details}")
  }
}

/// Calculate semantic search result for a single insight
#[cfg(feature = "semantic")]
fn calculate_single_insight_semantic(
  topic_name: &str,
  insight_path: &Path,
  query_words: &HashSet<String>,
  overview_only: bool,
) -> Result<Option<SearchResult>> {
  let insight_name = extract_insight_name_from_path(insight_path);
  let content = fs::read_to_string(insight_path)?;
  let (overview, details) = parse_insight_content(&content)?;

  let search_text = build_search_text(topic_name, insight_name, &overview, &details, overview_only);
  let similarity = semantic::similarity(query_words, &search_text);

  if similarity > SEMANTIC_SIMILARITY_THRESHOLD {
    Ok(Some(SearchResult {
      topic: topic_name.to_string(),
      name: insight_name.to_string(),
      content: if overview_only {
        overview.to_string()
      } else {
        format!("{overview}\n\n{details}")
      },
      score: similarity,
    }))
  } else {
    Ok(None)
  }
}

#[cfg(feature = "semantic")]
fn process_topic_semantic(
  topic_path: &Path,
  query_words: &HashSet<String>,
  overview_only: bool,
) -> Result<Vec<SearchResult>> {
  let mut results = Vec::new();
  let name = topic_path.file_name().and_then(|name| name.to_str()).unwrap_or("unknown");

  for entry in fs::read_dir(topic_path)? {
    let entry = entry?;
    let path = entry.path();

    if path.extension().and_then(|s| s.to_str()) != Some("md") {
      continue;
    }

    if let Ok(Some(result)) =
      calculate_single_insight_semantic(name, &path, query_words, overview_only)
    {
      results.push(result);
    }
  }

  Ok(results)
}

/// Validate insights directory exists, return early if not
#[cfg(feature = "semantic")]
fn get_insights_dir() -> Result<std::path::PathBuf> {
  let insights_dir = get_insights_root()?;
  if !insights_dir.exists() {
    println!("No insights found. Create some insights first!");
    return Err(anyhow!("No insights directory found"));
  }
  Ok(insights_dir)
}

/// Process semantic search across all relevant topics
#[cfg(feature = "semantic")]
fn process_all_topics_semantic(
  insights_dir: &std::path::Path,
  query_words: &HashSet<String>,
  topic_filter: Option<&str>,
  overview_only: bool,
) -> Result<Vec<SearchResult>> {
  let mut results = Vec::new();

  for entry in fs::read_dir(insights_dir)? {
    let entry = entry?;
    let topic_path = entry.path();

    if !topic_path.is_dir() {
      continue;
    }

    let topic_name = get_topic_name(&topic_path);
    if !should_process_topic(topic_name, topic_filter) {
      continue;
    }

    let topic_results = process_topic_semantic(&topic_path, query_words, overview_only)?;
    results.extend(topic_results);
  }

  Ok(results)
}

/// Check if a topic should be processed based on filter
#[cfg(feature = "semantic")]
fn should_process_topic(topic_name: &str, topic_filter: Option<&str>) -> bool {
  match topic_filter {
    Some(filter) => topic_name == filter,
    None => true,
  }
}

/// Get topic name from directory path
#[cfg(feature = "semantic")]
fn get_topic_name(topic_path: &std::path::Path) -> &str {
  topic_path.file_name().and_then(|name| name.to_str()).unwrap_or("unknown")
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

/// Process a single insight file and return SearchResult if it matches
fn process_single_insight_file(
  path: &Path,
  insight_name: &str,
  topic_name: &str,
  terms: &[String],
  case_sensitive: bool,
  overview_only: bool,
) -> Result<Option<SearchResult>> {
  let content = fs::read_to_string(path)?;
  let search_content = extract_searchable_content(&content, overview_only)?;
  let score = calc_term_matches(&search_content, terms, case_sensitive);

  if score > 0.0 {
    Ok(Some(SearchResult {
      topic: topic_name.to_string(),
      name: insight_name.to_string(),
      content: search_content,
      score,
    }))
  } else {
    Ok(None)
  }
}

/// Process insight files in a topic directory
fn process_insight_files_in_topic(
  topic_path: &Path,
  terms: &[String],
  case_sensitive: bool,
  overview_only: bool,
) -> Result<Vec<SearchResult>> {
  let mut results = Vec::new();

  if !topic_path.exists() {
    return Ok(results);
  }

  let topic_name = topic_path.file_name().and_then(|n| n.to_str()).unwrap_or("unknown");

  for entry in fs::read_dir(topic_path)? {
    let entry = entry?;
    let path = entry.path();

    if let Some(insight_name) = is_insight_file(&path) {
      if let Ok(Some(result)) = process_single_insight_file(
        &path,
        insight_name,
        topic_name,
        terms,
        case_sensitive,
        overview_only,
      ) {
        results.push(result);
      }
    }
  }

  Ok(results)
}

/// Helper function to collect exact search results without printing them
fn find_exact_matches(
  terms: &[String],
  topic_filter: Option<&str>,
  case_sensitive: bool,
  overview_only: bool,
) -> Result<Vec<SearchResult>> {
  let insights_root = get_insights_dir()?;
  let mut results = Vec::new();

  let search_paths = build_search_paths(&insights_root, topic_filter)?;

  for topic_path in search_paths {
    let topic_results =
      process_insight_files_in_topic(&topic_path, terms, case_sensitive, overview_only)?;
    results.extend(topic_results);
  }

  Ok(results)
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

#[cfg(feature = "neural")]
fn format_insight_content(insight: &Insight, overview_only: bool) -> String {
  if overview_only {
    format!("{} {} {}", insight.topic, insight.name, insight.overview)
  } else {
    format!("{} {} {} {}", insight.topic, insight.name, insight.overview, insight.details)
  }
}

#[cfg(feature = "neural")]
fn process_insight_for_neural_search(
  insight: &Insight,
  session: &mut ort::session::Session,
  query_embedding: &[f32],
  overview_only: bool,
) -> Result<Option<SearchResult>> {
  let content = format_insight_content(insight, overview_only);
  let content_embedding = embedding_client::create_embedding(session, &content)?;
  let similarity = similarity::cosine_similarity(query_embedding, &content_embedding);

  if similarity <= 0.2 {
    return Ok(None);
  }

  Ok(Some(SearchResult {
    topic: insight.topic.clone(),
    name: insight.name.clone(),
    content: if overview_only {
      insight.overview.clone()
    } else {
      format!("{}\n\n{}", insight.overview, insight.details)
    },
    score: similarity,
  }))
}

#[cfg(feature = "neural")]
fn find_embedding_matches(
  terms: &[String],
  topic_filter: Option<&str>,
  _case_sensitive: bool,
  overview_only: bool,
) -> Result<Vec<SearchResult>> {
  let mut session = init_embedding_session()?;
  let query_embedding = embedding_client::create_embedding(&mut session, &terms.join(" "))?;
  let insight_refs = get_insights(topic_filter)?;

  let results =
    process_insights_for_neural(&insight_refs, &mut session, &query_embedding, overview_only);
  Ok(results)
}

#[cfg(feature = "neural")]
fn process_insights_for_neural(
  insight_refs: &[(String, String)],
  session: &mut ort::session::Session,
  query_embedding: &[f32],
  overview_only: bool,
) -> Vec<SearchResult> {
  insight_refs
    .iter()
    .filter_map(|(topic, name)| {
      process_single_insight_neural(topic, name, session, query_embedding, overview_only)
    })
    .collect()
}

#[cfg(feature = "neural")]
fn process_single_insight_neural(
  topic: &str,
  name: &str,
  session: &mut ort::session::Session,
  query_embedding: &[f32],
  overview_only: bool,
) -> Option<SearchResult> {
  let insight = Insight::load(topic, name).ok()?;
  let search_result =
    process_insight_for_neural_search(&insight, session, query_embedding, overview_only)
      .ok()
      .flatten()?;
  Some(search_result)
}

/// Build search paths based on topic filter
fn build_search_paths(insights_root: &Path, topic_filter: Option<&str>) -> Result<Vec<PathBuf>> {
  if let Some(topic) = topic_filter {
    Ok(vec![insights_root.join(topic)])
  } else {
    Ok(get_topics()?.into_iter().map(|topic| insights_root.join(topic)).collect())
  }
}

/// Display the combined search results
fn display_results(results: &[SearchResult], terms: &[String]) {
  if results.is_empty() {
    println!("No matches found for: {}", terms.join(" "));
  } else {
    for result in results {
      println!(
        "=== {}/{} ({:.1}%) ===",
        result.topic.cyan(),
        result.name.yellow(),
        result.score * 100.0
      );

      let lines: Vec<&str> = result.content.lines().collect();
      for line in lines.iter() {
        if !line.starts_with("---") {
          println!("{line}");
        }
      }
      println!();
    }
  }
}
