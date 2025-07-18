use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use jerrod::platform::{
  Discussion, FileDiff, GitPlatform, MergeRequest, MergeRequestState, Note, Pipeline, ReactionType,
  Repository, User,
};
use std::collections::HashMap;

/// Mock GitHub implementation for testing
pub struct MockGitHub {
  pub repositories: HashMap<(String, String), Repository>,
  pub merge_requests: HashMap<String, MergeRequest>,
  pub discussions: HashMap<String, Vec<Discussion>>,
  #[allow(dead_code)]
  pub pipelines: HashMap<String, Vec<Pipeline>>,
  pub users: HashMap<String, User>,
  pub notes: HashMap<String, Vec<Note>>,
  pub should_fail: bool,
  pub api_call_count: u32,
}

impl Default for MockGitHub {
  fn default() -> Self {
    Self::new()
  }
}

impl MockGitHub {
  pub fn new() -> Self {
    Self {
      repositories: HashMap::new(),
      merge_requests: HashMap::new(),
      discussions: HashMap::new(),
      pipelines: HashMap::new(),
      users: HashMap::new(),
      notes: HashMap::new(),
      should_fail: false,
      api_call_count: 0,
    }
  }

  pub fn with_test_data() -> Self {
    let mut mock = Self::new();

    // Add test repository
    let repo = Repository {
      owner: "test_owner".to_string(),
      name: "test_repo".to_string(),
      full_name: "test_owner/test_repo".to_string(),
      url: "https://github.com/test_owner/test_repo".to_string(),
    };
    mock.repositories.insert(("test_owner".to_string(), "test_repo".to_string()), repo);

    // Add test user
    let user = User {
      id: "user123".to_string(),
      username: "testuser".to_string(),
      display_name: "Test User".to_string(),
      avatar_url: Some("https://avatar.example.com/user123".to_string()),
    };
    mock.users.insert("testuser".to_string(), user.clone());

    // Add test MR
    let created_at =
      DateTime::parse_from_rfc3339("2023-01-01T00:00:00Z").unwrap().with_timezone(&Utc);
    let updated_at =
      DateTime::parse_from_rfc3339("2023-01-01T12:00:00Z").unwrap().with_timezone(&Utc);

    let mr = MergeRequest {
      id: "mr123".to_string(),
      number: 123,
      title: "Test Pull Request".to_string(),
      description: Some("Test description".to_string()),
      state: MergeRequestState::Open,
      url: "https://github.com/test_owner/test_repo/pull/123".to_string(),
      source_branch: "feature-branch".to_string(),
      target_branch: "main".to_string(),
      author: user.clone(),
      assignee: None,
      created_at,
      updated_at,
    };
    mock.merge_requests.insert("mr123".to_string(), mr);

    // Add test note
    let note_created_at =
      DateTime::parse_from_rfc3339("2023-01-01T10:00:00Z").unwrap().with_timezone(&Utc);
    let note = Note {
      id: "note123".to_string(),
      author: user.clone(),
      body: "This needs improvement".to_string(),
      created_at: note_created_at,
      updated_at: note_created_at,
    };

    // Add test discussions
    let discussion = Discussion {
      id: "disc123".to_string(),
      resolved: false,
      resolvable: true,
      file_path: Some("src/main.rs".to_string()),
      line_number: Some(42),
      notes: vec![note.clone()],
    };
    mock.discussions.insert("mr123".to_string(), vec![discussion]);
    mock.notes.insert("disc123".to_string(), vec![note]);

    mock
  }

  pub fn set_should_fail(&mut self, should_fail: bool) {
    self.should_fail = should_fail;
  }

  #[allow(dead_code)]
  pub fn get_api_call_count(&self) -> u32 {
    self.api_call_count
  }

  #[allow(dead_code)]
  fn increment_call_count(&mut self) {
    self.api_call_count += 1;
  }
}

