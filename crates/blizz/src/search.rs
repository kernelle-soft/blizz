use anyhow::{anyhow, Result};
use colored::*;
use std::collections::HashMap;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use clap::Args;

use crate::embedding_client;
use crate::insight::*;
use crate::semantic;
use crate::similarity;

// Search result scoring constants
const SEMANTIC_SCALE_FACTOR: f32 = 0.8;
const EXACT_SCALE_FACTOR: f32 = 0.3;

#[derive(Debug)]
pub struct SearchResult {
  pub topic: String,
  pub name: String,
  pub content: String,
  pub score: f32, // number of matching terms
}

#[cfg(feature = "semantic")]
#[derive(Debug)]
pub struct SemanticSearchResult {
  pub topic: String,
  pub name: String,
  pub content: String,
  pub similarity: f32, // semantic similarity score
}

#[derive(Debug, Clone)]
pub struct CombinedSearchResult {
  pub topic: String,
  pub name: String,
  pub content: String,
  pub score: f32,          // Unified score across search methods
  pub search_type: String, // Which search method found this result
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

  let insights_dir = validate_insights_directory()?;
  let query = terms.join(" ");
  let query_words = semantic::extract_words(&query);

  let results = process_all_topics_semantic(&insights_dir, &query_words, options.topic.as_deref(), options.overview_only)?;
  Ok(results)
}

