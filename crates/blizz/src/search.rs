use anyhow::Result;
use crate::insight::Insight; 

struct SearchResult {
  insight: Insight,
  score: f32,
} 

struct SearchOptions {
  topic: Option<&str>,
  case_sensitive: bool,
  overview_only: bool,
  semantic: bool,
  exact: bool,
}

pub fn search_insights(query: &[&str], options: SearchOptions) -> None {

}

pub fn embedding_search(query: &str) -> Result<Vec<Insight>> {

}

pub fn semantic_search(query: &str) -> Result<Vec<Insight>> {

}

pub fn exact_search(query: &str) -> Result<Vec<Insight>> {

}