#[async_trait]
impl GitPlatform for MockGitHub {
  async fn get_repository(&self, owner: &str, repo: &str) -> Result<Repository> {
    if self.should_fail {
      return Err(anyhow::anyhow!("Mock failure"));
    }

    self
      .repositories
      .get(&(owner.to_string(), repo.to_string()))
      .cloned()
      .ok_or_else(|| anyhow::anyhow!("Repository not found"))
  }

  async fn get_merge_request(
    &self,
    _owner: &str,
    _repo: &str,
    number: u64,
  ) -> Result<MergeRequest> {
    if self.should_fail {
      return Err(anyhow::anyhow!("Mock failure"));
    }

    let key = format!("mr{number}");
    self.merge_requests.get(&key).cloned().ok_or_else(|| anyhow::anyhow!("Merge request not found"))
  }

  async fn get_discussions(
    &self,
    _owner: &str,
    _repo: &str,
    number: u64,
  ) -> Result<Vec<Discussion>> {
    if self.should_fail {
      return Err(anyhow::anyhow!("Mock failure"));
    }

    let key = format!("mr{number}");
    Ok(self.discussions.get(&key).cloned().unwrap_or_default())
  }

  async fn get_diffs(&self, _owner: &str, _repo: &str, _number: u64) -> Result<Vec<FileDiff>> {
    if self.should_fail {
      return Err(anyhow::anyhow!("Mock failure"));
    }

    // Return empty diffs for now
    Ok(vec![])
  }

  async fn get_pipelines(&self, _owner: &str, _repo: &str, _sha: &str) -> Result<Vec<Pipeline>> {
    if self.should_fail {
      return Err(anyhow::anyhow!("Mock failure"));
    }

    // Return empty pipelines for now
    Ok(vec![])
  }

  async fn add_comment(
    &self,
    _owner: &str,
    _repo: &str,
    _discussion_id: &str,
    text: &str,
  ) -> Result<Note> {
    if self.should_fail {
      return Err(anyhow::anyhow!("Mock failure"));
    }

    let now = Utc::now();
    let user = User {
      id: "mock_user".to_string(),
      username: "mock_user".to_string(),
      display_name: "Mock User".to_string(),
      avatar_url: None,
    };

    Ok(Note {
      id: format!("comment_{}", self.api_call_count),
      author: user,
      body: text.to_string(),
      created_at: now,
      updated_at: now,
    })
  }

  async fn resolve_discussion(
    &self,
    _owner: &str,
    _repo: &str,
    _discussion_id: &str,
  ) -> Result<bool> {
    if self.should_fail {
      return Err(anyhow::anyhow!("Mock failure"));
    }

    Ok(true)
  }

  async fn add_reaction(
    &self,
    _owner: &str,
    _repo: &str,
    _comment_id: &str,
    _reaction: ReactionType,
  ) -> Result<bool> {
    if self.should_fail {
      return Err(anyhow::anyhow!("Mock failure"));
    }

    Ok(true)
  }

  async fn remove_reaction(
    &self,
    _owner: &str,
    _repo: &str,
    _comment_id: &str,
    _reaction: ReactionType,
  ) -> Result<bool> {
    if self.should_fail {
      return Err(anyhow::anyhow!("Mock failure"));
    }

    Ok(true)
  }

  async fn get_reactions(
    &self,
    _owner: &str,
    _repo: &str,
    _comment_id: &str,
  ) -> Result<Vec<ReactionType>> {
    if self.should_fail {
      return Err(anyhow::anyhow!("Mock failure"));
    }

    Ok(vec![])
  }

  async fn add_review_comment_reply(
    &self,
    _owner: &str,
    _repo: &str,
    _pr_number: u64,
    _comment_id: &str,
    text: &str,
  ) -> Result<Note> {
    if self.should_fail {
      return Err(anyhow::anyhow!("Mock failure"));
    }

    let now = Utc::now();
    let user = User {
      id: "mock_user".to_string(),
      username: "mock_user".to_string(),
      display_name: "Mock User".to_string(),
      avatar_url: None,
    };

    Ok(Note {
      id: format!("reply_{}", self.api_call_count),
      author: user,
      body: text.to_string(),
      created_at: now,
      updated_at: now,
    })
  }

