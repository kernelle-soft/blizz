use crate::auth::get_gitlab_token;
use crate::platform::{
  Discussion, FileDiff, GitPlatform, MergeRequest, MergeRequestState, Note, Pipeline,
  PipelineStatus, ReactionType, Repository, User,
};
use anyhow::Result;
use gitlab::api::projects::merge_requests::discussions::MergeRequestDiscussions;
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
    let project_path = self.get_project_path(owner, repo);

    // Use REST API to fetch merge request discussions
    let endpoint =
      MergeRequestDiscussions::builder().project(project_path).merge_request(number).build()?;

    let discussions_response: Vec<DiscussionInfo> = endpoint.query_async(&self.client).await?;

    let mut result = Vec::new();
    for discussion in discussions_response {
      // Extract position info from first note if available
      let (file_path, line_number) = if let Some(first_note) = discussion.notes.first() {
        if let Some(position) = &first_note.position {
          (position.new_path.clone(), position.new_line.map(|l| l as u32))
        } else {
          (None, None)
        }
      } else {
        (None, None)
      };

      let notes = discussion
        .notes
        .into_iter()
        .map(|note| Note {
          id: note.id.to_string(),
          author: User {
            id: note.author.id.to_string(),
            username: note.author.username.clone(),
            display_name: note.author.name.unwrap_or(note.author.username),
            avatar_url: note.author.avatar_url,
          },
          body: note.body,
          created_at: note.created_at,
          updated_at: note.updated_at,
        })
        .collect();

      result.push(Discussion {
        id: discussion.id,
        resolved: discussion.resolved.unwrap_or(false),
        resolvable: discussion.resolvable.unwrap_or(false),
        file_path,
        line_number,
        notes,
      });
    }

    Ok(result)
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
    // For now, return a placeholder implementation
    // This would require parsing the discussion_id and finding the MR number
    // Then using gitlab::api::projects::merge_requests::CreateMergeRequestDiscussionNote
    tracing::warn!("GitLab add_comment implementation needed - REST API integration pending");
    anyhow::bail!("GitLab add_comment not yet implemented with REST API")
  }

  async fn resolve_discussion(
    &self,
    _owner: &str,
    _repo: &str,
    _discussion_id: &str,
  ) -> Result<bool> {
    // For now, return a placeholder implementation
    // This would use gitlab::api::projects::merge_requests::EditMergeRequestDiscussion
    tracing::warn!(
      "GitLab resolve_discussion implementation needed - REST API integration pending"
    );
    anyhow::bail!("GitLab resolve_discussion not yet implemented with REST API")
  }

  async fn add_reaction(
    &self,
    _owner: &str,
    _repo: &str,
    _comment_id: &str,
    _reaction: ReactionType,
  ) -> Result<bool> {
    // For now, return a placeholder implementation
    // This would use gitlab::api::projects::merge_requests::CreateMergeRequestAwardEmoji
    tracing::warn!("GitLab add_reaction implementation needed - REST API integration pending");
    anyhow::bail!("GitLab add_reaction not yet implemented with REST API")
  }

  async fn remove_reaction(
    &self,
    _owner: &str,
    _repo: &str,
    _comment_id: &str,
    _reaction: ReactionType,
  ) -> Result<bool> {
    // For now, return a placeholder implementation
    tracing::warn!("GitLab remove_reaction implementation needed - REST API integration pending");
    anyhow::bail!("GitLab remove_reaction not yet implemented with REST API")
  }

  async fn get_reactions(
    &self,
    _owner: &str,
    _repo: &str,
    _comment_id: &str,
  ) -> Result<Vec<ReactionType>> {
    // For now, return a placeholder implementation
    tracing::warn!("GitLab get_reactions implementation needed - REST API integration pending");
    Ok(Vec::new())
  }

  async fn add_review_comment_reply(
    &self,
    owner: &str,
    repo: &str,
    _pr_number: u64,
    comment_id: &str,
    text: &str,
  ) -> Result<Note> {
    // In GitLab, this is the same as add_comment - replying to a discussion
    self.add_comment(owner, repo, comment_id, text).await
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
  #[allow(dead_code)]
  id: u64,
  #[allow(dead_code)]
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

// REST API structures for discussions
#[derive(Debug, Deserialize, Clone)]
struct DiscussionInfo {
  id: String,
  #[allow(dead_code)]
  individual_note: bool,
  resolved: Option<bool>,
  resolvable: Option<bool>,
  notes: Vec<NoteInfo>,
}

#[derive(Debug, Deserialize, Clone)]
struct NoteInfo {
  id: u64,
  #[serde(rename = "type")]
  #[allow(dead_code)]
  note_type: Option<String>,
  body: String,
  author: AuthorInfo,
  created_at: chrono::DateTime<chrono::Utc>,
  updated_at: chrono::DateTime<chrono::Utc>,
  #[allow(dead_code)]
  system: bool,
  position: Option<PositionInfo>,
  #[allow(dead_code)]
  resolved: Option<bool>,
  #[allow(dead_code)]
  resolvable: Option<bool>,
}

#[derive(Debug, Deserialize, Clone)]
struct AuthorInfo {
  id: u64,
  username: String,
  name: Option<String>,
  avatar_url: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
struct PositionInfo {
  new_path: Option<String>,
  new_line: Option<i32>,
  #[allow(dead_code)]
  old_path: Option<String>,
  #[allow(dead_code)]
  old_line: Option<i32>,
}
