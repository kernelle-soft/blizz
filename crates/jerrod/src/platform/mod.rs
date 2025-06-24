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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ReactionType {
  ThumbsUp,   // 👍
  ThumbsDown, // 👎
  Laugh,      // 😄
  Hooray,     // 🎉
  Confused,   // 😕
  Heart,      // ❤️
  Rocket,     // 🚀
  Eyes,       // 👀
}

impl ReactionType {
  pub fn emoji(&self) -> &'static str {
    match self {
      ReactionType::ThumbsUp => "👍",
      ReactionType::ThumbsDown => "👎",
      ReactionType::Laugh => "😄",
      ReactionType::Hooray => "🎉",
      ReactionType::Confused => "😕",
      ReactionType::Heart => "❤️",
      ReactionType::Rocket => "🚀",
      ReactionType::Eyes => "👀",
    }
  }

  pub fn github_name(&self) -> &'static str {
    match self {
      ReactionType::ThumbsUp => "+1",
      ReactionType::ThumbsDown => "-1",
      ReactionType::Laugh => "laugh",
      ReactionType::Hooray => "hooray",
      ReactionType::Confused => "confused",
      ReactionType::Heart => "heart",
      ReactionType::Rocket => "rocket",
      ReactionType::Eyes => "eyes",
    }
  }
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
  #[allow(dead_code)]
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

  /// Add a reaction to a comment/discussion
  async fn add_reaction(
    &self,
    owner: &str,
    repo: &str,
    comment_id: &str,
    reaction: ReactionType,
  ) -> Result<bool>;

  /// Remove a reaction from a comment/discussion
  #[allow(dead_code)]
  async fn remove_reaction(
    &self,
    owner: &str,
    repo: &str,
    comment_id: &str,
    reaction: ReactionType,
  ) -> Result<bool>;

  /// Get reactions for a comment/discussion
  #[allow(dead_code)]
  async fn get_reactions(
    &self,
    owner: &str,
    repo: &str,
    comment_id: &str,
  ) -> Result<Vec<ReactionType>>;

  /// Add a review comment reply (platform-specific)
  async fn add_review_comment_reply(
    &self,
    owner: &str,
    repo: &str,
    pr_number: u64,
    comment_id: &str,
    text: &str,
  ) -> Result<Note>;

  /// Format a URL for a specific comment/thread within a merge request
  fn format_comment_url(&self, mr_url: &str, comment_id: &str) -> String;

  /// Format a URL for a merge request/pull request
  #[allow(dead_code)]
  fn format_merge_request_url(&self, owner: &str, repo: &str, number: u64) -> String;
}

/// Strategy pattern factory - creates appropriate platform implementation
pub async fn create_platform(platform_name: &str) -> Result<Box<dyn GitPlatform>> {
  match platform_name {
    "github" => {
      let github_platform = github::GitHubPlatform::new().await?;
      Ok(Box::new(github_platform))
    }
    "gitlab" => {
      // TODO: Implement GitLab platform
      anyhow::bail!("GitLab platform not yet implemented")
    }
    _ => {
      anyhow::bail!("Unsupported platform: {}", platform_name)
    }
  }
}
