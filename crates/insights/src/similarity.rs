use std::collections::HashSet;

// violet ignore chunk
/// Common English stop words to filter out
pub const STOP_WORDS: &[&str] = &[
  "the", "a", "an", "and", "or", "but", "in", "on", "at", "to", "for", "of", "with", "by", "over",
  "is", "are", "was", "were", "be", "been", "have", "has", "had", "do", "does", "did", "will",
  "would", "could", "should", "you", "your", "we", "our", "us", "they", "them", "their", "it",
  "its",
];

/// Calculate cosine similarity between two embeddings
pub fn cosine(a: &[f32], b: &[f32]) -> f32 {
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

/// Calculate semantic similarity using Jaccard + frequency analysis
pub fn semantic(query_words: &HashSet<String>, content: &str) -> f32 {
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

/// Extract meaningful words from text, filtering out common stop words
pub fn extract_words(text: &str) -> HashSet<String> {
  text
    .split_whitespace()
    .map(|word| word.trim_matches(|c: char| !c.is_alphanumeric()).to_lowercase())
    .filter(|word| !word.is_empty() && !STOP_WORDS.contains(&word.as_str()))
    .collect()
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_extract_words_basic() {
    let text = "The quick brown fox jumps over the lazy dog";
    let words = extract_words(text);

    // Should exclude stop words like "the", "over"
    assert!(!words.contains("the"));
    assert!(!words.contains("over"));

    // Should include meaningful words
    assert!(words.contains("quick"));
    assert!(words.contains("brown"));
    assert!(words.contains("fox"));
    assert!(words.contains("jumps"));
  }

  #[test]
  fn test_extract_words_punctuation() {
    let text = "Hello, world! How are you?";
    let words = extract_words(text);

    assert!(words.contains("hello"));
    assert!(words.contains("world"));
    assert!(words.contains("how"));
    assert!(!words.contains("are")); // Stop word
    assert!(!words.contains("you")); // Stop word
  }

  #[test]
  fn test_extract_words_empty() {
    let text = "";
    let words = extract_words(text);
    assert!(words.is_empty());
  }

  #[test]
  fn test_extract_words_only_stop_words() {
    let text = "the and or but";
    let words = extract_words(text);
    assert!(words.is_empty());
  }

  #[test]
  fn test_calculate_semantic_similarity_exact_match() {
    let query_words: HashSet<String> =
      ["machine", "learning"].iter().map(|s| s.to_string()).collect();
    let content = "machine learning algorithms";

    let similarity = semantic(&query_words, content);
    assert!(similarity > 0.6); // Should be high similarity
  }

  #[test]
  fn test_calculate_semantic_similarity_partial_match() {
    let query_words: HashSet<String> =
      ["machine", "learning"].iter().map(|s| s.to_string()).collect();
    let content = "machine algorithms and data science";

    let similarity = semantic(&query_words, content);
    assert!(similarity > 0.2 && similarity < 0.6); // Should be medium similarity
  }

  #[test]
  fn test_calculate_semantic_similarity_no_match() {
    let query_words: HashSet<String> =
      ["machine", "learning"].iter().map(|s| s.to_string()).collect();
    let content = "completely different topic about cooking";

    let similarity = semantic(&query_words, content);
    assert!(similarity < 0.1); // Should be very low similarity
  }

  #[test]
  fn test_calculate_semantic_similarity_empty_query() {
    let query_words: HashSet<String> = HashSet::new();
    let content = "some content here";

    let similarity = semantic(&query_words, content);
    assert_eq!(similarity, 0.0);
  }

  #[test]
  fn test_calculate_semantic_similarity_empty_content() {
    let query_words: HashSet<String> = ["test"].iter().map(|s| s.to_string()).collect();
    let content = "";

    let similarity = semantic(&query_words, content);
    assert_eq!(similarity, 0.0);
  }

  #[test]
  fn test_calculate_semantic_similarity_frequency_boost() {
    let query_words: HashSet<String> = ["test"].iter().map(|s| s.to_string()).collect();
    let content_single = "test algorithm";
    let content_multiple = "test test test algorithm";

    let similarity_single = semantic(&query_words, content_single);
    let similarity_multiple = semantic(&query_words, content_multiple);

    // Multiple occurrences should get frequency boost
    assert!(similarity_multiple > similarity_single);
  }

  #[test]
  fn test_semantic_search_result_creation() {
    let query_words: HashSet<String> =
      ["machine", "learning"].iter().map(|s| s.to_string()).collect();
    let content = "machine learning algorithms";

    let similarity = semantic(&query_words, content);
    assert!(similarity > 0.6); // Should be high similarity
  }
}
