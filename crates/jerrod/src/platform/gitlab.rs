use crate::auth::get_gitlab_token;
use crate::platform::{
  Discussion, FileDiff, GitPlatform, MergeRequest, MergeRequestState, Note, Pipeline,
  PipelineStatus, ReactionType, Repository, User,
};
use anyhow::Result;
use gitlab::api::AsyncQuery;
use gitlab::{AsyncGitlab, GitlabBuilder};
use serde::Deserialize;

pub struct GitLabPlatform {
  client: AsyncGitlab,
  host: String,
}

impl GitLabPlatform {
  pub async fn new() -> Result<Self> {
    Self::new_with_host("https://gitlab.com").await
  }

  pub async fn new_with_host(host: &str) -> Result<Self> {
    let token = get_gitlab_token().await?;
    let client = GitlabBuilder::new(host, token).build_async().await?;

    Ok(Self { client, host: host.to_string() })
  }

  /// Create a GitLab platform client from an existing client instance  
  #[allow(dead_code)]
  pub fn from_client(client: AsyncGitlab, host: String) -> Self {
    Self { client, host }
  }

  /// Get the project full path from owner and repo
  fn get_project_path(&self, owner: &str, repo: &str) -> String {
    format!("{}/{}", owner, repo)
  }

  /// Convert GitLab merge request state to our common type
  fn convert_mr_state(&self, state: &str) -> MergeRequestState {
    match state {
      "opened" => MergeRequestState::Open,
      "closed" => MergeRequestState::Closed,
      "merged" => MergeRequestState::Merged,
      "draft" => MergeRequestState::Draft,
      _ => MergeRequestState::Open, // Default fallback
    }
  }

  /// Convert GitLab pipeline status to our common type
  fn convert_pipeline_status(&self, status: &str) -> PipelineStatus {
    match status {
      "running" => PipelineStatus::Running,
      "success" => PipelineStatus::Success,
      "failed" => PipelineStatus::Failed,
      "canceled" => PipelineStatus::Canceled,
      "skipped" => PipelineStatus::Skipped,
      "pending" => PipelineStatus::Pending,
      _ => PipelineStatus::Pending, // Default fallback
    }
  }
}

#[async_trait::async_trait]
impl GitPlatform for GitLabPlatform {
  async fn get_repository(&self, owner: &str, repo: &str) -> Result<Repository> {
    let project_path = self.get_project_path(owner, repo);

    let endpoint =
      gitlab::api::projects::Project::builder().project(project_path.clone()).build()?;

    let project: ProjectInfo = endpoint.query_async(&self.client).await?;

    Ok(Repository {
      owner: owner.to_string(),
      name: repo.to_string(),
      full_name: project_path,
      url: project.web_url,
    })
  }

  async fn get_merge_request(&self, owner: &str, repo: &str, number: u64) -> Result<MergeRequest> {
    let project_path = self.get_project_path(owner, repo);

    let endpoint = gitlab::api::projects::merge_requests::MergeRequest::builder()
      .project(project_path.clone())
      .merge_request(number)
      .build()?;

    let mr: MergeRequestInfo = endpoint.query_async(&self.client).await?;

    Ok(MergeRequest {
      id: mr.id.to_string(),
      number: mr.iid,
      title: mr.title,
      description: mr.description,
      state: self.convert_mr_state(&mr.state),
      author: User {
        id: mr.author.id.to_string(),
        username: mr.author.username.clone(),
        display_name: mr.author.name.unwrap_or(mr.author.username),
        avatar_url: mr.author.avatar_url,
      },
      assignee: mr.assignee.map(|assignee| User {
        id: assignee.id.to_string(),
        username: assignee.username.clone(),
        display_name: assignee.name.unwrap_or(assignee.username),
        avatar_url: assignee.avatar_url,
      }),
      source_branch: mr.source_branch,
      target_branch: mr.target_branch,
      url: mr.web_url,
      created_at: mr.created_at,
      updated_at: mr.updated_at,
    })
  }

  async fn get_discussions(&self, owner: &str, repo: &str, number: u64) -> Result<Vec<Discussion>> {
    // TODO: Implement GitLab discussions API
    // For now, return empty list as placeholder
    let _project_path = self.get_project_path(owner, repo);
    let _mr_number = number;

    tracing::warn!("GitLab discussions not yet implemented, returning empty list");
    Ok(Vec::new())
  }

  async fn get_diffs(&self, owner: &str, repo: &str, number: u64) -> Result<Vec<FileDiff>> {
    let project_path = self.get_project_path(owner, repo);

    let endpoint = gitlab::api::projects::merge_requests::MergeRequestDiffs::builder()
      .project(project_path)
      .merge_request(number)
      .build()?;

    let diffs: Vec<ChangeInfo> = endpoint.query_async(&self.client).await?;

    let mut result = Vec::new();
    for change in diffs {
      result.push(FileDiff {
        old_path: change.old_path,
        new_path: change.new_path,
        diff: change.diff,
      });
    }

    Ok(result)
  }

