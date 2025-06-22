use crate::platform::{
  Discussion, FileDiff, GitPlatform, MergeRequest, MergeRequestState, Note, Pipeline, Repository,
  User,
};
use anyhow::{anyhow, Result};
use octocrab::Octocrab;

pub struct GitHubPlatform {
  #[allow(dead_code)]
  client: Octocrab,
}

impl GitHubPlatform {
  /// Create a new GitHub platform client with authentication
  pub fn new(token: Option<String>) -> Result<Self> {
    let client = if let Some(token) = token {
      Octocrab::builder().personal_token(token).build()?
    } else {
      // Try to use environment variable or default
      Octocrab::default()
    };

    Ok(Self { client })
  }

  /// Create a GitHub platform client from an existing Octocrab instance
  #[allow(dead_code)]
  pub fn from_client(client: Octocrab) -> Self {
    Self { client }
  }
}

#[async_trait::async_trait]
impl GitPlatform for GitHubPlatform {
  async fn get_repository(&self, owner: &str, repo: &str) -> Result<Repository> {
    let repo_data = self.client.repos(owner, repo).get().await?;
    
    Ok(Repository {
      owner: owner.to_string(),
      name: repo.to_string(),
      full_name: repo_data.full_name.unwrap_or_else(|| format!("{}/{}", owner, repo)),
      url: repo_data.html_url.map(|u| u.to_string()).unwrap_or_else(|| format!("https://github.com/{}/{}", owner, repo)),
    })
  }

  async fn get_merge_request(&self, owner: &str, repo: &str, number: u64) -> Result<MergeRequest> {
    let pr = self.client.pulls(owner, repo).get(number).await?;
    
    let state = match pr.state {
      Some(octocrab::models::IssueState::Open) => {
        if pr.draft == Some(true) {
          MergeRequestState::Draft
        } else {
          MergeRequestState::Open
        }
      }
      Some(octocrab::models::IssueState::Closed) => {
        if pr.merged_at.is_some() {
          MergeRequestState::Merged
        } else {
          MergeRequestState::Closed
        }
      }
      Some(_) => MergeRequestState::Open, // Handle any other states
      None => MergeRequestState::Open,
    };

    let author = if let Some(user) = pr.user {
      User {
        id: user.id.to_string(),
        username: user.login.clone(),
        display_name: user.name.clone().unwrap_or(user.login.clone()),
        avatar_url: Some(user.avatar_url.to_string()),
      }
    } else {
      User {
        id: "unknown".to_string(),
        username: "unknown".to_string(),
        display_name: "Unknown User".to_string(),
        avatar_url: None,
      }
    };

    let assignee = pr.assignee.map(|assignee| User {
      id: assignee.id.to_string(),
      username: assignee.login.clone(),
      display_name: assignee.name.unwrap_or(assignee.login.clone()),
      avatar_url: Some(assignee.avatar_url.to_string()),
    });

    Ok(MergeRequest {
      id: pr.id.to_string(),
      number,
      title: pr.title.unwrap_or_else(|| format!("Pull Request #{}", number)),
      description: pr.body,
      state,
      author,
      assignee,
      source_branch: pr.head.ref_field,
      target_branch: pr.base.ref_field,
      url: pr.html_url.map(|u| u.to_string()).unwrap_or_else(|| format!("https://github.com/{}/{}/pull/{}", owner, repo, number)),
      created_at: pr.created_at.unwrap_or_else(chrono::Utc::now),
      updated_at: pr.updated_at.unwrap_or_else(chrono::Utc::now),
    })
  }

  async fn get_discussions(
    &self,
    owner: &str,
    repo: &str,
    number: u64,
  ) -> Result<Vec<Discussion>> {
    let mut discussions = Vec::new();

    // Fetch regular issue comments (top-level comments on the PR)
    let issue_comments = self.client.issues(owner, repo).list_comments(number).send().await?;
    
    for comment in issue_comments {
      let author = User {
        id: comment.user.id.to_string(),
        username: comment.user.login.clone(),
        display_name: comment.user.name.clone().unwrap_or(comment.user.login.clone()),
        avatar_url: Some(comment.user.avatar_url.to_string()),
      };

      let note = Note {
        id: comment.id.to_string(),
        author,
        body: comment.body.unwrap_or_default(),
        created_at: comment.created_at,
        updated_at: comment.updated_at.unwrap_or(comment.created_at),
      };

      discussions.push(Discussion {
        id: comment.id.to_string(),
        resolved: false, // Issue comments don't have resolved status
        resolvable: false,
        file_path: None,
        line_number: None,
        notes: vec![note],
      });
    }

    // TODO: Add review comments once we figure out the correct API structure
    bentley::info("Review comments fetch temporarily disabled due to API differences");

    Ok(discussions)
  }

  async fn get_diffs(&self, _owner: &str, _repo: &str, _number: u64) -> Result<Vec<FileDiff>> {
    bentley::info("GitHub diffs fetch not yet implemented");
    Ok(vec![])
  }

  async fn get_pipelines(&self, _owner: &str, _repo: &str, _sha: &str) -> Result<Vec<Pipeline>> {
    // TODO: Implement workflow runs fetching - the Octocrab API for this is complex
    bentley::info("GitHub workflows fetch not yet implemented");
    Ok(vec![])
  }

  async fn add_comment(
    &self,
    _owner: &str,
    _repo: &str,
    _discussion_id: &str,
    _text: &str,
  ) -> Result<Note> {
    bentley::info("GitHub comment creation not yet implemented");
    Err(anyhow!("GitHub comment creation not yet implemented"))
  }

  async fn resolve_discussion(
    &self,
    _owner: &str,
    _repo: &str,
    _discussion_id: &str,
  ) -> Result<bool> {
    bentley::info("GitHub conversation resolution is supported but not yet implemented");
    Ok(false)
  }
}
