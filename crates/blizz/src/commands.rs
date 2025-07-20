use anyhow::{anyhow, Result};
use colored::*;
use std::collections::HashMap;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use crate::embedding_client;
use crate::insight::*;
use crate::search;



/// Compute embedding for an insight using the daemon
#[cfg(feature = "neural")]
async fn compute_insight_embedding(insight: &Insight) -> Result<(String, Vec<f32>, String)> {
  let embedding_text = insight.get_embedding_text();
  let embedding = embedding_client::get_embedding_from_daemon(&embedding_text).await?;
  let version = "all-MiniLM-L6-v2".to_string();

  Ok((version, embedding, embedding_text))
}

#[cfg(feature = "neural")]
fn handle_embedding_computation(insight: &mut Insight) -> Result<()> {
  let rt = tokio::runtime::Runtime::new()?;
  match rt.block_on(async { compute_insight_embedding(insight).await }) {
    Ok((version, embedding, text)) => {
      insight.set_embedding(version, embedding, text);
    }
    Err(e) => {
      eprintln!("  {} Warning: Failed to compute embedding: {}", "⚠".yellow(), e);
      eprintln!(
        "  {} Insight saved without embedding (can be computed later with 'blizz index')",
        "ℹ".blue()
      );
    }
  }
  Ok(())
}

fn compute_and_set_embedding(insight: &mut Insight) -> Result<()> {
  #[cfg(feature = "neural")]
  return handle_embedding_computation(insight);

  #[cfg(not(feature = "neural"))]
  {
    let _ = insight;
    Ok(())
  }
}

/// Add a new insight to the knowledge base
pub fn add_insight(topic: &str, name: &str, overview: &str, details: &str) -> Result<()> {
  let mut insight =
    Insight::new(topic.to_string(), name.to_string(), overview.to_string(), details.to_string());

  // Compute embedding before saving
  compute_and_set_embedding(&mut insight)?;

  insight.save()?;

  println!("{} Added insight {}/{}", "✓".green(), topic.cyan(), name.yellow());
  Ok(())
}