  async fn get_pipelines(&self, owner: &str, repo: &str, sha: &str) -> Result<Vec<Pipeline>> {
    let project_path = self.get_project_path(owner, repo);

    let endpoint = gitlab::api::projects::pipelines::Pipelines::builder()
      .project(project_path)
      .sha(sha)
      .build()?;

    let pipelines: Vec<PipelineInfo> = endpoint.query_async(&self.client).await?;

    let mut result = Vec::new();
    for pipeline in pipelines {
      result.push(Pipeline {
        id: pipeline.id.to_string(),
        status: self.convert_pipeline_status(&pipeline.status),
        ref_name: pipeline.r#ref,
        sha: pipeline.sha,
        url: pipeline.web_url,
        created_at: pipeline.created_at,
        updated_at: pipeline.updated_at,
      });
    }

    Ok(result)
  }

  async fn add_comment(
    &self,
    _owner: &str,
    _repo: &str,
    _discussion_id: &str,
    _text: &str,
  ) -> Result<Note> {
    // TODO: Implement GitLab comment creation
    tracing::warn!("GitLab add_comment not yet implemented");
    anyhow::bail!("GitLab add_comment not yet implemented")
  }

  async fn resolve_discussion(
    &self,
    _owner: &str,
    _repo: &str,
    _discussion_id: &str,
  ) -> Result<bool> {
    // TODO: Implement GitLab discussion resolution
    tracing::warn!("GitLab resolve_discussion not yet implemented");
    anyhow::bail!("GitLab resolve_discussion not yet implemented")
  }

  async fn add_reaction(
    &self,
    _owner: &str,
    _repo: &str,
    _comment_id: &str,
    _reaction: ReactionType,
  ) -> Result<bool> {
    // TODO: Implement GitLab reactions
    tracing::warn!("GitLab add_reaction not yet implemented");
    anyhow::bail!("GitLab add_reaction not yet implemented")
  }

  async fn remove_reaction(
    &self,
    _owner: &str,
    _repo: &str,
    _comment_id: &str,
    _reaction: ReactionType,
  ) -> Result<bool> {
    // TODO: Implement GitLab reaction removal
    tracing::warn!("GitLab remove_reaction not yet implemented");
    anyhow::bail!("GitLab remove_reaction not yet implemented")
  }

  async fn get_reactions(
    &self,
    _owner: &str,
    _repo: &str,
    _comment_id: &str,
  ) -> Result<Vec<ReactionType>> {
    // TODO: Implement GitLab reaction retrieval
    tracing::warn!("GitLab get_reactions not yet implemented");
    Ok(Vec::new())
  }

  async fn add_review_comment_reply(
    &self,
    _owner: &str,
    _repo: &str,
    _pr_number: u64,
    _comment_id: &str,
    _text: &str,
  ) -> Result<Note> {
    // TODO: Implement GitLab review comment replies
    tracing::warn!("GitLab add_review_comment_reply not yet implemented");
    anyhow::bail!("GitLab add_review_comment_reply not yet implemented")
  }

  fn format_comment_url(&self, mr_url: &str, comment_id: &str) -> String {
    // Parse comment_id to extract note ID
    let parts: Vec<&str> = comment_id.split('_').collect();
    if parts.len() >= 3 {
      format!("{}#note_{}", mr_url, parts[2])
    } else {
      mr_url.to_string()
    }
  }

  fn format_merge_request_url(&self, owner: &str, repo: &str, number: u64) -> String {
    let base_url = if self.host.starts_with("http") {
      self.host.clone()
    } else {
      format!("https://{}", self.host)
    };

    format!("{}/{}/{}/-/merge_requests/{}", base_url, owner, repo, number)
  }
}

impl GitLabPlatform {
  /// Convert GitLab emoji name to our ReactionType
  #[allow(dead_code)]
  fn gitlab_name_to_reaction(&self, name: &str) -> Option<ReactionType> {
    match name {
      "thumbsup" => Some(ReactionType::ThumbsUp),
      "thumbsdown" => Some(ReactionType::ThumbsDown),
      "smile" => Some(ReactionType::Laugh),
      "tada" => Some(ReactionType::Hooray),
      "confused" => Some(ReactionType::Confused),
      "heart" => Some(ReactionType::Heart),
      "rocket" => Some(ReactionType::Rocket),
      "eyes" => Some(ReactionType::Eyes),
      _ => None,
    }
  }
}

// Data structures for GitLab API responses
#[derive(Debug, Deserialize)]
struct ProjectInfo {
  id: u64,
  name: String,
  web_url: String,
}

#[derive(Debug, Deserialize)]
struct UserInfo {
  id: u64,
  username: String,
  name: Option<String>,
  avatar_url: Option<String>,
}

#[derive(Debug, Deserialize)]
struct MergeRequestInfo {
  id: u64,
  iid: u64,
  title: String,
  description: Option<String>,
  state: String,
  author: UserInfo,
  assignee: Option<UserInfo>,
  source_branch: String,
  target_branch: String,
  web_url: String,
  created_at: chrono::DateTime<chrono::Utc>,
  updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Deserialize)]
struct ChangeInfo {
  old_path: Option<String>,
  new_path: String,
  diff: String,
}

#[derive(Debug, Deserialize)]
struct PipelineInfo {
  id: u64,
  status: String,
  r#ref: String,
  sha: String,
  web_url: Option<String>,
  created_at: chrono::DateTime<chrono::Utc>,
  updated_at: chrono::DateTime<chrono::Utc>,
}