  fn format_comment_url(&self, pr_url: &str, comment_id: &str) -> String {
    format!("{pr_url}#issuecomment-{comment_id}")
  }

  fn format_merge_request_url(&self, owner: &str, repo: &str, number: u64) -> String {
    format!("https://github.com/{owner}/{repo}/pull/{number}")
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[tokio::test]
  async fn test_mock_github_creation() {
    let mock = MockGitHub::new();
    assert_eq!(mock.api_call_count, 0);
    assert!(!mock.should_fail);
  }

  #[tokio::test]
  async fn test_mock_github_with_test_data() {
    let mock = MockGitHub::with_test_data();
    assert_eq!(mock.repositories.len(), 1);
    assert_eq!(mock.users.len(), 1);
    assert_eq!(mock.merge_requests.len(), 1);
    assert_eq!(mock.discussions.len(), 1);
  }

  #[tokio::test]
  async fn test_get_repository_success() {
    let mock = MockGitHub::with_test_data();
    let result = mock.get_repository("test_owner", "test_repo").await;
    assert!(result.is_ok());
  }

  #[tokio::test]
  async fn test_get_repository_not_found() {
    let mock = MockGitHub::new();
    let result = mock.get_repository("nonexistent", "repo").await;
    assert!(result.is_err());
  }

  #[tokio::test]
  async fn test_get_repository_failure() {
    let mut mock = MockGitHub::with_test_data();
    mock.set_should_fail(true);
    let result = mock.get_repository("test_owner", "test_repo").await;
    assert!(result.is_err());
  }

  #[tokio::test]
  async fn test_get_merge_request_success() {
    let mock = MockGitHub::with_test_data();

    let result = mock.get_merge_request("test_owner", "test_repo", 123).await;
    assert!(result.is_ok());

    let mr = result.unwrap();
    assert_eq!(mr.number, 123);
    assert_eq!(mr.title, "Test Pull Request");
  }

  #[tokio::test]
  async fn test_get_discussions_success() {
    let mock = MockGitHub::with_test_data();

    let result = mock.get_discussions("test_owner", "test_repo", 123).await;
    assert!(result.is_ok());

    let discussions = result.unwrap();
    assert_eq!(discussions.len(), 1);
    assert_eq!(discussions[0].notes.len(), 1);
    assert_eq!(discussions[0].notes[0].body, "This needs improvement");
  }

  #[tokio::test]
  async fn test_add_comment_success() {
    let mock = MockGitHub::with_test_data();

    let result = mock.add_comment("test_owner", "test_repo", "disc123", "Test comment").await;
    assert!(result.is_ok());

    let note = result.unwrap();
    assert_eq!(note.body, "Test comment");
    assert!(note.id.starts_with("comment_"));
  }

  #[tokio::test]
  async fn test_resolve_discussion_success() {
    let mock = MockGitHub::with_test_data();

    let result = mock.resolve_discussion("test_owner", "test_repo", "disc123").await;
    assert!(result.is_ok());
    assert!(result.unwrap());
  }

  #[tokio::test]
  async fn test_add_reaction_success() {
    let mock = MockGitHub::with_test_data();

    let result = mock.add_reaction("test_owner", "test_repo", "disc123", ReactionType::Heart).await;
    assert!(result.is_ok());
    assert!(result.unwrap());
  }

  #[tokio::test]
  async fn test_get_diffs_returns_empty() {
    let mock = MockGitHub::with_test_data();

    let result = mock.get_diffs("test_owner", "test_repo", 123).await;
    assert!(result.is_ok());

    let diffs = result.unwrap();
    assert_eq!(diffs.len(), 0);
  }

  #[tokio::test]
  async fn test_get_pipelines_returns_empty() {
    let mock = MockGitHub::with_test_data();

    let result = mock.get_pipelines("test_owner", "test_repo", "sha123").await;
    assert!(result.is_ok());

    let pipelines = result.unwrap();
    assert_eq!(pipelines.len(), 0);
  }
}