/// Exact term matching search (the original implementation)
pub fn search_insights_exact(
  terms: &[String],
  topic_filter: Option<&str>,
  case_sensitive: bool,
  overview_only: bool,
) -> Result<()> {
  search::search_insights_exact(terms, topic_filter, case_sensitive, overview_only)
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

/// Create search result if similarity meets threshold
#[cfg(feature = "semantic")]
fn create_semantic_result_if_meaningful(
  topic_name: &str,
  insight_name: &str,
  overview: &str,
  details: &str,
  similarity: f32,
  overview_only: bool,
) -> Option<SemanticSearchResult> {
  if similarity > SEMANTIC_SIMILARITY_THRESHOLD {
    Some(SemanticSearchResult {
      topic: topic_name.to_string(),
      name: insight_name.to_string(),
      content: if overview_only {
        overview.to_string()
      } else {
        format!("{overview}\n\n{details}")
      },
      similarity,
    })
  } else {
    None
  }
}

/// Calculate semantic search result for a single insight
#[cfg(feature = "semantic")]
fn calculate_single_insight_semantic(
  topic_name: &str,
  insight_path: &Path,
  query_words: &HashSet<String>,
  overview_only: bool,
) -> Result<Option<SemanticSearchResult>> {
  let insight_name = extract_insight_name_from_path(insight_path);
  let content = fs::read_to_string(insight_path)?;
  let (overview, details) = parse_insight_content(&content)?;

  let search_text = build_search_text(topic_name, insight_name, &overview, &details, overview_only);
  let similarity = calculate_semantic_similarity(query_words, &search_text);

  Ok(create_semantic_result_if_meaningful(
    topic_name,
    insight_name,
    &overview,
    &details,
    similarity,
    overview_only,
  ))
}

#[cfg(feature = "semantic")]
fn process_topic_semantic(
  topic_path: &Path,
  query_words: &HashSet<String>,
  overview_only: bool,
) -> Result<Vec<SemanticSearchResult>> {
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

/// Display semantic search results
#[cfg(feature = "semantic")]
fn display_semantic_search_results(results: &[SemanticSearchResult], terms: &[String]) {
  if results.is_empty() {
    println!("No matches found for: {}", terms.join(" "));
  } else {
    for result in results {
      // Format exactly like regular search
      println!("=== {}/{} ===", result.topic.cyan(), result.name.yellow());

      // Show content same as regular search - split into lines and show first several
      let lines: Vec<&str> = result.content.lines().collect();
      for line in lines.iter() {
        if !line.trim().is_empty() && !line.starts_with("---") {
          println!("{line}");
        }
      }
      println!();
    }
  }
}

/// Sort semantic search results by similarity and name
#[cfg(feature = "semantic")]
fn sort_semantic_results(results: &mut [SemanticSearchResult]) {
  results.sort_by(|a, b| {
    b.similarity
      .partial_cmp(&a.similarity)
      .unwrap_or(std::cmp::Ordering::Equal)
      .then(a.topic.cmp(&b.topic))
      .then(a.name.cmp(&b.name))
  });
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
) -> Result<Vec<SemanticSearchResult>> {
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

/// Finalize and display semantic search results
#[cfg(feature = "semantic")]
fn finalize_semantic_results(mut results: Vec<SemanticSearchResult>, terms: &[String]) {
  sort_semantic_results(&mut results);
  display_semantic_search_results(&results, terms);
}

/// Semantic similarity search using advanced text analysis
#[cfg(feature = "semantic")]
#[allow(dead_code)]
pub fn search_insights_semantic(
  terms: &[String],
  topic_filter: Option<&str>,
  case_sensitive: bool,
  overview_only: bool,
) -> Result<()> {
  search::search_insights_semantic(terms, topic_filter, case_sensitive, overview_only)
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

pub fn list_insights(filter: Option<&str>, verbose: bool) -> Result<()> {
  let insights = get_insights(filter)?;

  if insights.is_empty() {
    if let Some(topic) = filter {
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

/// Check if insight content has actually changed
fn has_content_changed(new_overview: Option<&str>, new_details: Option<&str>) -> bool {
  new_overview.is_some() || new_details.is_some()
}

/// Recompute embedding and save insight using existing methods
fn recompute_and_save_updated_insight(insight: &mut Insight) -> Result<()> {
  compute_and_set_embedding(insight)?;

  // For updates, we need to delete first then save since save() checks for existing files
  let file_path = insight.file_path()?;
  if file_path.exists() {
    std::fs::remove_file(&file_path)?;
  }
  insight.save()?;

  Ok(())
}

/// Print success message for insight update
fn print_update_success(topic: &str, name: &str) {
  println!("{} Updated insight {}/{}", "✓".green(), topic.cyan(), name.yellow());
}

/// Update an existing insight
pub fn update_insight(
  topic: &str,
  name: &str,
  new_overview: Option<&str>,
  new_details: Option<&str>,
) -> Result<()> {
  let mut insight = Insight::load(topic, name)?;
  let content_changed = has_content_changed(new_overview, new_details);

  insight.update(new_overview, new_details)?;

  if content_changed {
    recompute_and_save_updated_insight(&mut insight)?;
  }

  print_update_success(topic, name);
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
    embedding_client::get_embedding_from_daemon(&content).await.map_err(|e| {
      anyhow!("Failed to compute embedding for {}/{}: {}", insight.topic, insight.name, e)
    })
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
    let similarity = embedding_client::cosine_similarity(query_embedding, &content_embedding);

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
  case_sensitive: bool,
  overview_only: bool,
) -> Result<()> {
  search::search_neural(terms, topic_filter, case_sensitive, overview_only)
}

/// Create embedding - now uses daemon for speed!
#[cfg(feature = "neural")]
fn create_embedding(_session: &mut ort::session::Session, text: &str) -> Result<Vec<f32>> {
  embedding_client::create_embedding(_session, text)
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
      content: result.matching_lines.join("\n"),
      score: result.score as f32 * 0.3,
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

  let exact_results = collect_exact_results(terms, topic_filter, case_sensitive, overview_only)?;
  all_results.extend(transform_exact_results(exact_results));

  Ok(all_results)
}

#[cfg(feature = "semantic")]
pub fn search_insights_combined_semantic(
  terms: &[String],
  topic_filter: Option<&str>,
  case_sensitive: bool,
  overview_only: bool,
) -> Result<()> {
  search::search_insights_combined_semantic(terms, topic_filter, case_sensitive, overview_only)
}

// Search result scoring constants
const SEMANTIC_SCALE_FACTOR: f32 = 0.8;
const EXACT_SCALE_FACTOR: f32 = 0.3;

/// Transform neural search results into combined format
#[cfg(feature = "neural")]
fn add_neural_results(
  all_results: &mut Vec<CombinedSearchResult>,
  terms: &[String],
  topic_filter: Option<&str>,
  case_sensitive: bool,
  overview_only: bool,
) -> Result<()> {
  let neural_results = collect_neural_results(terms, topic_filter, case_sensitive, overview_only)?;
  for result in neural_results {
    all_results.push(CombinedSearchResult {
      topic: result.topic,
      name: result.name,
      content: result.content,
      score: result.similarity,
      search_type: "neural".to_string(),
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
  let exact_results = collect_exact_results(terms, topic_filter, case_sensitive, overview_only)?;
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

pub fn search_all(
  terms: &[String],
  topic_filter: Option<&str>,
  case_sensitive: bool,
  overview_only: bool,
) -> Result<()> {
  search::search_all(terms, topic_filter, case_sensitive, overview_only)
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
  let (score, mut matching_lines) = calc_term_matches(&search_content, terms, case_sensitive);

  if score > 0 {
    // Filter out empty lines and frontmatter for cleaner output
    matching_lines.retain(|line| !line.trim().is_empty() && !line.starts_with("---"));

    Ok(Some(SearchResult {
      topic: topic_name.to_string(),
      name: insight_name.to_string(),
      matching_lines,
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
fn collect_exact_results(
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
fn init_neural_session() -> Result<ort::session::Session> {
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
) -> Result<Option<SemanticSearchResult>> {
  let content = format_insight_content(insight, overview_only);
  let content_embedding = create_embedding(session, &content)?;
  let similarity = embedding_client::cosine_similarity(query_embedding, &content_embedding);

  if similarity <= 0.2 {
    return Ok(None);
  }

  Ok(Some(SemanticSearchResult {
    topic: insight.topic.clone(),
    name: insight.name.clone(),
    content: if overview_only {
      insight.overview.clone()
    } else {
      format!("{}\n\n{}", insight.overview, insight.details)
    },
    similarity,
  }))
}

#[cfg(feature = "neural")]
fn collect_neural_results(
  terms: &[String],
  topic_filter: Option<&str>,
  _case_sensitive: bool,
  overview_only: bool,
) -> Result<Vec<SemanticSearchResult>> {
  let mut session = init_neural_session()?;
  let query_embedding = create_embedding(&mut session, &terms.join(" "))?;
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
) -> Vec<SemanticSearchResult> {
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
) -> Option<SemanticSearchResult> {
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

/// Group results by insight, collecting all search methods that found each one
fn group_results_by_insight(
  results: Vec<CombinedSearchResult>,
) -> HashMap<(String, String), Vec<CombinedSearchResult>> {
  let mut insight_groups: HashMap<(String, String), Vec<CombinedSearchResult>> = HashMap::new();

  for result in results {
    // Normalize insight name by removing .insight suffix for deduplication
    let normalized_name = result.name.strip_suffix(".insight").unwrap_or(&result.name).to_string();
    let key = (result.topic.clone(), normalized_name);

    insight_groups.entry(key).or_default().push(result);
  }

  insight_groups
}

/// Sort a group of results by priority and score to find the best representative
fn sort_group_by_priority(group: &mut [CombinedSearchResult]) {
  group.sort_by(|a, b| {
    let a_priority = search_type_priority(&a.search_type);
    let b_priority = search_type_priority(&b.search_type);

    match a_priority.cmp(&b_priority) {
      std::cmp::Ordering::Equal => {
        b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal)
      }
      other => other,
    }
  });
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

/// Extract the best result from each insight group
fn extract_best_from_groups(
  insight_groups: HashMap<(String, String), Vec<CombinedSearchResult>>,
) -> Vec<(CombinedSearchResult, usize)> {
  let mut final_results = Vec::new();

  for (_, mut group) in insight_groups {
    let method_count = group.len(); // Number of search methods that found this insight
    sort_group_by_priority(&mut group);

    // Take the best result from this group
    if let Some(best_result) = group.into_iter().next() {
      final_results.push((best_result, method_count));
    }
  }

  final_results
}

/// Select best result from each group and prepare final sorted list
fn select_best_results(
  insight_groups: HashMap<(String, String), Vec<CombinedSearchResult>>,
) -> Vec<CombinedSearchResult> {
  let mut final_results = extract_best_from_groups(insight_groups);

  // Sort by: number of search methods (descending), then search type priority, then score
  final_results.sort_by(compare_final_results);

  // Extract just the results for display
  final_results.into_iter().map(|(result, _)| result).collect()
}

/// Display the combined search results
fn display_search_results_combined(results: &[CombinedSearchResult], terms: &[String]) {
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

/// Display combined results from multiple search methods
fn display_combined_results(results: Vec<CombinedSearchResult>, terms: &[String]) -> Result<()> {
  let insight_groups = group_results_by_insight(results);
  let final_results = select_best_results(insight_groups);
  display_search_results_combined(&final_results, terms);
  Ok(())
}



/// Check if insight should be skipped during indexing
#[cfg(feature = "neural")]
fn should_skip_insight(insight: &crate::insight::Insight, force: bool, missing_only: bool) -> bool {
  !force && missing_only && insight.has_embedding()
}

/// Save insight with embedding metadata to file
#[cfg(feature = "neural")]
fn save_insight_with_embedding(insight: &crate::insight::Insight) -> Result<()> {
  let file_path = insight.file_path()?;

  let frontmatter = crate::insight::FrontMatter {
    overview: insight.overview.clone(),
    embedding_version: insight.embedding_version.clone(),
    embedding: insight.embedding.clone(),
    embedding_text: insight.embedding_text.clone(),
    embedding_computed: insight.embedding_computed,
  };

  let yaml_content = serde_yaml::to_string(&frontmatter)?;
  let content = format!("---\n{}---\n\n# Details\n{}", yaml_content, insight.details);
  std::fs::write(&file_path, content)?;

  Ok(())
}

/// Process a single insight for indexing
#[cfg(feature = "neural")]
fn process_single_insight_index(
  topic: &str,
  insight_name: &str,
  force: bool,
  missing_only: bool,
) -> Result<(bool, bool)> {
  // (processed, skipped)
  use crate::insight::Insight;

  let mut insight = Insight::load(topic, insight_name)?;

  // Check if we should skip this insight
  if should_skip_insight(&insight, force, missing_only) {
    println!("    · {insight_name} (already has embedding)");
    return Ok((false, true));
  }

  print!("  · {insight_name}... ");
  std::io::Write::flush(&mut std::io::stdout())?;

  // Compute embedding
  compute_and_set_embedding(&mut insight)?;
  save_insight_with_embedding(&insight)?;

  println!("done");
  Ok((true, false))
}

/// Print indexing summary
#[cfg(feature = "neural")]
fn print_index_summary(processed: usize, skipped: usize, errors: usize, topic_count: usize) {
  println!(
    "\nIndexed {} insights across {} topics {}",
    processed.to_string().yellow(),
    topic_count.to_string().yellow(),
    "⚡".blue()
  );

  if skipped > 0 {
    println!("  {} Skipped: {}", "⏭".blue(), skipped.to_string().yellow());
  }
  if errors > 0 {
    println!("  {} Errors: {}", "❌".red(), errors.to_string().yellow());
  }
}

/// Statistics tracking for indexing operations
#[cfg(feature = "neural")]
#[derive(Default)]
struct IndexingStats {
  processed: usize,
  skipped: usize,
  errors: usize,
}

#[cfg(feature = "neural")]
impl IndexingStats {
  fn add_result(&mut self, result: Result<(bool, bool)>) {
    match result {
      Ok((was_processed, was_skipped)) => {
        if was_processed {
          self.processed += 1;
        }
        if was_skipped {
          self.skipped += 1;
        }
      }
      Err(_) => {
        self.errors += 1;
      }
    }
  }
}

/// Validate that topics exist for indexing
#[cfg(feature = "neural")]
fn validate_topics_for_indexing() -> Result<Vec<String>> {
  use crate::insight::get_topics;

  let topics = get_topics()?;
  if topics.is_empty() {
    println!("No insights found to index.");
  }
  Ok(topics)
}

/// Process indexing for all insights in a single topic
#[cfg(feature = "neural")]
fn process_topic_indexing(
  topic: &str,
  force: bool,
  missing_only: bool,
  stats: &mut IndexingStats,
) -> Result<()> {
  use crate::insight::get_insights;

  let insights = get_insights(Some(topic))?;
  println!("{} {}:", "◈".blue(), topic.cyan());

  for (_, insight_name) in insights {
    let result = process_single_insight_index(topic, &insight_name, force, missing_only);
    if let Err(ref e) = result {
      eprintln!("    · Failed to process {insight_name}: {e}");
    }
    stats.add_result(result);
  }

  Ok(())
}

/// Recompute embeddings for all insights
#[cfg(feature = "neural")]
pub fn index_insights(force: bool, missing_only: bool) -> Result<()> {
  let topics = validate_topics_for_indexing()?;

  if topics.is_empty() {
    return Ok(());
  }

  let mut stats = IndexingStats::default();

  for topic in &topics {
    process_topic_indexing(topic, force, missing_only, &mut stats)?;
  }

  print_index_summary(stats.processed, stats.skipped, stats.errors, topics.len());
  Ok(())
}
