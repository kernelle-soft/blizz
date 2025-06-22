use anyhow::Result;
use serde::{Deserialize, Serialize};

pub mod detection;
pub mod github;

/// Common types used across all platforms
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Repository {
  pub owner: String,
  pub name: String,
  pub full_name: String,
  pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
  pub id: String,
  pub username: String,
  pub display_name: String,
  pub avatar_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MergeRequestState {
  Open,
  Closed,
  Merged,
  Draft,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MergeRequest {
  pub id: String,
  pub number: u64,
  pub title: String,
  pub description: Option<String>,
  pub state: MergeRequestState,
  pub author: User,
  pub assignee: Option<User>,
  pub source_branch: String,
  pub target_branch: String,
  pub url: String,
  pub created_at: chrono::DateTime<chrono::Utc>,
  pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Note {
  pub id: String,
  pub author: User,
  pub body: String,
  pub created_at: chrono::DateTime<chrono::Utc>,
  pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Discussion {
  pub id: String,
  pub resolved: bool,
  pub resolvable: bool,
  pub file_path: Option<String>,
  pub line_number: Option<u32>,
  pub notes: Vec<Note>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileDiff {
  pub old_path: Option<String>,
  pub new_path: String,
  pub diff: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PipelineStatus {
  Running,
  Success,
  Failed,
  Canceled,
  Skipped,
  Pending,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pipeline {
  pub id: String,
  pub status: PipelineStatus,
  pub ref_name: String,
  pub sha: String,
  pub url: Option<String>,
  pub created_at: chrono::DateTime<chrono::Utc>,
  pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// Platform abstraction trait - start simple and expand later
#[async_trait::async_trait]
pub trait GitPlatform {
  /// Get repository information
  async fn get_repository(&self, owner: &str, repo: &str) -> Result<Repository>;

  /// Get merge request/pull request information
  async fn get_merge_request(&self, owner: &str, repo: &str, number: u64) -> Result<MergeRequest>;

  /// Get discussions/review comments for a merge request
  async fn get_discussions(&self, owner: &str, repo: &str, number: u64) -> Result<Vec<Discussion>>;

  /// Get file diffs for a merge request
  async fn get_diffs(&self, owner: &str, repo: &str, number: u64) -> Result<Vec<FileDiff>>;

  /// Get pipeline/workflow information
  async fn get_pipelines(&self, owner: &str, repo: &str, sha: &str) -> Result<Vec<Pipeline>>;

  /// Add a comment to a discussion thread
  async fn add_comment(
    &self,
    owner: &str,
    repo: &str,
    discussion_id: &str,
    text: &str,
  ) -> Result<Note>;

  /// Mark a discussion as resolved (where supported)
  async fn resolve_discussion(&self, owner: &str, repo: &str, discussion_id: &str) -> Result<bool>;
}
