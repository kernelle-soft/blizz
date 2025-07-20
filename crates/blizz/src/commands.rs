use anyhow::{anyhow, Result};
use colored::*;
use std::collections::HashMap;
#[cfg(feature = "semantic")]
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

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

#[derive(Debug, Clone)]
struct CombinedSearchResult {
  topic: String,
  name: String,
  content: String,
  score: f32,          // Unified score across search methods
  search_type: String, // Which search method found this result
}

/// Compute embedding for an insight using the daemon
#[cfg(feature = "neural")]
async fn compute_insight_embedding(insight: &Insight) -> Result<(String, Vec<f32>, String)> {
  let embedding_text = insight.get_embedding_text();
  let embedding = daemon_client::get_embedding_from_daemon(&embedding_text).await?;
  let version = "all-MiniLM-L6-v2".to_string();

  Ok((version, embedding, embedding_text))
}

/// Compute embedding for an insight with fallback
fn compute_and_set_embedding(insight: &mut Insight) -> Result<()> {
  #[cfg(feature = "neural")]
  {
    // Use async runtime to compute embedding
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
  }

  #[cfg(not(feature = "neural"))]
  {
    // No embedding computation available
    let _ = insight; // Suppress unused warning
  }

  Ok(())
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

// Legacy search_insights function removed - now using specific search mode functions directly

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

/// Calculate term matches and collect matching lines
fn calculate_term_matches(
  content: &str,
  terms: &[String],
  case_sensitive: bool,
) -> (usize, Vec<String>) {
  let mut score = 0;
  let mut matching_lines = Vec::new();

  // Count matching terms
  for term in terms {
    let term_matches = if case_sensitive {
      content.contains(term)
    } else {
      content.to_lowercase().contains(&term.to_lowercase())
    };

    if term_matches {
      score += 1;
    }
  }

  // Collect matching lines if any terms matched
  if score > 0 {
    for line in content.lines() {
      let line_has_match = terms.iter().any(|term| {
        if case_sensitive {
          line.contains(term)
        } else {
          line.to_lowercase().contains(&term.to_lowercase())
        }
      });

      if line_has_match {
        matching_lines.push(line.to_string());
      }
    }
  }

  (score, matching_lines)
}

/// Display exact search results
fn display_exact_search_results(results: &[SearchResult], terms: &[String]) {
  if results.is_empty() {
    let terms_str = terms.join(" ");
    println!("No matches found for: {}", terms_str.yellow());
  } else {
    for result in results {
      println!(
        "=== {}/{} ({} terms) ===",
        result.topic.cyan(),
        result.name.yellow(),
        result.score.to_string().green()
      );

      for line in &result.matching_lines {
        println!("{line}");
      }
      println!();
    }
  }
}

/// Exact term matching search (the original implementation)
pub fn search_insights_exact(
  terms: &[String],
  topic_filter: Option<&str>,
  case_sensitive: bool,
  overview_only: bool,
) -> Result<()> {
  let results = collect_exact_results(terms, topic_filter, case_sensitive, overview_only)?;
  
  // Sort by relevance score (descending), then by topic/name for consistency
  let mut sorted_results = results;
  sorted_results.sort_by(|a, b| {
    b.score.cmp(&a.score).then_with(|| a.topic.cmp(&b.topic).then_with(|| a.name.cmp(&b.name)))
  });

  display_exact_search_results(&sorted_results, terms);
  Ok(())
}

/// Calculate semantic search result for a single insight
#[cfg(feature = "semantic")]
fn calculate_single_insight_semantic(
  topic_name: &str,
  insight_path: &Path,
  query_words: &HashSet<String>,
  overview_only: bool,
) -> Result<Option<SemanticSearchResult>> {
  let insight_name = insight_path.file_stem().and_then(|name| name.to_str()).unwrap_or("unknown");
  
  let content = fs::read_to_string(insight_path)?;
  let (overview, details) = parse_insight_content(&content)?;

  // Include topic and insight names for better matching (consistent with neural search)
  let search_text = if overview_only {
    format!("{topic_name} {insight_name} {overview}")
  } else {
    format!("{topic_name} {insight_name} {overview} {details}")
  };

  // Calculate semantic similarity
  let similarity = calculate_semantic_similarity(query_words, &search_text);

  // Only include results with meaningful similarity (> 0.2)
  if similarity > 0.2 {
    Ok(Some(SemanticSearchResult {
      topic: topic_name.to_string(),
      name: insight_name.to_string(),
      content: if overview_only { overview } else { format!("{overview}\n\n{details}") },
      similarity,
    }))
  } else {
    Ok(None)
  }
}

/// Process semantic search for insights in a single topic
#[cfg(feature = "semantic")]
fn process_topic_semantic_search(
  topic_path: &Path,
  query_words: &HashSet<String>,
  overview_only: bool,
) -> Result<Vec<SemanticSearchResult>> {
  let mut results = Vec::new();
  let topic_name = topic_path.file_name().and_then(|name| name.to_str()).unwrap_or("unknown");

  for insight_entry in fs::read_dir(topic_path)? {
    let insight_entry = insight_entry?;
    let insight_path = insight_entry.path();

    if insight_path.extension().and_then(|s| s.to_str()) != Some("md") {
      continue;
    }

    if let Ok(Some(result)) = calculate_single_insight_semantic(topic_name, &insight_path, query_words, overview_only) {
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

/// Semantic similarity search using advanced text analysis
#[cfg(feature = "semantic")]
#[allow(dead_code)]
pub fn search_insights_semantic(
  terms: &[String],
  topic_filter: Option<&str>,
  _case_sensitive: bool, // Note: Semantic search normalizes text, so case sensitivity doesn't apply
  overview_only: bool,
) -> Result<()> {
  let query = terms.join(" ");
  let insights_dir = get_insights_root()?;

  if !insights_dir.exists() {
    println!("No insights found. Create some insights first!");
    return Ok(());
  }

  let mut results = Vec::new();
  let query_words = extract_words(&query.to_lowercase());

  // Process all topics
  for entry in fs::read_dir(&insights_dir)? {
    let entry = entry?;
    let topic_path = entry.path();

    if !topic_path.is_dir() {
      continue;
    }

    let topic_name = topic_path.file_name().and_then(|name| name.to_str()).unwrap_or("unknown");

    // Apply topic filter
    if let Some(filter) = topic_filter {
      if topic_name != filter {
        continue;
      }
    }

    let topic_results = process_topic_semantic_search(&topic_path, &query_words, overview_only)?;
    results.extend(topic_results);
  }

  sort_semantic_results(&mut results);
  display_semantic_search_results(&results, terms);

  Ok(())
}


/// Extract words from text, filtering stop words  
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

  // Check if content is actually changing
  let content_changed = new_overview.is_some() || new_details.is_some();

  insight.update(new_overview, new_details)?;

  // Recompute embedding if content changed
  if content_changed {
    compute_and_set_embedding(&mut insight)?;

    // Save again with new embedding
    let file_path = insight.file_path()?;

    // Create frontmatter with updated embedding
    let frontmatter = crate::insight::FrontMatter {
      overview: insight.overview.clone(),
      embedding_version: insight.embedding_version.clone(),
      embedding: insight.embedding.clone(),
      embedding_text: insight.embedding_text.clone(),
      embedding_computed: insight.embedding_computed,
    };

    // Serialize frontmatter to YAML
    let yaml_content = serde_yaml::to_string(&frontmatter)?;

    // Write the updated content with new embedding
    let content = format!("---\n{}---\n\n# Details\n{}", yaml_content, insight.details);
    std::fs::write(&file_path, content)?;
  }

  println!("{} Updated insight {}/{}", "✓".green(), topic.cyan(), name.yellow());
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
    daemon_client::get_embedding_from_daemon(&content).await
      .map_err(|e| anyhow!("Failed to compute embedding for {}/{}: {}", insight.topic, insight.name, e))
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
    let similarity = cosine_similarity(query_embedding, &content_embedding);

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

/// Display neural search results
#[cfg(feature = "neural")]
fn display_neural_search_results(results: &[(crate::insight::Insight, f32)], terms: &[String]) {
  if results.is_empty() {
    println!("No matches found for: {}", terms.join(" "));
  } else {
    for (result, similarity) in results {
      println!(
        "=== {}/{} === (similarity: {:.1}%)",
        result.topic.cyan(),
        result.name.yellow(),
        similarity * 100.0
      );

      let full_content = format!("{}\n\n{}", result.overview, result.details);
      let lines: Vec<&str> = full_content.lines().collect();
      for line in lines.iter() {
        if !line.starts_with("---") {
          println!("{line}");
        }
      }
      println!();
    }
  }
}

/// Neural embedding search using ONNX
#[cfg(feature = "neural")]
#[allow(dead_code)]
pub fn search_insights_neural(
  terms: &[String],
  topic_filter: Option<&str>,
  _case_sensitive: bool, // Note: Neural embeddings normalize text, so case sensitivity doesn't apply
  overview_only: bool,
) -> Result<()> {
  let query_text = terms.join(" ");

  // Get query embedding using daemon for speed
  let rt = tokio::runtime::Runtime::new()?;
  let query_embedding = rt
    .block_on(async { daemon_client::get_embedding_from_daemon(&query_text).await })
    .map_err(|e| anyhow!("Failed to get query embedding: {}", e))?;

  // Get all insights and compute similarities using cached embeddings
  let insight_refs = get_insights(topic_filter)?;
  let (mut results, warnings) = rt.block_on(async {
    calculate_neural_similarities(insight_refs, &query_embedding, overview_only).await
  })?;

  // Sort by similarity (highest first)
  results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

  display_embedding_warnings(&warnings);
  display_neural_search_results(&results, terms);

  Ok(())
}

/// Create embedding - now uses daemon for speed!
#[cfg(feature = "neural")]
fn create_embedding(_session: &mut ort::session::Session, text: &str) -> Result<Vec<f32>> {
  // Use async runtime to call the new daemon-enabled function
  let rt = tokio::runtime::Runtime::new()?;
  rt.block_on(async { create_embedding_async(text).await })
}

/// Calculate cosine similarity between two embeddings
#[cfg(feature = "neural")]
fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
  if a.len() != b.len() {
    return 0.0;
  }

  let dot_product: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
  let magnitude_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
  let magnitude_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

  if magnitude_a == 0.0 || magnitude_b == 0.0 {
    0.0
  } else {
    dot_product / (magnitude_a * magnitude_b)
  }
}

/// Tier 2: Combined semantic + exact search (drops neural for speed)
#[cfg(feature = "semantic")]
pub fn search_insights_combined_semantic(
  terms: &[String],
  topic_filter: Option<&str>,
  case_sensitive: bool,
  overview_only: bool,
) -> Result<()> {
  let mut all_results = Vec::new();

  // Collect semantic search results
  let semantic_results =
    collect_semantic_results(terms, topic_filter, case_sensitive, overview_only)?;
  for result in semantic_results {
    all_results.push(CombinedSearchResult {
      topic: result.topic,
      name: result.name,
      content: result.content,
      score: result.similarity,
      search_type: "semantic".to_string(),
    });
  }

  // Collect exact search results
  let exact_results = collect_exact_results(terms, topic_filter, case_sensitive, overview_only)?;
  for result in exact_results {
    all_results.push(CombinedSearchResult {
      topic: result.topic,
      name: result.name,
      content: result.matching_lines.join("\n"),
      score: result.score as f32 * 0.3, // Scale exact scores to be comparable
      search_type: "exact".to_string(),
    });
  }

  display_combined_results(all_results, terms)
}

/// Tier 1: Combined neural + semantic + exact search (best results)
pub fn search_insights_combined_all(
  terms: &[String],
  topic_filter: Option<&str>,
  case_sensitive: bool,
  overview_only: bool,
) -> Result<()> {
  let mut all_results = Vec::new();

  // Collect neural search results (if available)
  #[cfg(feature = "neural")]
  {
    let neural_results =
      collect_neural_results(terms, topic_filter, case_sensitive, overview_only)?;
    for result in neural_results {
      all_results.push(CombinedSearchResult {
        topic: result.topic,
        name: result.name,
        content: result.content,
        score: result.similarity,
        search_type: "neural".to_string(),
      });
    }
  }

  // Collect semantic search results (if available)
  #[cfg(feature = "semantic")]
  {
    let semantic_results =
      collect_semantic_results(terms, topic_filter, case_sensitive, overview_only)?;
    for result in semantic_results {
      all_results.push(CombinedSearchResult {
        topic: result.topic,
        name: result.name,
        content: result.content,
        score: result.similarity * 0.8, // Scale semantic to be slightly lower than neural
        search_type: "semantic".to_string(),
      });
    }
  }

  // Collect exact search results
  let exact_results = collect_exact_results(terms, topic_filter, case_sensitive, overview_only)?;
  for result in exact_results {
    all_results.push(CombinedSearchResult {
      topic: result.topic,
      name: result.name,
      content: result.matching_lines.join("\n"),
      score: result.score as f32 * 0.3, // Scale exact scores to be comparable
      search_type: "exact".to_string(),
    });
  }

  // If no advanced search methods are available, fall back to exact only
  #[cfg(all(not(feature = "neural"), not(feature = "semantic")))]
  {
    return search_insights_exact(terms, topic_filter, case_sensitive, overview_only);
  }

  display_combined_results(all_results, terms)
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

  let query_words = extract_words(&terms.join(" ").to_lowercase());
  let mut results = Vec::new();

  for entry in fs::read_dir(&insights_dir)? {
    let topic_path = entry?.path();
    if !topic_path.is_dir() {
      continue;
    }

    let topic_name = topic_path.file_name().and_then(|name| name.to_str()).unwrap_or("unknown");
    if let Some(filter) = topic_filter {
      if topic_name != filter {
        continue;
      }
    }

    for insight_entry in fs::read_dir(&topic_path)? {
      let insight_path = insight_entry?.path();
      if let Ok(Some(result)) = process_insight_for_semantic_search(&insight_path, topic_name, &query_words, overview_only) {
        results.push(result);
      }
    }
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
  let (score, mut matching_lines) = calculate_term_matches(&search_content, terms, case_sensitive);

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
        &path, insight_name, topic_name, terms, case_sensitive, overview_only
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
    let topic_results = process_insight_files_in_topic(&topic_path, terms, case_sensitive, overview_only)?;
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
  let similarity = cosine_similarity(query_embedding, &content_embedding);

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

  Ok(insight_refs
    .into_iter()
    .filter_map(|(topic, name)| {
      Insight::load(&topic, &name)
        .ok()
        .and_then(|insight| process_insight_for_neural_search(&insight, &mut session, &query_embedding, overview_only).ok().flatten())
    })
    .collect())
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
  results: Vec<CombinedSearchResult>
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
fn sort_group_by_priority(group: &mut Vec<CombinedSearchResult>) {
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
  insight_groups: HashMap<(String, String), Vec<CombinedSearchResult>>
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
  insight_groups: HashMap<(String, String), Vec<CombinedSearchResult>>
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

/// Daemon client functions for invisible performance boost
#[cfg(feature = "neural")]
mod daemon_client {
  use anyhow::{anyhow, Result};
  use serde::{Deserialize, Serialize};
  use std::process::Stdio;
  use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
  use tokio::net::UnixStream;
  use tokio::process::Command;
  use tokio::time::{sleep, Duration};

  /// Request to compute embeddings (supports batching!)
  #[derive(Serialize, Deserialize)]
  struct EmbeddingRequest {
    texts: Vec<String>,
    id: String,
  }

  /// Response with computed embeddings (supports batching!)
  #[derive(Serialize, Deserialize)]
  struct EmbeddingResponse {
    embeddings: Vec<Vec<f32>>,
    id: String,
    error: Option<String>,
  }

  const SOCKET_PATH: &str = "/tmp/blizz-embeddings.sock";

  pub async fn get_embedding_from_daemon(text: &str) -> Result<Vec<f32>> {
    if let Ok(embedding) = request_embedding(text).await {
      return Ok(embedding);
    }

    start_daemon().await?;
    sleep(Duration::from_millis(500)).await;
    request_embedding(text).await
  }

  async fn request_embedding(text: &str) -> Result<Vec<f32>> {
    let mut stream = UnixStream::connect(SOCKET_PATH).await.map_err(|_| anyhow!("Daemon not running"))?;

    let request = EmbeddingRequest { 
      texts: vec![text.to_string()], 
      id: uuid::Uuid::new_v4().to_string() 
    };

    let request_json = serde_json::to_string(&request)?;
    stream.write_all(request_json.as_bytes()).await?;
    stream.write_all(b"\n").await?;

    let mut reader = BufReader::new(&mut stream);
    let mut line = String::new();
    reader.read_line(&mut line).await?;

    let response: EmbeddingResponse = serde_json::from_str(line.trim())?;

    if let Some(error) = response.error {
      return Err(anyhow!("Daemon error: {}", error));
    }

    Ok(response.embeddings.into_iter().next().unwrap_or_default())
  }

  async fn start_daemon() -> Result<()> {
    let executable_path = std::env::current_exe()?
      .parent()
      .ok_or_else(|| anyhow!("Could not find executable directory"))?
      .join("blizz-daemon");

    Command::new(executable_path)
      .stdout(Stdio::null())
      .stderr(Stdio::null())
      .stdin(Stdio::null())
      .spawn()
      .map_err(|e| anyhow!("Failed to start daemon: {}", e))?;

    Ok(())
  }
}

/// Enhanced create_embedding that uses daemon for speed
#[cfg(feature = "neural")]
async fn create_embedding_async(text: &str) -> Result<Vec<f32>> {
  // Try daemon first for speed
  if let Ok(embedding) = daemon_client::get_embedding_from_daemon(text).await {
    return Ok(embedding);
  }

  // Fallback to direct computation (current slow method)
  create_embedding_direct(text)
}

/// Initialize ONNX Runtime and load model
#[cfg(feature = "neural")]
fn init_onnx_model() -> Result<ort::session::Session> {
  use ort::session::{builder::GraphOptimizationLevel, Session};

  // Initialize ONNX Runtime (required!)
  ort::init()
    .with_name("blizz")
    .commit()
    .map_err(|e| anyhow!("Failed to initialize ONNX Runtime: {}", e))?;

  // Load model
  Session::builder()
    .map_err(|e| anyhow!("Failed to create session builder: {}", e))?
    .with_optimization_level(GraphOptimizationLevel::Level1)
    .map_err(|e| anyhow!("Failed to set optimization level: {}", e))?
    .with_intra_threads(1)
    .map_err(|e| anyhow!("Failed to set thread count: {}", e))?
    .commit_from_url("https://cdn.pyke.io/0/pyke:ort-rs/example-models@0.0.0/all-MiniLM-L6-v2.onnx")
    .map_err(|e| anyhow!("Failed to load model: {}", e))
}

/// Load and initialize tokenizer
#[cfg(feature = "neural")]
fn load_tokenizer() -> Result<tokenizers::Tokenizer> {
  use std::path::Path;
  
  let tokenizer_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("data").join("tokenizer.json");
  tokenizers::Tokenizer::from_file(&tokenizer_path)
    .map_err(|e| anyhow!("Failed to load tokenizer: {}", e))
}

/// Tokenize text and prepare input sequences
#[cfg(feature = "neural")]
fn prepare_input_sequences(text: &str, tokenizer: &tokenizers::Tokenizer) -> Result<(Vec<i64>, Vec<i64>)> {
  // Encode the text using the real tokenizer
  let encoding = tokenizer.encode(text, false).map_err(|e| anyhow!("Failed to encode text: {}", e))?;

  // Get token IDs and attention mask
  let token_ids: Vec<i64> = encoding.get_ids().iter().map(|&id| id as i64).collect();
  let attention_mask: Vec<i64> = encoding.get_attention_mask().iter().map(|&mask| mask as i64).collect();

  Ok((token_ids, attention_mask))
}

/// Pad or truncate sequences to correct length for model
#[cfg(feature = "neural")]
fn pad_sequences(token_ids: Vec<i64>, attention_mask: Vec<i64>, max_length: usize) -> (Vec<i64>, Vec<i64>) {
  let mut padded_ids = token_ids;
  let mut padded_mask = attention_mask;

  // Truncate if too long
  if padded_ids.len() > max_length {
    padded_ids.truncate(max_length);
    padded_mask.truncate(max_length);
  }

  // Pad if too short
  while padded_ids.len() < max_length {
    padded_ids.push(0); // PAD token
    padded_mask.push(0); // Attention mask 0 for padding
  }

  (padded_ids, padded_mask)
}

/// Run model inference and extract embeddings
#[cfg(feature = "neural")]
fn run_inference_and_extract(
  session: &mut ort::session::Session,
  padded_ids: Vec<i64>,
  padded_mask: Vec<i64>,
  max_length: usize,
) -> Result<Vec<f32>> {
  use ort::value::TensorRef;

  // Create tensors with proper shape [1, sequence_length]
  let ids_tensor = TensorRef::from_array_view(([1, max_length], &*padded_ids))?;
  let mask_tensor = TensorRef::from_array_view(([1, max_length], &*padded_mask))?;

  // Run inference
  let outputs = session.run(ort::inputs![ids_tensor, mask_tensor])?;

  // Extract embeddings from output (index 1 for sentence transformers contains pooled embeddings)
  let embedding_output = if outputs.len() > 1 { &outputs[1] } else { &outputs[0] };
  let embeddings = embedding_output.try_extract_array::<f32>()?.into_dimensionality::<ndarray::Ix2>()?;

  // Get the sentence embedding (should be shape [1, 384] for all-MiniLM-L6-v2)
  let embedding_view = embeddings.index_axis(ndarray::Axis(0), 0);
  let embedding_vec: Vec<f32> = embedding_view.iter().copied().collect();

  Ok(embedding_vec)
}

/// Direct embedding computation (the current slow method)
#[cfg(feature = "neural")]
fn create_embedding_direct(text: &str) -> Result<Vec<f32>> {
  let mut session = init_onnx_model()?;
  let tokenizer = load_tokenizer()?;
  let (token_ids, attention_mask) = prepare_input_sequences(text, &tokenizer)?;
  
  const MAX_LENGTH: usize = 512;
  let (padded_ids, padded_mask) = pad_sequences(token_ids, attention_mask, MAX_LENGTH);
  
  run_inference_and_extract(&mut session, padded_ids, padded_mask, MAX_LENGTH)
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
) -> Result<(bool, bool)> { // (processed, skipped)
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

/// Recompute embeddings for all insights
#[cfg(feature = "neural")]
pub fn index_insights(force: bool, missing_only: bool) -> Result<()> {
  use crate::insight::{get_insights, get_topics};

  let topics = get_topics()?;

  if topics.is_empty() {
    println!("No insights found to index.");
    return Ok(());
  }

  let mut processed = 0;
  let mut skipped = 0;
  let mut errors = 0;

  for topic in &topics {
    let insights = get_insights(Some(topic))?;
    println!("{} {}:", "◈".blue(), topic.cyan());

    for (_, insight_name) in insights {
      match process_single_insight_index(topic, &insight_name, force, missing_only) {
        Ok((was_processed, was_skipped)) => {
          if was_processed { processed += 1; }
          if was_skipped { skipped += 1; }
        }
        Err(e) => {
          eprintln!("    · Failed to process {insight_name}: {e}");
          errors += 1;
        }
      }
    }
  }

  print_index_summary(processed, skipped, errors, topics.len());
  Ok(())
}