/// Build search paths based on topic filter
fn build_search_paths(insights_root: &Path, topic_filter: Option<&str>) -> Result<Vec<PathBuf>> {
  if let Some(topic) = topic_filter {
    Ok(vec![insights_root.join(topic)])
  } else {
    Ok(get_topics()?.into_iter().map(|topic| insights_root.join(topic)).collect())
  }
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
  let similarity = calculate_semantic_similarity(query_words, &search_text);

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

/// Sort semantic search results by similarity and name
#[cfg(feature = "semantic")]
fn sort_semantic_results(results: &mut [SearchResult]) {
  results.sort_by(|a, b| b.score.cmp(&a.score).then_with(|| a.topic.cmp(&b.topic).then_with(|| a.name.cmp(&b.name))));
}

/// Validate insights directory exists, return early if not
#[cfg(feature = "semantic")]
fn validate_insights_directory() -> Result<std::path::PathBuf> {
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

/// Semantic similarity search using advanced text analysis
#[cfg(feature = "semantic")]
pub fn search_insights_semantic(
  terms: &[String],
  topic_filter: Option<&str>,
  _case_sensitive: bool, // Note: Semantic search normalizes text, so case sensitivity doesn't apply
  overview_only: bool,
) -> Result<Vec<SearchResult>> {
  let insights_dir = validate_insights_directory()?;
  let query = terms.join(" ");
  let query_words = semantic::extract_words(&query.to_lowercase());

  process_all_topics_semantic(&insights_dir, &query_words, topic_filter, overview_only)?;
}

/// Calculate semantic similarity using Jaccard + frequency analysis
#[cfg(feature = "semantic")]
fn calculate_semantic_similarity(query_words: &HashSet<String>, content: &str) -> f32 {
  let content_words = semantic::extract_words(&content.to_lowercase());

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

/// Get embedding for insight content, using cache if available
#[cfg(feature = "neural")]
async fn get_insight_embedding(
  insight: &crate::insight::Insight,
  overview_only: bool,
  warnings: &mut Vec<String>,
) -> Result<Vec<f32>> {
  if let Some(cached_embedding) = &insight.embedding {
    // Use cached embedding for speed!
    Ok(cached_embedding.clone())
  } else {
    // Fallback: compute embedding on-the-fly (slow)
    warnings.push(format!("{}/{}", insight.topic, insight.name));

    let content = if overview_only {
      format!("{} {} {}", insight.topic, insight.name, insight.overview)
    } else {
      insight.get_embedding_text()
    };

    // Use daemon for computation
    embedding_client::generate_embedding(&content).await.map_err(|e| {
      anyhow!("Failed to compute embedding for {}/{}: {}", insight.topic, insight.name, e)
    }).map(|embedding| embedding.embedding)
  }
}

/// Process insights and calculate neural similarities
#[cfg(feature = "neural")]
async fn calculate_neural_similarities(
  insight_refs: Vec<(String, String)>,
  query_embedding: &[f32],
  overview_only: bool,
) -> Result<(Vec<(crate::insight::Insight, f32)>, Vec<String>)> {
  let mut results = Vec::new();
  let mut warnings = Vec::new();

  for (topic, name) in insight_refs {
    // Load the insight with cached metadata
    let insight = crate::insight::Insight::load(&topic, &name)?;

    let content_embedding = get_insight_embedding(&insight, overview_only, &mut warnings).await?;

    // Calculate cosine similarity
    let similarity = similarity::cosine_similarity(query_embedding, &content_embedding);

    if similarity > 0.2 {
      // Similarity threshold (20% for quality results)
      results.push((insight, similarity));
    }
  }

  Ok((results, warnings))
}

/// Display warnings about missing embeddings
#[cfg(feature = "neural")]
fn display_embedding_warnings(warnings: &[String]) {
  if !warnings.is_empty() {
    eprintln!(
      "{} {} insights computed embeddings on-the-fly (slower):",
      "⚠".yellow(),
      warnings.len()
    );
    for warning in warnings {
      eprintln!("  {warning}");
    }
    eprintln!("  {} Tip: Run 'blizz index' to cache embeddings for faster searches", "💡".blue());
    eprintln!();
  }
}

#[cfg(feature = "neural")]
fn display_neural_results(results: &[(crate::insight::Insight, f32)], terms: &[String]) {
  if results.is_empty() {
    println!("No matches found for: {}", terms.join(" "));
  } else {
    for (result, sim) in results {
      println!(
        "=== {}/{} === (similarity: {:.1}%)",
        result.topic.cyan(),
        result.name.yellow(),
        sim * 100.0
      );

      let content = format!("{}\n\n{}", result.overview, result.details);
      let lines: Vec<&str> = content.lines().collect();
      for line in lines.iter() {
        if !line.starts_with("---") {
          println!("{line}");
        }
      }
      println!();
    }
  }
}

#[cfg(feature = "neural")]
#[allow(dead_code)]
pub fn search_neural(
  terms: &[String],
  topic_filter: Option<&str>,
  _case_sensitive: bool,
  overview_only: bool,
) -> Result<()> {
  let query = terms.join(" ");

  let rt = tokio::runtime::Runtime::new()?;
  let embedding = rt
    .block_on(async { embedding_client::generate_embedding(&query).await })
    .map_err(|e| anyhow!("Failed to get query embedding: {}", e))?;

  let refs = get_insights(topic_filter)?;
  let (mut results, warnings) =
    rt.block_on(async { calculate_neural_similarities(refs, &embedding.embedding, overview_only).await })?;

  results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

  display_embedding_warnings(&warnings);
  display_neural_results(&results, terms);

  Ok(())
}

#[cfg(feature = "semantic")]
fn transform_semantic_results(
  semantic_results: Vec<SemanticSearchResult>,
) -> Vec<CombinedSearchResult> {
  semantic_results
    .into_iter()
    .map(|result| CombinedSearchResult {
      topic: result.topic,
      name: result.name,
      content: result.content,
      score: result.similarity,
      search_type: "semantic".to_string(),
    })
    .collect()
}

#[cfg(feature = "semantic")]
fn transform_exact_results(exact_results: Vec<SearchResult>) -> Vec<CombinedSearchResult> {
  exact_results
    .into_iter()
    .map(|result| CombinedSearchResult {
      topic: result.topic,
      name: result.name,
      content: result.content,
      score: result.score as f32 * EXACT_SCALE_FACTOR,
      search_type: "exact".to_string(),
    })
    .collect()
}

#[cfg(feature = "semantic")]
fn collect_all_search_results(
  terms: &[String],
  topic_filter: Option<&str>,
  case_sensitive: bool,
  overview_only: bool,
) -> Result<Vec<CombinedSearchResult>> {
  let mut all_results = Vec::new();

  let semantic_results =
    collect_semantic_results(terms, topic_filter, case_sensitive, overview_only)?;
  all_results.extend(transform_semantic_results(semantic_results));

  let exact_results = find_exact_matches(terms, topic_filter, case_sensitive, overview_only)?;
  all_results.extend(transform_exact_results(exact_results));

  Ok(all_results)
}

/// Transform neural search results into combined format
#[cfg(feature = "neural")]
fn add_neural_results(
  all_results: &mut Vec<SearchResult>,
  terms: &[String],
  topic_filter: Option<&str>,
  case_sensitive: bool,
  overview_only: bool,
) -> Result<()> {
  let neural_results = find_embedding_matches(terms, topic_filter, case_sensitive, overview_only)?;
  for result in neural_results {
    all_results.push(SearchResult {
      topic: result.topic,
      name: result.name,
      content: result.content,
      score: result.score,
    });
  }
  Ok(())
}

/// Transform semantic search results into combined format
#[cfg(feature = "semantic")]
fn add_semantic_results(
  all_results: &mut Vec<CombinedSearchResult>,
  terms: &[String],
  topic_filter: Option<&str>,
  case_sensitive: bool,
  overview_only: bool,
) -> Result<()> {
  let semantic_results =
    collect_semantic_results(terms, topic_filter, case_sensitive, overview_only)?;
  for result in semantic_results {
    all_results.push(CombinedSearchResult {
      topic: result.topic,
      name: result.name,
      content: result.content,
      score: result.similarity * SEMANTIC_SCALE_FACTOR,
      search_type: "semantic".to_string(),
    });
  }
  Ok(())
}

/// Transform exact search results into combined format
fn add_exact_results(
  all_results: &mut Vec<CombinedSearchResult>,
  terms: &[String],
  topic_filter: Option<&str>,
  case_sensitive: bool,
  overview_only: bool,
) -> Result<()> {
  let exact_results = find_exact_matches(terms, topic_filter, case_sensitive, overview_only)?;
  for result in exact_results {
    all_results.push(CombinedSearchResult {
      topic: result.topic,
      name: result.name,
      content: result.matching_lines.join("\n"),
      score: result.score as f32 * EXACT_SCALE_FACTOR,
      search_type: "exact".to_string(),
    });
  }
  Ok(())
}

#[cfg(feature = "semantic")]
fn process_insight_for_semantic_search(
  insight_path: &Path,
  topic_name: &str,
  query_words: &HashSet<String>,
  overview_only: bool,
) -> Result<Option<SemanticSearchResult>> {
  if insight_path.extension().and_then(|s| s.to_str()) != Some("md") {
    return Ok(None);
  }

  let insight_name = insight_path.file_stem().and_then(|name| name.to_str()).unwrap_or("unknown");
  let content = fs::read_to_string(insight_path)?;
  let (overview, details) = parse_insight_content(&content)?;

  let search_text = if overview_only {
    format!("{topic_name} {insight_name} {overview}")
  } else {
    format!("{topic_name} {insight_name} {overview} {details}")
  };

  let similarity = calculate_semantic_similarity(query_words, &search_text);

  if similarity <= 0.2 {
    return Ok(None);
  }

  Ok(Some(SemanticSearchResult {
    topic: topic_name.to_string(),
    name: insight_name.to_string(),
    content: if overview_only { overview } else { format!("{overview}\n\n{details}") },
    similarity,
  }))
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

/// Collect semantic results from a single topic directory
#[cfg(feature = "semantic")]
fn collect_topic_semantic_results(
  topic_path: &std::path::Path,
  topic_name: &str,
  query_words: &HashSet<String>,
  overview_only: bool,
) -> Result<Vec<SemanticSearchResult>> {
  let mut results = Vec::new();

  for insight_entry in fs::read_dir(topic_path)? {
    let insight_path = insight_entry?.path();
    if let Ok(Some(result)) =
      process_insight_for_semantic_search(&insight_path, topic_name, query_words, overview_only)
    {
      results.push(result);
    }
  }

  Ok(results)
}

#[cfg(feature = "semantic")]
fn collect_semantic_results(
  terms: &[String],
  topic_filter: Option<&str>,
  _case_sensitive: bool,
  overview_only: bool,
) -> Result<Vec<SemanticSearchResult>> {
  let insights_dir = get_insights_root()?;
  if !insights_dir.exists() {
    return Ok(Vec::new());
  }

  let query_words = semantic::extract_words(&terms.join(" ").to_lowercase());
  let mut results = Vec::new();

  for entry in fs::read_dir(&insights_dir)? {
    let topic_path = entry?.path();
    if !topic_path.is_dir() {
      continue;
    }

    let topic_name = get_topic_name(&topic_path);
    if !should_process_topic(topic_name, topic_filter) {
      continue;
    }

    let topic_results =
      collect_topic_semantic_results(&topic_path, topic_name, &query_words, overview_only)?;
    results.extend(topic_results);
  }

  Ok(results)
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
  let insights_root = get_insights_root()?;
  let mut results = Vec::new();

  if !insights_root.exists() {
    return Ok(results);
  }

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

/// Get search type priority (lower = higher priority)
fn search_type_priority(search_type: &str) -> u8 {
  match search_type {
    "exact" => 1,
    "semantic" => 2,
    "neural" => 3,
    _ => 4,
  }
}


/// Compare two final results for sorting by method count, priority, and score
fn compare_final_results(
  a: &(CombinedSearchResult, usize),
  b: &(CombinedSearchResult, usize),
) -> std::cmp::Ordering {
  // First by number of search methods that found it (more = better)
  match b.1.cmp(&a.1) {
    std::cmp::Ordering::Equal => {
      let a_priority = search_type_priority(&a.0.search_type);
      let b_priority = search_type_priority(&b.0.search_type);

      // Then by search type priority
      match a_priority.cmp(&b_priority) {
        std::cmp::Ordering::Equal => {
          // Finally by score (descending)
          b.0.score.partial_cmp(&a.0.score).unwrap_or(std::cmp::Ordering::Equal)
        }
        other => other,
      }
    }
    other => other,
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
